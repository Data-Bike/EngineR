//! Database dump administration (export side of `apps/dump`).
//!
//! `POST /admin/dump` spawns `enginer-dump export` into
//! `ENGINER_DUMP_DIR/<dump_id>/`, tracked by a `status.json` marker.
//! `GET /admin/dump` lists runs; `GET /admin/dump/<id>/<file>` streams a
//! part file; `DELETE /admin/dump/<id>` removes a finished run.
//!
//! The binary connects with `ENGINER_DUMP_DATABASE_URL` — a superuser /
//! BYPASSRLS DSN (dumps span every tenant and contain credential hashes).
//! That DSN is never exposed through the API. Gated by `security.manage`.
//! Restore stays CLI-only: replaying a dump can overwrite production data.

use crate::workspace_routes::{meta, ApiEnvelope, CapabilityGuard};
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

const PART_FILES: &[&str] = &[
    "01_schema.ndjson",
    "02_data.ndjson",
    "03_system.ndjson",
    "manifest.json",
    "export.log",
    "status.json",
];

fn dump_root() -> PathBuf {
    std::env::var("ENGINER_DUMP_DIR").map_or_else(|_| PathBuf::from("./dumps"), PathBuf::from)
}

fn dump_bin() -> String {
    std::env::var("ENGINER_DUMP_BIN").unwrap_or_else(|_| "enginer-dump".into())
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

fn any_running() -> bool {
    std::fs::read_dir(dump_root()).is_ok_and(|rd| {
        rd.flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .any(|p| status_of(&p)["status"] == "running")
    })
}

#[post("/admin/dump")]
pub fn run_dump(cap: CapabilityGuard) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let _ = &cap.ctx;
    let dsn = std::env::var("ENGINER_DUMP_DATABASE_URL").map_err(|_| {
        tracing::warn!("dump requested but ENGINER_DUMP_DATABASE_URL is unset");
        Status::ServiceUnavailable
    })?;
    if any_running() {
        return Err(Status::Conflict);
    }

    let dump_id = chrono::Utc::now().format("dump-%Y%m%dT%H%M%SZ").to_string();
    let dir = dump_root().join(&dump_id);
    std::fs::create_dir_all(&dir).map_err(|_| Status::InternalServerError)?;
    let log_path = dir.join("export.log");
    let log_file = std::fs::File::create(&log_path).map_err(|_| Status::InternalServerError)?;
    let log_err = log_file
        .try_clone()
        .map_err(|_| Status::InternalServerError)?;

    let mut child = match std::process::Command::new(dump_bin())
        .arg("export")
        .arg("--dsn")
        .arg(&dsn)
        .arg("--dir")
        .arg(&dir)
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(log_err))
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("cannot spawn enginer-dump: {e}");
            let _ = std::fs::remove_dir_all(&dir);
            return Err(Status::ServiceUnavailable);
        }
    };

    let status_path = dir.join("status.json");
    std::thread::spawn(move || {
        let result = match child.wait() {
            Ok(s) => json!({
                "status": if s.success() { "done" } else { "failed" },
                "exit_code": s.code().unwrap_or(-1),
                "finished_at": chrono::Utc::now().to_rfc3339(),
            }),
            Err(e) => json!({
                "status": "failed",
                "exit_code": Value::Null,
                "error": e.to_string(),
                "finished_at": chrono::Utc::now().to_rfc3339(),
            }),
        };
        if let Ok(mut f) = std::fs::File::create(&status_path) {
            let _ = f.write_all(result.to_string().as_bytes());
        }
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
    if status_of(&dir)["status"] == "running" {
        return Err(Status::Conflict);
    }
    std::fs::remove_dir_all(&dir).map_err(|_| Status::InternalServerError)?;
    Ok(Json(ApiEnvelope {
        data: json!({"deleted": true}),
        meta: meta(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![run_dump, list_dumps, download_dump_part, delete_dump]
}
