//! PostgreSQL-backed files, staged uploads, entity documents, and reports.
//!
//! Blob bytes live in `core.file_object.content` (bytea) — sufficient for the
//! single-node deployment and keeps the stack dependency-free. Staged uploads
//! persist per-chunk in `core.upload_chunk` so they survive restarts.

use crate::authn::AuthContext;
use crate::entity_pg::{db_err, http_err, rls_tx, to_status, ENTITY_SELECT};
use crate::workspace_routes::{meta, ApiEnvelope, CapabilityGuard};
use crate::AppState;
use blake2::Blake2b512;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use rocket::data::Data;
use rocket::data::ToByteUnit;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, routes, State};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgPool, PgRow};
use sqlx::{Postgres, Row, Transaction};
use std::sync::Arc;

fn pg(state: &State<AppState>) -> Result<Arc<AsyncStdDbExecutor>, Status> {
    state.db_executor.clone().ok_or(Status::ServiceUnavailable)
}

async fn tx<'e>(pool: &'e PgPool, ctx: &AuthContext) -> Result<Transaction<'e, Postgres>, DbError> {
    rls_tx(pool, ctx, "files").await
}

fn canonical_hash_algorithm(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().replace('-', "").as_str() {
        "sha256" => Some("sha256"),
        "blake2b" | "blake2b512" => Some("blake2b"),
        _ => None,
    }
}

fn hash_bytes(algorithm: &str, bytes: &[u8]) -> Option<String> {
    match canonical_hash_algorithm(algorithm)? {
        "sha256" => Some(hex::encode(Sha256::digest(bytes))),
        "blake2b" => Some(hex::encode(Blake2b512::digest(bytes))),
        _ => None,
    }
}

fn file_json(r: &PgRow) -> Value {
    let created: Option<chrono::DateTime<chrono::Utc>> = r.try_get("created_at").ok().flatten();
    json!({
        "object_id": r.try_get::<uuid::Uuid,_>("file_id").map(|u|u.to_string()).unwrap_or_default(),
        "filename": r.try_get::<String,_>("name").unwrap_or_default(),
        "content_type": r.try_get::<String,_>("content_type").unwrap_or_default(),
        "size": r.try_get::<i64,_>("size").unwrap_or_default(),
        "content_hash": r.try_get::<Option<String>,_>("sha256").ok().flatten().unwrap_or_default(),
        "created_at": created.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "classification": "internal",
    })
}

// ---------------------------------------------------------------------------
// Staged uploads
// ---------------------------------------------------------------------------

fn upload_json(r: &PgRow) -> Value {
    let total: i64 = r.try_get("total_chunks").unwrap_or(0) as i64;
    let received: i64 = r.try_get("received").ok().flatten().unwrap_or(0);
    json!({
        "upload_id": r.try_get::<uuid::Uuid,_>("upload_id").map(|u|u.to_string()).unwrap_or_default(),
        "status": r.try_get::<String,_>("status").unwrap_or_default(),
        "progress": if total==0 {0.0} else {received as f64/total as f64},
        "received_chunks": received,
        "total_chunks": total,
        "object_id": r.try_get::<Option<uuid::Uuid>,_>("object_id").ok().flatten().map(|u|u.to_string()),
    })
}

const UPLOAD_SELECT: &str = "SELECT u.*,
        (SELECT count(*) FROM core.upload_chunk ch
         WHERE ch.tenant_id=u.tenant_id AND ch.upload_id=u.upload_id) AS received
     FROM core.upload_session u WHERE u.tenant_id=$1";

#[derive(Debug, Deserialize)]
pub struct UploadInitInput {
    pub filename: String,
    pub content_type: String,
    pub expected_size: u64,
    pub expected_hash: String,
    pub hash_algorithm: String,
    pub chunk_size: u64,
}

#[post("/files/upload/init", data = "<body>")]
pub async fn upload_init(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: Json<UploadInitInput>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let mut body = body.into_inner();
    let hash_algorithm = canonical_hash_algorithm(&body.hash_algorithm)
        .ok_or(Status::UnprocessableEntity)?
        .to_string();
    body.expected_hash.make_ascii_lowercase();
    let expected_hash_len = if hash_algorithm == "sha256" { 64 } else { 128 };
    if body.filename.trim().is_empty()
        || body.expected_size == 0
        || body.expected_size > 100 * 1024 * 1024
        || body.chunk_size == 0
        || body.expected_hash.trim().is_empty()
        || body.expected_hash.len() != expected_hash_len
        || !body
            .expected_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(Status::UnprocessableEntity);
    }
    let total_chunks = body.expected_size.div_ceil(body.chunk_size) as i32;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let uid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.upload_session
                 (upload_id, tenant_id, filename, content_type, expected_size,
                  expected_hash, hash_algorithm, chunk_size, total_chunks, status)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'initiated')",
            )
            .bind(uid)
            .bind(ctx.tenant_id)
            .bind(&body.filename)
            .bind(&body.content_type)
            .bind(body.expected_size as i64)
            .bind(&body.expected_hash)
            .bind(&hash_algorithm)
            .bind(body.chunk_size as i64)
            .bind(total_chunks)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query(&format!("{UPLOAD_SELECT} AND u.upload_id=$2"))
                .bind(ctx.tenant_id)
                .bind(uid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(upload_json(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[put("/files/upload/<upload_id>/chunks/<index>", data = "<data>")]
pub async fn upload_chunk(
    cap: CapabilityGuard,
    state: &State<AppState>,
    upload_id: String,
    index: usize,
    data: Data<'_>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let bytes = data
        .open(25.mebibytes())
        .into_bytes()
        .await
        .map_err(|_| Status::BadRequest)?;
    if !bytes.is_complete() {
        return Err(Status::PayloadTooLarge);
    }
    let bytes = bytes.into_inner();
    let uid: uuid::Uuid = upload_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(30), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let session = sqlx::query(&format!(
                "{UPLOAD_SELECT} AND u.upload_id=$2 FOR UPDATE OF u"
            ))
            .bind(ctx.tenant_id)
            .bind(uid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?
            .ok_or_else(|| http_err(404, "upload not found"))?;
            let status: String = session.try_get("status").unwrap_or_default();
            let total: i32 = session.try_get("total_chunks").unwrap_or_default();
            let chunk_size: i64 = session.try_get("chunk_size").unwrap_or_default();
            let expected_size: i64 = session.try_get("expected_size").unwrap_or_default();
            if matches!(status.as_str(), "completed" | "aborted") || index as i32 >= total {
                return Err(http_err(409, "upload is complete or index out of range"));
            }
            let expected_chunk_size = if index as i32 + 1 == total {
                (expected_size - chunk_size * index as i64) as usize
            } else {
                chunk_size as usize
            };
            if bytes.len() != expected_chunk_size {
                return Err(http_err(422, "chunk size mismatch"));
            }
            // Idempotent re-upload: same bytes → OK, different bytes → conflict.
            let existing: Option<(Vec<u8>,)> = sqlx::query_as(
                "SELECT data FROM core.upload_chunk
                 WHERE tenant_id=$1 AND upload_id=$2 AND chunk_index=$3",
            )
            .bind(ctx.tenant_id)
            .bind(uid)
            .bind(index as i32)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            if let Some((existing,)) = existing {
                if existing != bytes {
                    return Err(http_err(
                        409,
                        "chunk already uploaded with different content",
                    ));
                }
            } else {
                sqlx::query(
                    "INSERT INTO core.upload_chunk (tenant_id, upload_id, chunk_index, data)
                     VALUES ($1,$2,$3,$4)",
                )
                .bind(ctx.tenant_id)
                .bind(uid)
                .bind(index as i32)
                .bind(&bytes)
                .execute(&mut *t)
                .await
                .map_err(db_err)?;
            }
            let received: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM core.upload_chunk WHERE tenant_id=$1 AND upload_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(uid)
            .fetch_one(&mut *t)
            .await
            .map_err(db_err)?;
            let new_status = if received as i32 == total {
                "all_chunks_received"
            } else {
                "uploading"
            };
            sqlx::query(
                "UPDATE core.upload_session SET status=$3 WHERE tenant_id=$1 AND upload_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(uid)
            .bind(new_status)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query(&format!("{UPLOAD_SELECT} AND u.upload_id=$2"))
                .bind(ctx.tenant_id)
                .bind(uid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(upload_json(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UploadCompleteInput {
    pub final_hash: String,
    pub total_size: u64,
}

#[post("/files/upload/<upload_id>/complete", data = "<body>")]
pub async fn upload_complete(
    cap: CapabilityGuard,
    state: &State<AppState>,
    upload_id: String,
    body: Json<UploadCompleteInput>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let body = body.into_inner();
    let uid: uuid::Uuid = upload_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(30), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let session = sqlx::query(&format!(
                "{UPLOAD_SELECT} AND u.upload_id=$2 FOR UPDATE OF u"
            ))
            .bind(ctx.tenant_id)
            .bind(uid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?
            .ok_or_else(|| http_err(404, "upload not found"))?;
            let status: String = session.try_get("status").unwrap_or_default();
            if status == "completed" {
                return Ok(upload_json(&session));
            }
            let total: i32 = session.try_get("total_chunks").unwrap_or_default();
            let received: i64 = session.try_get("received").ok().flatten().unwrap_or(0);
            let expected_size: i64 = session.try_get("expected_size").unwrap_or_default();
            let expected_hash: String = session.try_get("expected_hash").unwrap_or_default();
            let algorithm: String = session.try_get("hash_algorithm").unwrap_or_default();
            let filename: String = session.try_get("filename").unwrap_or_default();
            let content_type: String = session.try_get("content_type").unwrap_or_default();
            if status == "aborted"
                || received as i32 != total
                || body.total_size as i64 != expected_size
                || !body.final_hash.eq_ignore_ascii_case(&expected_hash)
            {
                return Err(http_err(409, "upload incomplete or hash mismatch"));
            }
            let chunks: Vec<(i32, Vec<u8>)> = sqlx::query_as(
                "SELECT chunk_index, data FROM core.upload_chunk
                 WHERE tenant_id=$1 AND upload_id=$2 ORDER BY chunk_index",
            )
            .bind(ctx.tenant_id)
            .bind(uid)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            let mut bytes = Vec::with_capacity(expected_size as usize);
            for (i, chunk) in chunks.iter().enumerate() {
                if chunk.0 as usize != i {
                    return Err(http_err(409, "chunk sequence gap"));
                }
                bytes.extend_from_slice(&chunk.1);
            }
            if bytes.len() as i64 != expected_size {
                return Err(http_err(409, "assembled size mismatch"));
            }
            let actual = hash_bytes(&algorithm, &bytes).ok_or_else(|| http_err(500, "hash"))?;
            if actual != expected_hash {
                return Err(http_err(409, "content hash mismatch"));
            }
            let file_id = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.file_object
                 (file_id, tenant_id, name, content_type, size, sha256, content, status, created_by)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'ready',$8)",
            )
            .bind(file_id)
            .bind(ctx.tenant_id)
            .bind(&filename)
            .bind(&content_type)
            .bind(expected_size)
            .bind(format!("{algorithm}:{expected_hash}"))
            .bind(&bytes)
            .bind(ctx.principal_id)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            sqlx::query(
                "UPDATE core.upload_session SET status='completed', object_id=$3
                 WHERE tenant_id=$1 AND upload_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(uid)
            .bind(file_id)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            sqlx::query("DELETE FROM core.upload_chunk WHERE tenant_id=$1 AND upload_id=$2")
                .bind(ctx.tenant_id)
                .bind(uid)
                .execute(&mut *t)
                .await
                .map_err(db_err)?;
            let r = sqlx::query(&format!("{UPLOAD_SELECT} AND u.upload_id=$2"))
                .bind(ctx.tenant_id)
                .bind(uid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(upload_json(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/files/upload/<upload_id>/abort", data = "<_body>")]
pub async fn upload_abort(
    cap: CapabilityGuard,
    state: &State<AppState>,
    upload_id: String,
    _body: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let uid: uuid::Uuid = upload_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let session = sqlx::query(&format!("{UPLOAD_SELECT} AND u.upload_id=$2 FOR UPDATE OF u"))
                .bind(ctx.tenant_id)
                .bind(uid)
                .fetch_optional(&mut *t)
                .await
                .map_err(db_err)?
                .ok_or_else(|| http_err(404, "upload not found"))?;
            if session.try_get::<String, _>("status").unwrap_or_default() == "completed" {
                return Err(http_err(409, "upload already completed"));
            }
            sqlx::query("UPDATE core.upload_session SET status='aborted' WHERE tenant_id=$1 AND upload_id=$2")
                .bind(ctx.tenant_id)
                .bind(uid)
                .execute(&mut *t)
                .await
                .map_err(db_err)?;
            sqlx::query("DELETE FROM core.upload_chunk WHERE tenant_id=$1 AND upload_id=$2")
                .bind(ctx.tenant_id)
                .bind(uid)
                .execute(&mut *t)
                .await
                .map_err(db_err)?;
            let r = sqlx::query(&format!("{UPLOAD_SELECT} AND u.upload_id=$2"))
                .bind(ctx.tenant_id)
                .bind(uid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(upload_json(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/files/upload/<upload_id>")]
pub async fn get_upload_status(
    cap: CapabilityGuard,
    state: &State<AppState>,
    upload_id: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let uid: uuid::Uuid = upload_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query(&format!("{UPLOAD_SELECT} AND u.upload_id=$2"))
                .bind(ctx.tenant_id)
                .bind(uid)
                .fetch_optional(&mut *t)
                .await
                .map_err(db_err)?
                .ok_or_else(|| http_err(404, "upload not found"))?;
            Ok(upload_json(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

#[get("/files/<object_id>")]
pub async fn get_file(
    cap: CapabilityGuard,
    state: &State<AppState>,
    object_id: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let fid: uuid::Uuid = object_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query("SELECT * FROM core.file_object WHERE tenant_id=$1 AND file_id=$2")
                .bind(ctx.tenant_id)
                .bind(fid)
                .fetch_optional(&mut *t)
                .await
                .map_err(db_err)?
                .ok_or_else(|| http_err(404, "file not found"))?;
            Ok(file_json(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

async fn fetch_blob(
    exec: Arc<AsyncStdDbExecutor>,
    ctx: AuthContext,
    file_id: uuid::Uuid,
) -> Result<Value, Status> {
    exec.run(Deadline::in_secs(30), move |pool| async move {
        let mut t = tx(&pool, &ctx).await?;
        let row: Option<(String, Option<Vec<u8>>)> = sqlx::query_as(
            "SELECT content_type, content FROM core.file_object
             WHERE tenant_id=$1 AND file_id=$2",
        )
        .bind(ctx.tenant_id)
        .bind(file_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db_err)?;
        let (ct, content) = row.ok_or_else(|| http_err(404, "file not found"))?;
        let bytes = content.ok_or_else(|| http_err(404, "file content missing"))?;
        Ok(json!({ "content_type": ct, "bytes_hex": hex::encode(bytes) }))
    })
    .await
    .map_err(|e| to_status(&e))
}

fn blob_response(v: &Value) -> Result<(ContentType, Arc<[u8]>), Status> {
    let hex_str = v["bytes_hex"].as_str().unwrap_or_default();
    let bytes = hex::decode(hex_str).map_err(|_| Status::InternalServerError)?;
    let ct = ContentType::parse_flexible(v["content_type"].as_str().unwrap_or_default())
        .unwrap_or(ContentType::Binary);
    Ok((ct, Arc::from(bytes)))
}

#[get("/files/<object_id>/download", rank = 2)]
pub async fn download_file(
    cap: CapabilityGuard,
    state: &State<AppState>,
    object_id: String,
) -> Result<(ContentType, Arc<[u8]>), Status> {
    let fid: uuid::Uuid = object_id.parse().map_err(|_| Status::NotFound)?;
    let v = fetch_blob(pg(state)?, cap.ctx.clone(), fid).await?;
    blob_response(&v)
}

#[delete("/files/<object_id>")]
pub async fn delete_file(
    cap: CapabilityGuard,
    state: &State<AppState>,
    object_id: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let fid: uuid::Uuid = object_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    exec.run(Deadline::in_secs(10), move |pool| async move {
        let mut t = tx(&pool, &ctx).await?;
        let changed = sqlx::query("DELETE FROM core.file_object WHERE tenant_id=$1 AND file_id=$2")
            .bind(ctx.tenant_id)
            .bind(fid)
            .execute(&mut *t)
            .await
            .map_err(db_err)?
            .rows_affected();
        if changed == 0 {
            return Err(http_err(404, "file not found"));
        }
        t.commit().await.map_err(db_err)?;
        Ok(Value::Null)
    })
    .await
    .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: json!({ "deleted": true }),
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Entity documents
// ---------------------------------------------------------------------------

fn document_json(r: &PgRow) -> Value {
    let uploaded: Option<chrono::DateTime<chrono::Utc>> = r.try_get("created_at").ok().flatten();
    let qualifiers: Value = r.try_get("qualifiers").unwrap_or(json!({}));
    let quals: Vec<Value> = qualifiers
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| json!({"label": k, "value": v.as_str().unwrap_or_default()}))
                .collect()
        })
        .unwrap_or_default();
    json!({
        "id": r.try_get::<uuid::Uuid,_>("doc_id").map(|u|u.to_string()).unwrap_or_default(),
        "title": r.try_get::<String,_>("title").unwrap_or_default(),
        "kind": r.try_get::<String,_>("kind").unwrap_or_default(),
        "file_name": r.try_get::<Option<String>,_>("file_name").ok().flatten().unwrap_or_default(),
        "size_bytes": r.try_get::<Option<i64>,_>("size").ok().flatten().unwrap_or_default(),
        "content_type": r.try_get::<Option<String>,_>("content_type").ok().flatten().unwrap_or_default(),
        "uploaded_at": uploaded.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "qualifiers": quals,
        "valid_from": Value::Null,
        "valid_to": Value::Null,
        "source": "manual",
    })
}

const DOC_SELECT: &str = "SELECT d.*, f.name AS file_name, f.size, f.content_type
     FROM core.entity_document d
     LEFT JOIN core.file_object f ON f.file_id=d.file_id
     WHERE d.tenant_id=$1";

#[get("/entities/<public_id>/documents")]
pub async fn get_entity_documents(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let pid: uuid::Uuid = public_id
                .parse()
                .map_err(|_| http_err(404, "entity not found"))?;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM core.entity
                               WHERE tenant_id=$1 AND public_id=$2)",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_one(&mut *t)
            .await
            .map_err(db_err)?;
            if !exists {
                return Err(http_err(404, "entity not found"));
            }
            let rows = sqlx::query(&format!(
                "{DOC_SELECT} AND d.entity_id=(SELECT entity_id FROM core.entity WHERE tenant_id=$1 AND public_id=$2)
                 ORDER BY d.created_at DESC",
            ))
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            Ok(Value::Array(rows.iter().map(document_json).collect()))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct DocumentUploadInput {
    pub title: String,
    pub kind: String,
    pub file_name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[post("/entities/<public_id>/documents", format = "json", data = "<body>")]
pub async fn upload_entity_document(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: Json<DocumentUploadInput>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let body = body.into_inner();
    if body.kind.trim().is_empty()
        || body.file_name.trim().is_empty()
        || body.bytes.is_empty()
        || body.bytes.len() > 20 * 1024 * 1024
    {
        return Err(Status::UnprocessableEntity);
    }
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(30), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let pid: uuid::Uuid = public_id
                .parse()
                .map_err(|_| http_err(404, "entity not found"))?;
            let eid: Option<i64> = sqlx::query_scalar(
                "SELECT entity_id FROM core.entity
                 WHERE tenant_id=$1 AND public_id=$2 AND retired_at IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            let eid = eid.ok_or_else(|| http_err(404, "entity not found"))?;
            let file_id = uuid::Uuid::now_v7();
            let doc_id = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.file_object
                 (file_id, tenant_id, name, content_type, size, sha256, content, status, created_by)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'ready',$8)",
            )
            .bind(file_id)
            .bind(ctx.tenant_id)
            .bind(&body.file_name)
            .bind(if body.content_type.trim().is_empty() {
                "application/octet-stream"
            } else {
                body.content_type.trim()
            })
            .bind(body.bytes.len() as i64)
            .bind(format!(
                "sha256:{}",
                hex::encode(Sha256::digest(&body.bytes))
            ))
            .bind(&body.bytes)
            .bind(ctx.principal_id)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let title = if body.title.trim().is_empty() {
                body.file_name.clone()
            } else {
                body.title.trim().to_string()
            };
            sqlx::query(
                "INSERT INTO core.entity_document
                 (doc_id, tenant_id, entity_id, file_id, kind, title)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(doc_id)
            .bind(ctx.tenant_id)
            .bind(eid)
            .bind(file_id)
            .bind(&body.kind)
            .bind(&title)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query(&format!("{DOC_SELECT} AND d.doc_id=$2"))
                .bind(ctx.tenant_id)
                .bind(doc_id)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(document_json(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/entities/<public_id>/documents/<document_id>/download")]
pub async fn download_entity_document(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    document_id: String,
) -> Result<(ContentType, Arc<[u8]>), Status> {
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let did: uuid::Uuid = document_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(30), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let row: Option<(uuid::Uuid,)> = sqlx::query_as(
                "SELECT d.file_id FROM core.entity_document d
                 JOIN core.entity e ON e.tenant_id=d.tenant_id AND e.entity_id=d.entity_id
                 WHERE d.tenant_id=$1 AND e.public_id=$2 AND d.doc_id=$3",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .bind(did)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            let (file_id,) = row.ok_or_else(|| http_err(404, "document not found"))?;
            let row2: Option<(String, Option<Vec<u8>>)> = sqlx::query_as(
                "SELECT content_type, content FROM core.file_object WHERE file_id=$1",
            )
            .bind(file_id)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            let (ct, content) = row2.ok_or_else(|| http_err(404, "file not found"))?;
            Ok(json!({
                "content_type": ct,
                "bytes_hex": hex::encode(content.ok_or_else(|| http_err(404, "content missing"))?),
            }))
        })
        .await
        .map_err(|e| to_status(&e))?;
    blob_response(&v)
}

#[delete("/entities/<public_id>/documents/<document_id>")]
pub async fn delete_entity_document(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    document_id: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let did: uuid::Uuid = document_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    exec.run(Deadline::in_secs(10), move |pool| async move {
        let mut t = tx(&pool, &ctx).await?;
        let changed = sqlx::query(
            "DELETE FROM core.entity_document d USING core.entity e
             WHERE d.tenant_id=$1 AND e.tenant_id=d.tenant_id
               AND d.entity_id=e.entity_id AND e.public_id=$2 AND d.doc_id=$3",
        )
        .bind(ctx.tenant_id)
        .bind(pid)
        .bind(did)
        .execute(&mut *t)
        .await
        .map_err(db_err)?
        .rows_affected();
        if changed == 0 {
            return Err(http_err(404, "document not found"));
        }
        t.commit().await.map_err(db_err)?;
        Ok(Value::Null)
    })
    .await
    .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: json!({ "deleted": true }),
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Reports — renders a real report file from live entity data.
// ---------------------------------------------------------------------------

fn report_row(r: &PgRow) -> Value {
    let params: Value = r.try_get("params").unwrap_or(json!({}));
    let created: Option<chrono::DateTime<chrono::Utc>> = r.try_get("created_at").ok().flatten();
    let completed: Option<chrono::DateTime<chrono::Utc>> = r.try_get("completed_at").ok().flatten();
    let kind: String = r.try_get("kind").unwrap_or_default();
    let rid: String = r
        .try_get::<uuid::Uuid, _>("report_id")
        .map(|u| u.to_string())
        .unwrap_or_default();
    let size: Option<i64> = r.try_get("size").ok().flatten();
    let login: Option<String> = r.try_get("creator_login").ok().flatten();
    json!({
        "report_id": rid.clone(),
        "entity_public_id": params.get("entity_id").and_then(Value::as_str).unwrap_or_default(),
        "template_id": kind,
        "title": kind.replace('_', " "),
        "format": r.try_get::<String,_>("format").unwrap_or_default(),
        "status": r.try_get::<String,_>("status").unwrap_or_default(),
        "generated_at": completed.or(created).map(|t|t.to_rfc3339()),
        "download_url": r.try_get::<Option<uuid::Uuid>,_>("file_id").ok().flatten()
            .map(|f| format!("/api/v1/files/{f}/download")),
        "size_bytes": size,
        "expires_at": Value::Null,
        "generated_by": login.unwrap_or_default(),
    })
}

const REPORT_SELECT: &str = "SELECT r.*, f.size, c.login AS creator_login
     FROM core.report_instance r
     LEFT JOIN core.file_object f ON f.file_id=r.file_id
     LEFT JOIN sec.principal_credential c ON c.principal_id=r.created_by
     WHERE r.tenant_id=$1";

#[get("/reports")]
pub async fn list_reports(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let rows = sqlx::query(&format!(
                "{REPORT_SELECT} ORDER BY r.created_at DESC LIMIT 200",
            ))
            .bind(ctx.tenant_id)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            Ok(Value::Array(rows.iter().map(report_row).collect()))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/reports", data = "<body>")]
pub async fn create_report(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| Status::BadRequest)?;
    let entity_public_id = parsed
        .get("entity_id")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or(Status::UnprocessableEntity)?
        .to_string();
    let template_id = parsed
        .get("template")
        .and_then(Value::as_str)
        .unwrap_or("individual_check")
        .to_string();
    let format = parsed
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("csv")
        .to_string();
    if !matches!(format.as_str(), "pdf" | "xlsx" | "docx" | "json" | "csv") {
        return Err(Status::UnprocessableEntity);
    }
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(20), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let pid: uuid::Uuid = entity_public_id
                .parse()
                .map_err(|_| http_err(404, "entity not found"))?;
            let row: Option<_> = sqlx::query_as::<
                _,
                (
                    i64,
                    uuid::Uuid,
                    uuid::Uuid,
                    String,
                    i16,
                    i64,
                    chrono::DateTime<chrono::Utc>,
                    Option<chrono::DateTime<chrono::Utc>>,
                ),
            >(&format!("{ENTITY_SELECT} AND e.public_id = $2"))
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            let er = crate::entity_pg::row_to_entity_row(
                row.ok_or_else(|| http_err(404, "entity not found"))?,
            );
            let perm = crate::entity_pg::perm_for(&mut t, &ctx).await?;
            let records = crate::entity_pg::record_for(&mut t, ctx.tenant_id, &[er], &perm).await?;
            let rec = records
                .into_iter()
                .next()
                .ok_or_else(|| http_err(404, "entity not found"))?;

            // Render report content from live data.
            let (bytes, content_type) = match format.as_str() {
                "json" => (
                    serde_json::to_vec_pretty(&json!({
                        "report": template_id,
                        "entity": rec,
                        "generated_at": chrono::Utc::now().to_rfc3339(),
                    }))
                    .map_err(|e| db_err(e))?,
                    "application/json",
                ),
                _ => {
                    let mut csv = String::from("field,value\n");
                    if let Some(fields) = rec.fields.as_ref().and_then(Value::as_object) {
                        for (k, v) in fields {
                            let text = v
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| v.to_string());
                            csv.push_str(&format!("{k},\"{}\"\n", text.replace('"', "\"\"")));
                        }
                    }
                    (csv.into_bytes(), "text/csv")
                }
            };
            let file_id = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.file_object
                 (file_id, tenant_id, name, content_type, size, sha256, content, status, created_by)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'ready',$8)",
            )
            .bind(file_id)
            .bind(ctx.tenant_id)
            .bind(format!("{template_id}.{format}"))
            .bind(content_type)
            .bind(bytes.len() as i64)
            .bind(format!("sha256:{}", hex::encode(Sha256::digest(&bytes))))
            .bind(&bytes)
            .bind(ctx.principal_id)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let rid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.report_instance
                 (report_id, tenant_id, kind, format, status, params, file_id,
                  created_by, completed_at)
                 VALUES ($1,$2,$3,$4,'ready',$5,$6,$7,now())",
            )
            .bind(rid)
            .bind(ctx.tenant_id)
            .bind(&template_id)
            .bind(&format)
            .bind(json!({ "entity_id": entity_public_id }))
            .bind(file_id)
            .bind(ctx.principal_id)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query(&format!("{REPORT_SELECT} AND r.report_id=$2"))
                .bind(ctx.tenant_id)
                .bind(rid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(report_row(&r))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        upload_init,
        upload_chunk,
        upload_complete,
        upload_abort,
        get_upload_status,
        get_file,
        download_file,
        delete_file,
        get_entity_documents,
        upload_entity_document,
        download_entity_document,
        delete_entity_document,
        list_reports,
        create_report,
    ]
}
