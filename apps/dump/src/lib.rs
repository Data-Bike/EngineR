//! `EngineR` database dump/restore — library core.
//!
//! The `enginer-dump` binary is a thin CLI over this; the API calls
//! [`export`]/[`import`] in-process for `POST /api/v1/admin/dump` and
//! `POST /api/v1/admin/dump/<id>/restore`.
//!
//! Streams every application table out to a single xz-compressed NDJSON
//! file (one JSON object per line: `{"table":"<schema>.<name>","row":{...}}`)
//! and can replay it back into a database. Rows are never materialised
//! in memory as a set — export uses a lazy `fetch()` stream, import
//! batches rows through `jsonb_populate_recordset`, so the tool scales
//! to arbitrarily large databases.
//!
//! Single-file layout (`enginer-dump/2`):
//!   `dump.ndjson.xz` — the whole dump: a `_header` line (format,
//!                       `created_at`), then every table's rows grouped
//!                       by table, then `_sequences` lines, then a
//!                       `_manifest` trailer with per-table row counts.
//!   `manifest.json`  — uncompressed sidecar for quick inspection
//!                       (same data as the `_manifest` trailer).
//!   `export.log`     — human-readable export progress (per table).
//!
//! Legacy layouts still restore: `01/02/03_*.ndjson[.xz]` parts from
//! format `enginer-dump/1` are detected when `dump.ndjson[.xz]` is
//! absent.
//!
//! Compression is xz at level `9 | PRESET_EXTREME` (equivalent to
//! `xz -9e`) via `liblzma` — the maintained fork vendoring XZ Utils
//! 5.8.x, i.e. a post-backdoor upstream (the March-2024 backdoor only
//! shipped in the 5.6.0/5.6.1 tarballs).
//!
//! Tables are discovered at runtime from `information_schema`, so new
//! migrations are picked up automatically — keep `SCHEMAS` in sync when
//! adding a new schema.
//!
//! Export runs inside a single `REPEATABLE READ READ ONLY` transaction —
//! the dump is one consistent snapshot even while the source is written.
//!
//! Usage:
//!   enginer-dump export --dsn <url> --dir <`out_dir`>
//!   enginer-dump import --dsn <url> --dir <`dump_dir`> [--batch 500]
//!
//! The tool must connect as a superuser / BYPASSRLS role to see all
//! tenants and to disable FK triggers during import.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

use futures::StreamExt;
use liblzma::read::XzDecoder;
use liblzma::stream::PRESET_EXTREME;
use liblzma::write::XzEncoder;
use serde_json::{json, Value};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Row};

const FORMAT: &str = "enginer-dump/2";
/// Single-artifact dump file produced by [`export`].
const DUMP_FILE: &str = "dump.ndjson.xz";
/// Uncompressed twin accepted on import (e.g. an uploaded plain dump).
const DUMP_FILE_PLAIN: &str = "dump.ndjson";
// Legacy `enginer-dump/1` three-part layout — import fallback only.
const FILE_SCHEMA: &str = "01_schema.ndjson.xz";
const FILE_DATA: &str = "02_data.ndjson.xz";
const FILE_SYSTEM: &str = "03_system.ndjson.xz";
const FILE_MANIFEST: &str = "manifest.json";

/// Application schemas in dump order. `public` intentionally excluded —
/// `_sqlx_migrations` bookkeeping is re-applied by the app's own
/// migration runner, not by the dump.
const SCHEMAS: &[&str] = &[
    "meta",
    "core",
    "sec",
    "ops",
    "audit",
    "integration",
    "report",
];

/// Authentication secrets that never travel in a dump — excluded on BOTH
/// export and restore:
/// - `sec.session` rows hold live `session_id`+`csrf_token` pairs — a
///   dump file would be a ready-made hijack kit, and replayed sessions
///   would mint live credentials on restore.
/// - `sec.principal_credential`/`sec.api_token` hold password/token
///   hashes — offline-crackable in a shared artifact, and a hostile
///   uploaded dump could inject backdoor credentials bound to any
///   principal in any tenant (the restore DSN is BYPASSRLS).
/// A restored environment re-issues logins and service tokens
/// administratively; group/policy tables still restore because they are
/// data, not secrets, and a `security.manage` principal could already
/// mutate them through the API.
const SECRET_TABLES: &[&str] = &[
    "sec.session",
    "sec.principal_credential",
    "sec.api_token",
];

type DynError = Box<dyn std::error::Error + Send + Sync>;

fn qident(s: impl AsRef<str>) -> String {
    format!("\"{}\"", s.as_ref().replace('"', "\"\""))
}

async fn list_tables(conn: &mut PgConnection) -> Result<Vec<(String, String)>, DynError> {
    let rows = sqlx::query(
        "SELECT table_schema::text, table_name::text \
         FROM information_schema.tables \
         WHERE table_schema = ANY($1) AND table_type = 'BASE TABLE' \
         ORDER BY table_schema, table_name",
    )
    .bind(SCHEMAS)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.get::<String, _>(0), r.get::<String, _>(1)))
        .collect())
}

// ------------------------------------------------------------------ export

async fn export_table(
    conn: &mut PgConnection,
    w: &mut dyn Write,
    schema: &str,
    table: &str,
) -> Result<u64, DynError> {
    let sql = format!(
        "SELECT to_jsonb(t) AS row FROM {}.{} t",
        qident(schema),
        qident(table)
    );
    let mut stream = sqlx::query_scalar::<_, serde_json::Value>(&sql).fetch(&mut *conn);
    // Precompute the line envelope once — building a json!{} object and a
    // format!() string per row would double allocations at scale. The
    // table key is JSON-encoded, not interpolated raw — an exotic table
    // name containing '"' or '\' would corrupt the NDJSON line.
    let key = serde_json::to_string(&format!("{schema}.{table}"))?;
    let prefix = format!("{{\"table\":{key},\"row\":");
    let mut n = 0u64;
    while let Some(row) = stream.next().await {
        let row = row?;
        w.write_all(prefix.as_bytes())?;
        serde_json::to_writer(&mut *w, &row)?;
        w.write_all(b"}\n")?;
        n += 1;
        if n % 100_000 == 0 {
            w.flush()?;
            tracing::info!(%schema, %table, rows = n, "export progress");
        }
    }
    Ok(n)
}

/// Dump every sequence in `schema` — both column-owned (serial/identity)
/// and application-managed ones like `core.entity_id_seq` whose owning
/// column has no default. Written verbatim so import can replay the exact
/// `last_value/is_called` instead of guessing from MAX(column).
async fn export_sequences(
    conn: &mut PgConnection,
    w: &mut dyn Write,
    schema: &str,
) -> Result<u64, DynError> {
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT c.relname::text FROM pg_class c \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = $1 AND c.relkind = 'S' ORDER BY c.relname",
    )
    .bind(schema)
    .fetch_all(&mut *conn)
    .await?;
    let mut n = 0u64;
    for seq in names {
        // Read (last_value, is_called) straight from the sequence
        // relation — pg_sequences loses the is_called flag, so a
        // setval(v,false) state would restore off-by-one otherwise.
        let row = sqlx::query(&format!(
            "SELECT last_value, is_called FROM {}.{}",
            qident(schema),
            qident(&seq)
        ))
        .fetch_one(&mut *conn)
        .await?;
        let line = json!({
            "table": "_sequences",
            "row": {
                "name": format!("{schema}.{seq}"),
                "last_value": row.get::<i64, _>("last_value"),
                "is_called": row.get::<bool, _>("is_called"),
            },
        });
        w.write_all(serde_json::to_string(&line)?.as_bytes())?;
        w.write_all(b"\n")?;
        n += 1;
    }
    Ok(n)
}

/// Human-readable progress line → both `export.log`/`restore.log` and
/// the process tracing stream (the API surfaces the file; CLI users get
/// stderr).
fn log_line(log: &mut impl Write, msg: &str) {
    let _ = writeln!(log, "{} {msg}", chrono::Utc::now().to_rfc3339());
    let _ = log.flush();
    tracing::info!("{msg}");
}

/// Export the whole database into `dir` as a single xz-compressed
/// `dump.ndjson.xz` (plus a `manifest.json` sidecar and `export.log`
/// progress). `dsn` must be a superuser/BYPASSRLS connection.
///
/// # Errors
/// Returns an error on connection, query, IO, or serialization failure.
pub async fn export(dsn: &str, dir: &Path) -> Result<(), DynError> {
    std::fs::create_dir_all(dir)?;
    let mut log = BufWriter::new(std::fs::File::create(dir.join("export.log"))?);
    let mut conn = PgConnection::connect(dsn).await?;
    // Consistent snapshot: without this, each table streams at its own
    // commit point and a concurrent writer could produce a dump whose
    // rows violate FK relationships between tables. REPEATABLE READ
    // holds one snapshot for the whole export; READ ONLY keeps us from
    // accidentally mutating the source.
    sqlx::query("BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut conn)
        .await?;
    let tables = list_tables(&mut conn).await?;
    log_line(&mut log, &format!("tables discovered: {}", tables.len()));

    let created_at = chrono::Utc::now().to_rfc3339();
    let mut w = XzEncoder::new(
        BufWriter::with_capacity(1 << 20, std::fs::File::create(dir.join(DUMP_FILE))?),
        9 | PRESET_EXTREME, // xz -9e — maximal compression for storage
    );
    let header = json!({
        "table": "_header",
        "row": {"format": FORMAT, "created_at": created_at, "schemas": SCHEMAS},
    });
    w.write_all(serde_json::to_string(&header)?.as_bytes())?;
    w.write_all(b"\n")?;

    let mut table_rows: BTreeMap<String, u64> = BTreeMap::new();
    let mut total = 0u64;
    for (schema, table) in &tables {
        let fq = format!("{schema}.{table}");
        if SECRET_TABLES.contains(&fq.as_str()) {
            log_line(&mut log, &format!("skipped secret table {fq}"));
            continue;
        }
        let n = export_table(&mut conn, &mut w, schema, table).await?;
        table_rows.insert(fq.clone(), n);
        total += n;
        log_line(&mut log, &format!("exported {fq} rows={n}"));
    }

    // Sequences go last so a single-pass importer replays them after
    // the table rows.
    let mut sequence_rows: BTreeMap<String, u64> = BTreeMap::new();
    for schema in SCHEMAS {
        let n = export_sequences(&mut conn, &mut w, schema).await?;
        sequence_rows.insert((*schema).to_string(), n);
        total += n;
        log_line(&mut log, &format!("exported sequences {schema} rows={n}"));
    }

    let manifest = json!({
        "format": FORMAT,
        "created_at": created_at,
        "file": DUMP_FILE,
        "tables": table_rows,
        "sequences": sequence_rows,
        "total_rows": total,
    });
    // Trailer line inside the artifact — the dump is self-describing
    // even without the manifest.json sidecar.
    let trailer = json!({"table": "_manifest", "row": manifest});
    w.write_all(serde_json::to_string(&trailer)?.as_bytes())?;
    w.write_all(b"\n")?;
    // finish() writes the xz stream footer (index + CRC64) — without
    // it the file is not a valid .xz container.
    w.finish()?;
    sqlx::query("COMMIT").execute(&mut conn).await?;

    std::fs::write(
        dir.join(FILE_MANIFEST),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    log_line(&mut log, &format!("export complete total_rows={total}"));
    Ok(())
}

// ------------------------------------------------------------------ import

struct Importer<'a> {
    conn: &'a mut PgConnection,
    batch: usize,
    buf_table: Option<String>,
    buf: Vec<Value>,
    rows_read: u64,
    /// table -> quoted column list (non-generated columns only).
    /// A future migration adding a `GENERATED ALWAYS AS (expr) STORED`
    /// column must not break restores: `OVERRIDING SYSTEM VALUE` covers
    /// identity columns only, so generated columns are excluded from the
    /// insert list and recomputed by `PostgreSQL`.
    col_cache: std::collections::HashMap<String, String>,
}

impl Importer<'_> {
    /// Insertable columns for `schema.table`, cached per import run.
    async fn col_list(&mut self, schema: &str, table: &str) -> Result<String, DynError> {
        let key = format!("{schema}.{table}");
        if let Some(c) = self.col_cache.get(&key) {
            return Ok(c.clone());
        }
        let cols: Vec<String> = sqlx::query_scalar(
            "SELECT column_name::text FROM information_schema.columns \
             WHERE table_schema=$1 AND table_name=$2 \
               AND is_generated <> 'ALWAYS' \
             ORDER BY ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&mut *self.conn)
        .await?;
        if cols.is_empty() {
            return Err(format!("table {key} not found or has no insertable columns").into());
        }
        let list = cols.iter().map(qident).collect::<Vec<_>>().join(",");
        self.col_cache.insert(key, list.clone());
        Ok(list)
    }
}

impl Importer<'_> {
    async fn push(&mut self, table: &str, row: Value) -> Result<(), DynError> {
        // Pseudo-records: _header/_manifest describe the dump itself;
        // only _sequences carries replayable state.
        if table.starts_with('_') && table != "_sequences" {
            return Ok(());
        }
        if self.buf_table.as_deref() != Some(table) {
            self.flush().await?;
            self.buf_table = Some(table.to_string());
        }
        self.buf.push(row);
        if self.buf.len() >= self.batch {
            self.flush().await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DynError> {
        if self.buf.is_empty() {
            self.buf_table = None;
            return Ok(());
        }
        let table = self.buf_table.take().unwrap();
        if table == "_sequences" {
            // Replay sequence state (covers app-managed sequences like
            // core.entity_id_seq that no column default owns). Never
            // rewind a sequence: a merge-restore into a live database
            // must not hand out ids that already exist (inserts would
            // then be silently dropped by ON CONFLICT / dedup), so the
            // dump value only wins when it is ahead of the current one.
            for row in self.buf.drain(..) {
                let name = row["name"].as_str().unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                let Some(v) = row["last_value"].as_i64() else {
                    continue;
                };
                let is_called = row["is_called"].as_bool().unwrap_or(true);
                let Some((schema, seq)) = name.split_once('.') else {
                    continue;
                };
                // Only replay sequences that live in EngineR schemas —
                // a hostile dump must not setval catalog/internal
                // sequences (pg_*, information_schema, public).
                if !SCHEMAS.contains(&schema) {
                    tracing::warn!(sequence = name, "sequence outside dump schemas — skipped");
                    continue;
                }
                // Effective next value is last_value+1 when is_called.
                // Only move the sequence forward, never back — a merge
                // restore into a live db must not reissue existing ids.
                let cur = sqlx::query(&format!(
                    "SELECT last_value, is_called FROM {}.{}",
                    qident(schema),
                    qident(seq)
                ))
                .fetch_optional(&mut *self.conn)
                .await?;
                let Some(cur) = cur else {
                    tracing::warn!(sequence = name, "sequence missing in target — skipped");
                    continue;
                };
                let cur_next =
                    cur.get::<i64, _>("last_value") + i64::from(cur.get::<bool, _>("is_called"));
                let dump_next = v + i64::from(is_called);
                if dump_next > cur_next {
                    sqlx::query("SELECT setval($1::text::regclass, $2, $3)")
                        .bind(name.to_string())
                        .bind(v)
                        .bind(is_called)
                        .execute(&mut *self.conn)
                        .await?;
                }
            }
            return Ok(());
        }
        let (schema, name) = table
            .split_once('.')
            .ok_or_else(|| format!("bad table key {table}"))?;
        // Schema allowlist: the restore DSN is superuser/BYPASSRLS, so an
        // upload that names pg_catalog/public/_sqlx_migrations would
        // become arbitrary DB writes under a `security.manage` grant.
        // Only the EngineR schemas may receive rows.
        if !SCHEMAS.contains(&schema) {
            return Err(format!(
                "table {table}: schema {schema} is outside the dump schemas {SCHEMAS:?}"
            )
            .into());
        }
        // Authentication secrets are never restored from a file — an
        // uploaded dump could otherwise forge credentials, API tokens
        // or live sessions for any principal of any tenant.
        if SECRET_TABLES.contains(&table.as_str()) {
            tracing::warn!(table, "secret table rows skipped on restore");
            self.buf.clear();
            return Ok(());
        }
        // A single jsonb array parameter — one round-trip per batch.
        // Column mapping is derived from the table's composite type, so
        // dumps keep working when migrations add columns.
        let cols = self.col_list(schema, name).await?;
        let sql = format!(
            "INSERT INTO {}.{} ({cols}) OVERRIDING SYSTEM VALUE \
             SELECT {cols} FROM \
             jsonb_populate_recordset(NULL::{}.{}, $1::jsonb) \
             ON CONFLICT DO NOTHING",
            qident(schema),
            qident(name),
            qident(schema),
            qident(name)
        );
        sqlx::query(&sql)
            .bind(Value::Array(std::mem::take(&mut self.buf)))
            .execute(&mut *self.conn)
            .await?;
        Ok(())
    }
}

async fn import_file(dir: &Path, imp: &mut Importer<'_>, name: &str) -> Result<u64, DynError> {
    // Prefer the compressed file; fall back to a plain `.ndjson` twin
    // so older dumps and hand-unpacked files still restore.
    let plain = name.strip_suffix(".xz").unwrap_or(name);
    let (path, xz) = if dir.join(name).exists() {
        (dir.join(name), true)
    } else if dir.join(plain).exists() {
        (dir.join(plain), false)
    } else {
        tracing::warn!(file = name, "dump part missing — skipped");
        return Ok(0);
    };
    let file = std::fs::File::open(&path)?;
    let mut n = 0u64;
    // read::XzDecoder implements Read, not BufRead — buffer on top so the
    // decompressor is fed large chunks instead of per-line reads. The
    // `take` caps *decompressed* bytes at 8 GiB: a small `.xz` bomb that
    // would otherwise expand without bound stops with a parse error on
    // the truncated tail line.
    const MAX_DECOMPRESSED: u64 = 8 << 30;
    let inner: Box<dyn std::io::Read> = if xz {
        Box::new(XzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut reader: Box<dyn BufRead> =
        Box::new(BufReader::with_capacity(1 << 20, inner.take(MAX_DECOMPRESSED)));
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let mut v: Value = serde_json::from_str(line)?;
        let table = v["table"].as_str().unwrap_or("").to_string();
        if table == "_header" {
            let fmt = v["row"]["format"].as_str().unwrap_or("");
            if !fmt.starts_with("enginer-dump/") {
                tracing::warn!(format = %fmt, "unrecognised dump format — continuing anyway");
            }
        }
        // take() moves the row out without cloning — the row can be a
        // multi-KB statement object.
        imp.push(&table, v["row"].take()).await?;
        n += 1;
        if n % 100_000 == 0 {
            tracing::info!(file = name, rows = n, "import progress");
        }
    }
    Ok(n)
}

/// Cheap sanity check on a would-be dump file: the first NDJSON line
/// must parse as `{"table": ...}` — for `.xz` the container is opened
/// and the first line actually decoded. Used by the upload endpoint to
/// reject arbitrary blobs before they can reach [`import`].
#[must_use]
pub fn probe_dump(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut reader: Box<dyn BufRead> = if path.extension().is_some_and(|e| e == "xz") {
        Box::new(BufReader::with_capacity(1 << 16, XzDecoder::new(file)))
    } else {
        Box::new(BufReader::with_capacity(1 << 16, file))
    };
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim_end().is_empty() {
        return false;
    }
    serde_json::from_str::<Value>(line.trim_end())
        .ok()
        .and_then(|v| v["table"].as_str().map(String::from))
        .is_some_and(|t| !t.is_empty())
}

/// Replay a dump directory into the database at `dsn`.
/// Idempotent (`ON CONFLICT DO NOTHING`); needs superuser for
/// `session_replication_role=replica`. Accepts both the single-file
/// layout (`dump.ndjson[.xz]`) and the legacy three-part layout.
/// Progress lands in `<dir>/restore.log`.
///
/// # Errors
/// Returns an error on connection, query, IO, decode, or parse failure.
pub async fn import(dsn: &str, dir: &Path, batch: usize) -> Result<(), DynError> {
    let files: Vec<&str> = if dir.join(DUMP_FILE).exists() || dir.join(DUMP_FILE_PLAIN).exists() {
        vec![DUMP_FILE]
    } else {
        // Legacy enginer-dump/1 three-part layout.
        let manifest_path = dir.join(FILE_MANIFEST);
        if manifest_path.exists() {
            let m: Value = serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
            let fmt = m["format"].as_str().unwrap_or("");
            if !fmt.starts_with("enginer-dump/") {
                tracing::warn!(format = %fmt, "manifest format mismatch — continuing anyway");
            }
        } else {
            tracing::warn!("no manifest.json in dump dir — format unchecked");
        }
        vec![FILE_SCHEMA, FILE_DATA, FILE_SYSTEM]
    };

    std::fs::create_dir_all(dir)?;
    let mut log = BufWriter::new(std::fs::File::create(dir.join("restore.log"))?);
    log_line(&mut log, &format!("restore started files={files:?}"));

    let mut conn = PgConnection::connect(dsn).await?;
    // Disable FK/constraint triggers so row order within a file does not
    // matter; requires superuser. Warn only — ordered files still work
    // for most datasets when it fails.
    if let Err(e) = sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut conn)
        .await
    {
        log_line(
            &mut log,
            &format!("could not disable triggers ({e}) — continuing with constraint checks on"),
        );
    }

    let mut imp = Importer {
        conn: &mut conn,
        batch,
        buf_table: None,
        buf: Vec::with_capacity(batch),
        rows_read: 0,
        col_cache: std::collections::HashMap::new(),
    };
    for f in files {
        let n = import_file(dir, &mut imp, f).await?;
        imp.rows_read += n;
        log_line(&mut log, &format!("imported {f} rows={n}"));
    }
    imp.flush().await?;
    log_line(&mut log, &format!("import complete rows={}", imp.rows_read));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("enginer-dump-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn qident_quotes() {
        assert_eq!(qident("entity"), "\"entity\"");
        assert_eq!(qident("a\"b"), "\"a\"\"b\"");
        assert_eq!(qident(""), "\"\"");
    }

    #[test]
    fn probe_accepts_plain_ndjson() {
        let dir = tmp_dir("plain");
        let p = dir.join("dump.ndjson");
        std::fs::write(
            &p,
            "{\"table\":\"_header\",\"row\":{}}\n{\"table\":\"core.entity\",\"row\":{}}\n",
        )
        .unwrap();
        assert!(probe_dump(&p));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_accepts_xz_ndjson() {
        let dir = tmp_dir("xz");
        let p = dir.join("dump.ndjson.xz");
        let mut enc = XzEncoder::new(std::fs::File::create(&p).unwrap(), 6);
        enc.write_all(b"{\"table\":\"_header\",\"row\":{\"format\":\"enginer-dump/2\"}}\n")
            .unwrap();
        enc.finish().unwrap();
        assert!(probe_dump(&p));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_rejects_garbage_and_empty() {
        let dir = tmp_dir("bad");
        for (name, bytes) in [
            ("empty.ndjson", b"".as_slice()),
            ("json_obj.ndjson", b"{\"not\":\"a dump\"}\n".as_slice()),
            ("json_no_table.ndjson", b"{\"row\":{}}\n".as_slice()),
            (
                "empty_table.ndjson",
                b"{\"table\":\"\",\"row\":{}}\n".as_slice(),
            ),
            ("text.ndjson", b"hello world\n".as_slice()),
            ("binary.ndjson", &[0u8, 159, 146, 150][..]),
        ] {
            let p = dir.join(name);
            std::fs::write(&p, bytes).unwrap();
            assert!(!probe_dump(&p), "{name} must be rejected");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_rejects_truncated_xz() {
        let dir = tmp_dir("xztrunc");
        let p = dir.join("dump.ndjson.xz");
        // xz magic + truncated stream: the container opens but yields
        // no decodable first line.
        std::fs::write(&p, [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00, 0x01]).unwrap();
        assert!(!probe_dump(&p));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_rejects_missing_file() {
        let dir = tmp_dir("missing");
        assert!(!probe_dump(&dir.join("nope.ndjson")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
