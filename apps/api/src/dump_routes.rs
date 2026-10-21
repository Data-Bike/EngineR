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
use crate::ApiError;
use rocket::data::{Data, ToByteUnit};
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

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

/// The busy check and the "running"/"restoring" marker write are a
/// single critical section — without it two concurrent requests can
/// both observe "nothing busy" and run export/restore in parallel.
static BUSY_LOCK: Mutex<()> = Mutex::new(());

/// A run marked running/restoring whose status marker (or directory,
/// when the marker was never written — e.g. the process was killed)
/// hasn't been touched for a day is treated as crashed rather than
/// live. Otherwise a killed process would wedge every future dump and
/// refuse deletion forever. Concurrent exports are harmless anyway —
/// each reads a consistent snapshot into its own directory.
const STALE_AFTER: Duration = Duration::from_secs(24 * 3600);

fn valid_dump_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.contains("..")
        // The first char must be alphanumeric: a bare "." resolves to
        // the dump root itself (and DELETE would wipe every dump).
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
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

/// Is this run still live? A running/restoring marker that went stale
/// (marker or dir untouched for `STALE_AFTER`) counts as crashed.
fn is_active(dir: &Path) -> bool {
    if !matches!(
        status_of(dir)["status"].as_str(),
        Some("running" | "restoring")
    ) {
        return false;
    }
    let touched = std::fs::metadata(dir.join("status.json"))
        .or_else(|_| std::fs::metadata(dir))
        .and_then(|m| m.modified())
        .ok();
    !touched.is_some_and(|t| t.elapsed().unwrap_or_default() > STALE_AFTER)
}

/// Called once at process start: any `running`/`restoring` marker that
/// survived a previous process belongs to a dead run — the executor was
/// killed with it. Without the sweep such orphans wedge every future dump
/// and restore behind the busy check for up to `STALE_AFTER`.
/// Single-replica assumption: a live dump owned by another replica would be
/// marked failed, so only run this where one API instance owns the dir.
pub fn sweep_dead_markers() {
    if let Ok(rd) = std::fs::read_dir(dump_root()) {
        for e in rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            if matches!(
                status_of(&e)["status"].as_str(),
                Some("running" | "restoring")
            ) {
                let _ = write_status(
                    &e,
                    &json!({
                        "status": "failed",
                        "exit_code": -1,
                        "finished_at": chrono::Utc::now().to_rfc3339(),
                        "error": "crashed: API process restarted",
                    }),
                );
                tracing::warn!(dir = %e.display(), "dump run marked failed: orphaned by restart");
            }
        }
    }
}

/// Export and restore share the same database — never run concurrently.
fn any_busy() -> bool {
    std::fs::read_dir(dump_root()).is_ok_and(|rd| {
        rd.flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .any(|p| is_active(&p))
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
    PART_FILES
        .iter()
        .filter(|f| f.ends_with(".ndjson") || f.ends_with(".ndjson.xz"))
        .any(|f| dir.join(f).exists())
}

/// First dump line must be a JSON object with a `table` field — cheap
/// sanity gate so an uploaded arbitrary blob can't reach restore.
fn validate_dump_file(path: &Path) -> bool {
    enginer_dump::probe_dump(path)
}

/// Second-precision ids can collide on rapid successive calls —
/// append `-1`, `-2`, … until the name is free.
fn unique_dir(prefix: &str) -> Option<(String, PathBuf)> {
    for i in 0..100 {
        let id = if i == 0 {
            prefix.to_string()
        } else {
            format!("{prefix}-{}", i - 1)
        };
        let dir = dump_root().join(&id);
        if !dir.exists() {
            return Some((id, dir));
        }
    }
    None
}

#[post("/admin/dump")]
pub fn run_dump(cap: CapabilityGuard) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let _ = &cap.ctx;
    let dsn = std::env::var("ENGINER_DUMP_DATABASE_URL").map_err(|_| {
        tracing::warn!("dump requested but ENGINER_DUMP_DATABASE_URL is unset");
        Status::ServiceUnavailable
    })?;
    let _guard = BUSY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if any_busy() {
        return Err(ApiError::from(Status::Conflict));
    }

    let base = chrono::Utc::now().format("dump-%Y%m%dT%H%M%SZ").to_string();
    let Some((dump_id, dir)) = unique_dir(&base) else {
        return Err(ApiError::from(Status::Conflict));
    };
    std::fs::create_dir_all(&dir).map_err(|_| ApiError::from(Status::InternalServerError))?;
    // Mark busy before unlocking — the thread may outlive the request.
    write_status(
        &dir,
        &json!({"status": "running", "started_at": chrono::Utc::now().to_rfc3339()}),
    );
    drop(_guard);
    let log_path = dir.join("export.log");

    // In-process export on a dedicated OS thread: the dump crate uses
    // async-std/sqlx which has its own global reactor, so block_on here
    // works independently of Rocket's runtime.
    std::thread::spawn(move || {
        let result = match catch_unwind(AssertUnwindSafe(|| {
            async_std::task::block_on(enginer_dump::export(&dsn, &dir))
        })) {
            Ok(Ok(())) => json!({
                "status": "done",
                "exit_code": 0,
                "finished_at": chrono::Utc::now().to_rfc3339(),
            }),
            Ok(Err(e)) => {
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
            Err(_) => {
                // A panicking export must not leave the run stuck in
                // "running" forever — mark it failed so future dumps
                // aren't blocked by a zombie marker.
                tracing::error!("dump export panicked");
                json!({
                    "status": "failed",
                    "exit_code": Value::Null,
                    "error": "export thread panicked",
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
pub fn list_dumps(cap: CapabilityGuard) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
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
) -> Result<NamedFile, ApiError> {
    let _ = &cap.ctx;
    if !valid_dump_id(dump_id) || !PART_FILES.contains(&file) {
        return Err(ApiError::from(Status::NotFound));
    }
    NamedFile::open(dump_root().join(dump_id).join(file))
        .await
        .map_err(|_| ApiError::from(Status::NotFound))
}

/// Store an uploaded dump file as `upload-<ts>/dump.ndjson[.xz]`.
/// Streams to disk — never buffers the body in memory.
#[post("/admin/dump/upload", data = "<data>")]
pub async fn upload_dump(
    cap: CapabilityGuard,
    data: Data<'_>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let _ = &cap.ctx;
    let base = chrono::Utc::now()
        .format("upload-%Y%m%dT%H%M%SZ")
        .to_string();
    let Some((dump_id, dir)) = unique_dir(&base) else {
        return Err(ApiError::from(Status::Conflict));
    };
    std::fs::create_dir_all(&dir).map_err(|_| ApiError::from(Status::InternalServerError))?;

    let tmp = dir.join("upload.part");
    {
        // 2 GiB is already far beyond a realistic tenant dump; an
        // unbounded stream would let `security.manage` fill the host's
        // disk via a single upload.
        let mut stream = data.open(2_u64.gibibytes());
        let mut file = rocket::tokio::fs::File::create(&tmp)
            .await
            .map_err(|_| ApiError::from(Status::InternalServerError))?;
        rocket::tokio::io::copy(&mut stream, &mut file)
            .await
            .map_err(|_| ApiError::from(Status::InternalServerError))?;
    }

    // Sniff the container type, then verify the first NDJSON line.
    let mut magic = [0u8; 6];
    let n = std::fs::File::open(&tmp)
        .and_then(|mut f| f.read(&mut magic))
        .map_err(|_| ApiError::from(Status::InternalServerError))?;
    let name = if n == 6 && magic == XZ_MAGIC {
        "dump.ndjson.xz"
    } else {
        "dump.ndjson"
    };
    let target = dir.join(name);
    std::fs::rename(&tmp, &target).map_err(|_| ApiError::from(Status::InternalServerError))?;
    if !validate_dump_file(&target) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(ApiError::from(Status::UnprocessableEntity));
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
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let _ = &cap.ctx;
    if body["confirm"].as_str() != Some("RESTORE") {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let dsn = std::env::var("ENGINER_DUMP_DATABASE_URL").map_err(|_| {
        tracing::warn!("restore requested but ENGINER_DUMP_DATABASE_URL is unset");
        Status::ServiceUnavailable
    })?;
    if !valid_dump_id(dump_id) {
        return Err(ApiError::from(Status::NotFound));
    }
    let dir = dump_root().join(dump_id);
    if !dir.is_dir() || !has_dump_payload(&dir) {
        return Err(ApiError::from(Status::NotFound));
    }
    let _guard = BUSY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if any_busy() {
        return Err(ApiError::from(Status::Conflict));
    }

    write_status(
        &dir,
        &json!({"status": "restoring", "started_at": chrono::Utc::now().to_rfc3339()}),
    );
    drop(_guard);
    let log_path = dir.join("restore.log");
    std::thread::spawn(move || {
        let result = match catch_unwind(AssertUnwindSafe(|| {
            async_std::task::block_on(enginer_dump::import(&dsn, &dir, 500))
        })) {
            Ok(Ok(())) => json!({
                "status": "restored",
                "finished_at": chrono::Utc::now().to_rfc3339(),
            }),
            Ok(Err(e)) => {
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
            Err(_) => {
                tracing::error!("dump restore panicked");
                json!({
                    "status": "failed",
                    "error": "restore thread panicked",
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
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let _ = &cap.ctx;
    if !valid_dump_id(dump_id) {
        return Err(ApiError::from(Status::NotFound));
    }
    let dir = dump_root().join(dump_id);
    if !dir.is_dir() {
        return Err(ApiError::from(Status::NotFound));
    }
    if is_active(&dir) {
        return Err(ApiError::from(Status::Conflict));
    }
    std::fs::remove_dir_all(&dir).map_err(|_| ApiError::from(Status::InternalServerError))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "enginer-dump-routes-test-{}-{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn dump_id_validation() {
        for ok in [
            "dump-20261020T120000Z",
            "upload-1",
            "a",
            "0",
            "x.y_z",
            "dump.tar.gz",
        ] {
            assert!(valid_dump_id(ok), "{ok} should be valid");
        }
        for bad in [
            "",
            ".",  // resolves to the dump root itself
            "..", // traversal
            "../x",
            ".hidden", // dot-prefixed ids are reserved
            "-lead",
            "_lead",
            "a/b",
            "a\\b",
            "a b",
            "x".repeat(65).as_str(),
            "id?",
            "id#",
        ] {
            assert!(!valid_dump_id(bad), "{bad:?} should be invalid");
        }
    }

    #[test]
    fn dot_id_cannot_escape_root() {
        // Regression: dump_id "." used to pass validation — DELETE
        // would then remove the whole dump directory tree.
        assert!(!valid_dump_id("."));
    }

    #[test]
    fn payload_detection() {
        let dir = tmp_dir("payload");
        assert!(!has_dump_payload(&dir));
        std::fs::write(dir.join("status.json"), "{}").unwrap();
        assert!(!has_dump_payload(&dir), "status.json is not a payload");
        std::fs::write(dir.join("export.log"), "log").unwrap();
        assert!(!has_dump_payload(&dir), "logs are not a payload");
        // Legacy parts count too — not just the primary schema part.
        let legacy = tmp_dir("payload-legacy");
        std::fs::write(legacy.join("02_data.ndjson"), "{}\n").unwrap();
        assert!(
            has_dump_payload(&legacy),
            "a lone 02_data part is restorable"
        );
        std::fs::write(dir.join("dump.ndjson"), "{}\n").unwrap();
        assert!(has_dump_payload(&dir));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&legacy);
    }

    #[test]
    fn activity_and_staleness() {
        let dir = tmp_dir("active");
        // No marker at all → "running" by default, but the dir is
        // fresh → active.
        assert!(is_active(&dir));
        write_status(&dir, &json!({"status": "done"}));
        assert!(!is_active(&dir));
        write_status(&dir, &json!({"status": "running"}));
        assert!(is_active(&dir));
        write_status(&dir, &json!({"status": "restoring"}));
        assert!(is_active(&dir));
        write_status(&dir, &json!({"status": "failed"}));
        assert!(!is_active(&dir));

        // A running marker last touched over a day ago is a crashed
        // run, not a live one — it must not wedge future dumps.
        write_status(&dir, &json!({"status": "running"}));
        let old = std::time::SystemTime::now() - STALE_AFTER - Duration::from_secs(60);
        let f = std::fs::File::options()
            .write(true)
            .open(dir.join("status.json"))
            .unwrap();
        f.set_modified(old).unwrap();
        drop(f);
        assert!(!is_active(&dir), "stale marker should not count as busy");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_dir_suffixes() {
        // unique_dir operates on the real dump root; point it at a
        // scratch dir via the env override.
        let root = tmp_dir("uniqueroot");
        std::env::set_var("ENGINER_DUMP_DIR", &root);
        let (id0, _) = unique_dir("dump-x").unwrap();
        assert_eq!(id0, "dump-x");
        std::fs::create_dir_all(root.join("dump-x")).unwrap();
        let (id1, _) = unique_dir("dump-x").unwrap();
        assert_eq!(id1, "dump-x-0");
        std::fs::create_dir_all(root.join("dump-x-0")).unwrap();
        let (id2, _) = unique_dir("dump-x").unwrap();
        assert_eq!(id2, "dump-x-1");
        std::env::remove_var("ENGINER_DUMP_DIR");
        let _ = std::fs::remove_dir_all(&root);
    }
}
