//! Database dump administration (export/upload/restore of `enginer-dump`).
//!
//! `POST /admin/dump` runs `enginer_dump::export` in-process on a
//! dedicated thread into `ENGINER_DUMP_DIR/<dump_id>/dump.ndjson.xz`,
//! tracked by a `status.json` marker. `POST /admin/dump/upload` stores
//! an uploaded dump file as `upload-<ts>/`; `POST /admin/dump/<id>/restore`
//! replays it via `enginer_dump::import` in-process.
//! `GET /admin/dump` lists runs; `GET /admin/dump/<id>/<file>` streams
//! a file; `DELETE /admin/dump/<id>` removes a finished run.
//!
//! Both export and import connect with `ENGINER_DUMP_DATABASE_URL` — a
//! superuser / BYPASSRLS DSN (dumps span every tenant and contain
//! credential hashes). That DSN is never exposed through the API.
//! Gated by `security.manage`. Restore additionally requires a
//! `{"confirm":"RESTORE"}` body — replaying a dump mutates the database.

use crate::workspace_routes::{meta, ApiEnvelope, CapabilityGuard};
use rocket::data::{Data, ToByteUnit};
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const PART_FILES: &[&str] = &[
    // Current single-artifact layout.
    "dump.ndjson.xz",
    "dump.ndjson",
    // Legacy enginer-dump/1 three-part layout — listed so older dumps
    // remain downloadable.
    "01_schema.ndjson.xz",
    "02_data.ndjson.xz",
    "03_system.ndjson.xz",
    "01_schema.ndjson",
    "02_data.ndjson",
    "03_system.ndjson",
    "manifest.json",
    "export.log",
    "restore.log",
    "upload.log",
    "status.json",
];

/// XZ stream magic — sniffed on upload to pick the stored file name.
const XZ_MAGIC: [u8; 6] = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00];

fn dump_root() -> PathBuf {
    std::env::var("ENGINER_DUMP_DIR").map_or_else(|_| PathBuf::from("./dumps"), PathBuf::from)
}

fn valid_dump_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn status_of(dir: &Path) -> Value {
    std::fs::read_to_string(dir.join("status.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({"status": "running"}))
}

fn describe_dump(dir: &Path) -> Value {
    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut files = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if PART_FILES.contains(&name.as_str()) {
                let size = e.metadata().map_or(0, |m| m.len());
                files.push(json!({
                    "name": name,
                    "size": size,
                    "download_url": format!("/api/v1/admin/dump/{id}/{name}"),
                }));
            }
        }
    }
    files.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    let status = status_of(dir);
    json!({
        "dump_id": id,
        "status": status["status"],
        "exit_code": status["exit_code"],
        "finished_at": status["finished_at"],
        "files": files,
    })
}

/// Export and restore share the same database — never run concurrently.
fn any_busy() -> bool {
    std::fs::read_dir(dump_root()).is_ok_and(|rd| {
        rd.flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .any(|p| {
                matches!(
                    status_of(&p)["status"].as_str(),
                    Some("running" | "restoring")
                )
            })
    })
}

fn write_status(dir: &Path, v: &Value) {
    if let Ok(mut f) = std::fs::File::create(dir.join("status.json")) {
        let _ = f.write_all(v.to_string().as_bytes());
    }
}

/// The directory contains something restorable: a single-file dump
/// (either compression) or at least one legacy part.
fn has_dump_payload(dir: &Path) -> bool {
    [
        "dump.ndjson.xz",
        "dump.ndjson",
        "01_schema.ndjson.xz",
        "01_schema.ndjson",
    ]
    .iter()
    .any(|f| dir.join(f).exists())
}

/// First dump line must be a JSON object with a `table` field — cheap
/// sanity gate so an uploaded arbitrary blob can't reach restore.
fn validate_dump_file(path: &Path) -> bool {
    enginer_dump::probe_dump(path)
}

#[post("/admin/dump")]
pub fn run_dump(cap: CapabilityGuard) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let _ = &cap.ctx;
    let dsn = std::env::var("ENGINER_DUMP_DATABASE_URL").map_err(|_| {
        tracing::warn!("dump requested but ENGINER_DUMP_DATABASE_URL is unset");
        Status::ServiceUnavailable
    })?;
    if any_busy() {
        return Err(Status::Conflict);
    }

    let dump_id = chrono::Utc::now().format("dump-%Y%m%dT%H%M%SZ").to_string();
    let dir = dump_root().join(&dump_id);
    std::fs::create_dir_all(&dir).map_err(|_| Status::InternalServerError)?;
    let log_path = dir.join("export.log");

    // In-process export on a dedicated OS thread: the dump crate uses
    // async-std/sqlx which has its own global reactor, so block_on here
    // works independently of Rocket's runtime.
    std::thread::spawn(move || {
        let result = match async_std::task::block_on(enginer_dump::export(&dsn, &dir)) {
            Ok(()) => json!({
                "status": "done",
                "exit_code": 0,
                "finished_at": chrono::Utc::now().to_rfc3339(),
            }),
            Err(e) => {
                tracing::warn!("dump export failed: {e}");
                // Mirror the error into export.log — tracing goes to the
                // API's own log stream, not the dump directory.
                if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&log_path) {
                    let _ = writeln!(f, "export failed: {e}");
                }
                json!({
                    "status": "failed",
                    "exit_code": Value::Null,
                    "error": e.to_string(),
                    "finished_at": chrono::Utc::now().to_rfc3339(),
                })
            }
        };
        write_status(&dir, &result);
    });

    Ok(Json(ApiEnvelope {
        data: json!({"dump_id": dump_id, "status": "running"}),
        meta: meta(),
    }))
}

#[get("/admin/dump")]
pub fn list_dumps(cap: CapabilityGuard) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let _ = &cap.ctx;
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dump_root()) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.push(describe_dump(&p));
            }
        }
    }
    out.sort_by(|a, b| b["dump_id"].as_str().cmp(&a["dump_id"].as_str()));
    Ok(Json(ApiEnvelope {
        data: Value::Array(out),
        meta: meta(),
    }))
}

#[get("/admin/dump/<dump_id>/<file>")]
pub async fn download_dump_part(
    cap: CapabilityGuard,
    dump_id: &str,
    file: &str,
) -> Result<NamedFile, Status> {
    let _ = &cap.ctx;
    if !valid_dump_id(dump_id) || !PART_FILES.contains(&file) {
        return Err(Status::NotFound);
    }
    NamedFile::open(dump_root().join(dump_id).join(file))
        .await
        .map_err(|_| Status::NotFound)
}

/// Store an uploaded dump file as `upload-<ts>/dump.ndjson[.xz]`.
/// Streams to disk — never buffers the body in memory.
#[post("/admin/dump/upload", data = "<data>")]
pub async fn upload_dump(
    cap: CapabilityGuard,
    data: Data<'_>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let _ = &cap.ctx;
    let base = chrono::Utc::now()
        .format("upload-%Y%m%dT%H%M%SZ")
        .to_string();
    // Second-precision ids can collide on rapid successive uploads.
    let mut dump_id = base.clone();
    for i in 0..100 {
        if !dump_root().join(&dump_id).exists() {
            break;
        }
        dump_id = format!("{base}-{i}");
    }
    let dir = dump_root().join(&dump_id);
    std::fs::create_dir_all(&dir).map_err(|_| Status::InternalServerError)?;

    let tmp = dir.join("upload.part");
    {
        let mut stream = data.open(100_u64.gibibytes());
        let mut file = rocket::tokio::fs::File::create(&tmp)
            .await
            .map_err(|_| Status::InternalServerError)?;
        rocket::tokio::io::copy(&mut stream, &mut file)
            .await
            .map_err(|_| Status::InternalServerError)?;
    }

    // Sniff the container type, then verify the first NDJSON line.
    let mut magic = [0u8; 6];
    let n = std::fs::File::open(&tmp)
        .and_then(|mut f| f.read(&mut magic))
        .map_err(|_| Status::InternalServerError)?;
    let name = if n == 6 && magic == XZ_MAGIC {
        "dump.ndjson.xz"
    } else {
        "dump.ndjson"
    };
    let target = dir.join(name);
    std::fs::rename(&tmp, &target).map_err(|_| Status::InternalServerError)?;
    if !validate_dump_file(&target) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(Status::UnprocessableEntity);
    }

    let size = target.metadata().map_or(0, |m| m.len());
    let _ = std::fs::write(
        dir.join("upload.log"),
        format!("uploaded {name} size={size}\n"),
    );
    write_status(
        &dir,
        &json!({
            "status": "done",
            "source": "upload",
            "finished_at": chrono::Utc::now().to_rfc3339(),
        }),
    );
    Ok(Json(ApiEnvelope {
        data: json!({"dump_id": dump_id, "status": "done"}),
        meta: meta(),
    }))
}

/// Replay a stored dump into the database — in-process
/// `enginer_dump::import`. Requires `{"confirm":"RESTORE"}` because it
/// mutates production data (merge semantics: `ON CONFLICT DO NOTHING` +
/// sequence replay).
#[post("/admin/dump/<dump_id>/restore", data = "<body>")]
pub fn restore_dump(
    cap: CapabilityGuard,
    dump_id: &str,
    body: Json<Value>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let _ = &cap.ctx;
    if body["confirm"].as_str() != Some("RESTORE") {
        return Err(Status::UnprocessableEntity);
    }
    let dsn = std::env::var("ENGINER_DUMP_DATABASE_URL").map_err(|_| {
        tracing::warn!("restore requested but ENGINER_DUMP_DATABASE_URL is unset");
        Status::ServiceUnavailable
    })?;
    if !valid_dump_id(dump_id) {
        return Err(Status::NotFound);
    }
    let dir = dump_root().join(dump_id);
    if !dir.is_dir() || !has_dump_payload(&dir) {
        return Err(Status::NotFound);
    }
    if any_busy() {
        return Err(Status::Conflict);
    }

    write_status(&dir, &json!({"status": "restoring"}));
    let log_path = dir.join("restore.log");
    std::thread::spawn(move || {
        let result = match async_std::task::block_on(enginer_dump::import(&dsn, &dir, 500)) {
            Ok(()) => json!({
                "status": "restored",
                "finished_at": chrono::Utc::now().to_rfc3339(),
            }),
            Err(e) => {
                tracing::warn!("dump restore failed: {e}");
                if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&log_path) {
                    let _ = writeln!(f, "restore failed: {e}");
                }
                json!({
                    "status": "failed",
                    "error": e.to_string(),
                    "finished_at": chrono::Utc::now().to_rfc3339(),
                })
            }
        };
        write_status(&dir, &result);
    });

    Ok(Json(ApiEnvelope {
        data: json!({"dump_id": dump_id, "status": "restoring"}),
        meta: meta(),
    }))
}

#[delete("/admin/dump/<dump_id>")]
pub fn delete_dump(
    cap: CapabilityGuard,
    dump_id: &str,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let _ = &cap.ctx;
    if !valid_dump_id(dump_id) {
        return Err(Status::NotFound);
    }
    let dir = dump_root().join(dump_id);
    if !dir.is_dir() {
        return Err(Status::NotFound);
    }
    if matches!(
        status_of(&dir)["status"].as_str(),
        Some("running" | "restoring")
    ) {
        return Err(Status::Conflict);
    }
    std::fs::remove_dir_all(&dir).map_err(|_| Status::InternalServerError)?;
    Ok(Json(ApiEnvelope {
        data: json!({"deleted": true}),
        meta: meta(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        run_dump,
        list_dumps,
        download_dump_part,
        upload_dump,
        restore_dump,
        delete_dump
    ]
}
