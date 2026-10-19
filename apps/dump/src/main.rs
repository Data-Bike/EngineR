//! `EngineR` database dump/restore tool.
//!
//! Streams every application table out to NDJSON files (one JSON object
//! per line: `{"table":"<schema>.<name>","row":{...}}`) and can replay
//! them back into a database. Rows are never materialised in memory as a
//! set — export uses a lazy `fetch()` stream, import batches rows through
//! `jsonb_populate_recordset`, so the tool scales to arbitrarily large
//! databases.
//!
//! Layout (mirrors the normative split):
//!   `01_schema.ndjson`  — meta.* : object types, predicates, identity
//!                       policies, schema revisions, reference data.
//!   `02_data.ndjson`    — core.* : entities, statements (object graph
//!                       edges), files, workflow/domain instances.
//!   `03_system.ndjson`  — sec/ops/audit/integration/report : users,
//!                       credentials, roles/policies, sessions, outbox,
//!                       audit chain, connectors — everything else.
//!   manifest.json     — format version, counts, table->file map.
//!
//! Tables are discovered at runtime from `information_schema` and routed
//! by schema, so new migrations are picked up automatically — keep the
//! SCHEMA_* grouping in sync when adding a new schema.
//!
//! Usage:
//!   enginer-dump export --dsn <url> --dir <`out_dir`>
//!   enginer-dump import --dsn <url> --dir <`dump_dir`> [--batch 500]
//!
//! The tool must connect as a superuser / BYPASSRLS role to see all
//! tenants and to disable FK triggers during import.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use futures::StreamExt;
use serde_json::{json, Value};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Row};

const FORMAT: &str = "enginer-dump/1";
const FILE_SCHEMA: &str = "01_schema.ndjson";
const FILE_DATA: &str = "02_data.ndjson";
const FILE_SYSTEM: &str = "03_system.ndjson";
const FILE_MANIFEST: &str = "manifest.json";
const DEFAULT_BATCH: usize = 500;

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

fn part_for(schema: &str) -> &'static str {
    match schema {
        "meta" => FILE_SCHEMA,
        "core" => FILE_DATA,
        _ => FILE_SYSTEM,
    }
}

type DynError = Box<dyn std::error::Error + Send + Sync>;

fn usage() -> ! {
    eprintln!(
        "usage:\n  enginer-dump export --dsn <postgres-url> --dir <out_dir>\n  \
         enginer-dump import --dsn <postgres-url> --dir <dump_dir> [--batch <n>]"
    );
    std::process::exit(2);
}

struct Cli {
    command: String,
    dsn: String,
    dir: PathBuf,
    batch: usize,
}

fn parse_cli() -> Cli {
    let mut args = std::env::args().skip(1);
    let command = match args.next().as_deref() {
        Some(c @ ("export" | "import")) => c.to_string(),
        _ => usage(),
    };
    let mut dsn = std::env::var("ENGINER_DATABASE_URL").unwrap_or_default();
    let mut dir = None;
    let mut batch = DEFAULT_BATCH;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--dsn" => dsn = args.next().unwrap_or_default(),
            "--dir" | "--out" => dir = args.next().map(PathBuf::from),
            "--batch" => {
                batch = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_BATCH);
            }
            _ => usage(),
        }
    }
    if dsn.is_empty() {
        eprintln!("error: --dsn or ENGINER_DATABASE_URL required");
        usage();
    }
    let Some(dir) = dir else {
        eprintln!("error: --dir required");
        usage();
    };
    Cli { command, dsn, dir, batch }
}

fn qident(s: impl AsRef<str>) -> String {
    format!("\"{}\"", s.as_ref().replace('"', "\"\""))
}

async fn list_tables(
    conn: &mut PgConnection,
) -> Result<Vec<(String, String)>, DynError> {
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
    w: &mut BufWriter<std::fs::File>,
    schema: &str,
    table: &str,
) -> Result<u64, DynError> {
    let sql = format!(
        "SELECT to_jsonb(t) AS row FROM {}.{} t",
        qident(schema),
        qident(table)
    );
    let mut stream =
        sqlx::query_scalar::<_, serde_json::Value>(&sql).fetch(&mut *conn);
    let mut n = 0u64;
    while let Some(row) = stream.next().await {
        let row = row?;
        let line = json!({
            "table": format!("{schema}.{table}"),
            "row": row,
        });
        w.write_all(serde_json::to_string(&line)?.as_bytes())?;
        w.write_all(b"\n")?;
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
    w: &mut BufWriter<std::fs::File>,
    schema: &str,
) -> Result<u64, DynError> {
    let mut stream = sqlx::query(
        "SELECT sequencename::text, last_value, cycle FROM pg_sequences \
         WHERE schemaname = $1 ORDER BY sequencename",
    )
    .bind(schema)
    .fetch(&mut *conn);
    let mut n = 0u64;
    while let Some(r) = stream.next().await {
        let r = r?;
        let last: Option<i64> = r.get("last_value");
        let line = json!({
            "table": "_sequences",
            "row": {
                "name": format!("{}.{}", schema,
                                r.get::<String, _>("sequencename")),
                // last_value NULL <=> is_called = false
                "last_value": last,
                "is_called": last.is_some(),
            },
        });
        w.write_all(serde_json::to_string(&line)?.as_bytes())?;
        w.write_all(b"\n")?;
        n += 1;
    }
    Ok(n)
}

async fn run_export(cli: &Cli) -> Result<(), DynError> {
    std::fs::create_dir_all(&cli.dir)?;
    let mut conn = PgConnection::connect(&cli.dsn).await?;
    let tables = list_tables(&mut conn).await?;
    tracing::info!(tables = tables.len(), "tables discovered");

    let mut writers: BTreeMap<&'static str, BufWriter<std::fs::File>> =
        BTreeMap::new();
    let mut manifest_files: BTreeMap<String, Value> = BTreeMap::new();
    let mut table_map: BTreeMap<String, String> = BTreeMap::new();

    for f in [FILE_SCHEMA, FILE_DATA, FILE_SYSTEM] {
        let path = cli.dir.join(f);
        writers.insert(f, BufWriter::new(std::fs::File::create(&path)?));
    }

    for (schema, table) in &tables {
        let f = part_for(schema);
        let fq = format!("{schema}.{table}");
        tracing::info!(table = %fq, file = f, "exporting");
        let n = export_table(&mut conn, writers.get_mut(f).unwrap(), schema, table)
            .await?;
        table_map.insert(fq.clone(), f.to_string());
        let entry = manifest_files
            .entry(f.to_string())
            .or_insert_with(|| json!({"rows": 0, "tables": []}));
        entry["rows"] =
            json!(entry["rows"].as_u64().unwrap_or(0) + n);
        entry["tables"]
            .as_array_mut()
            .unwrap()
            .push(json!({"table": fq, "rows": n}));
        tracing::info!(table = %fq, rows = n, "exported");
    }

    // Sequences go last in each part so a single-pass importer replays
    // them after the table rows.
    for schema in SCHEMAS {
        let f = part_for(schema);
        let n = export_sequences(&mut conn, writers.get_mut(f).unwrap(), schema)
            .await?;
        let entry = manifest_files.get_mut(f).unwrap();
        entry["rows"] = json!(entry["rows"].as_u64().unwrap_or(0) + n);
    }

    for w in writers.values_mut() {
        w.flush()?;
    }

    let manifest = json!({
        "format": FORMAT,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "files": manifest_files,
        "tables": table_map,
        "note": "NDJSON lines {\"table\":\"<schema>.<name>\",\"row\":{..}}; \
                 row order within a file is grouped by table.",
    });
    std::fs::write(
        cli.dir.join(FILE_MANIFEST),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    tracing::info!(dir = %cli.dir.display(), "export complete");
    Ok(())
}

// ------------------------------------------------------------------ import

struct Importer<'a> {
    conn: &'a mut PgConnection,
    batch: usize,
    buf_table: Option<String>,
    buf: Vec<Value>,
    inserted: u64,
}

impl Importer<'_> {
    async fn push(&mut self, table: &str, row: Value) -> Result<(), DynError> {
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
            // Replay exact sequence state (covers app-managed sequences
            // like core.entity_id_seq that no column default owns).
            for row in self.buf.drain(..) {
                let name = row["name"].as_str().unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                match row["last_value"].as_i64() {
                    Some(v) => sqlx::query(
                        "SELECT setval($1::text::regclass, $2, $3)")
                        .bind(name.to_string())
                        .bind(v)
                        .bind(row["is_called"].as_bool().unwrap_or(true))
                        .execute(&mut *self.conn)
                        .await
                        .map(|_| ())?,
                    // never called: setval(1, is_called=false)
                    None => sqlx::query(
                        "SELECT setval($1::text::regclass, 1, false)")
                        .bind(name.to_string())
                        .execute(&mut *self.conn)
                        .await
                        .map(|_| ())?,
                }
            }
            return Ok(());
        }
        let (schema, name) = table
            .split_once('.')
            .ok_or_else(|| format!("bad table key {table}"))?;
        // A single jsonb array parameter — one round-trip per batch.
        // Column mapping is derived from the table's composite type, so
        // dumps keep working when migrations add columns.
        let sql = format!(
            "INSERT INTO {}.{} OVERRIDING SYSTEM VALUE SELECT * FROM \
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

async fn import_file(
    cli: &Cli,
    imp: &mut Importer<'_>,
    name: &str,
) -> Result<u64, DynError> {
    let path = cli.dir.join(name);
    if !path.exists() {
        tracing::warn!(file = name, "dump part missing — skipped");
        return Ok(0);
    }
    let file = std::fs::File::open(&path)?;
    let reader = BufReader::with_capacity(1 << 20, file);
    let mut n = 0u64;
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line)?;
        let table = v["table"].as_str().unwrap_or("").to_string();
        imp.push(&table, v["row"].clone()).await?;
        n += 1;
        if n % 100_000 == 0 {
            tracing::info!(file = name, rows = n, "import progress");
        }
    }
    Ok(n)
}

async fn run_import(cli: &Cli) -> Result<(), DynError> {
    let manifest_path = cli.dir.join(FILE_MANIFEST);
    if manifest_path.exists() {
        let m: Value = serde_json::from_str(&std::fs::read_to_string(
            manifest_path,
        )?)?;
        if m["format"].as_str() != Some(FORMAT) {
            tracing::warn!(format = %m["format"], "manifest format mismatch — continuing anyway");
        }
    }

    let mut conn = PgConnection::connect(&cli.dsn).await?;
    // Disable FK/constraint triggers so row order within a file does not
    // matter; requires superuser. Warn only — ordered files still work
    // for most datasets when it fails.
    if let Err(e) = sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut conn)
        .await
    {
        tracing::warn!("could not disable triggers ({e}) — \
                        continuing with constraint checks on");
    }

    let mut imp = Importer {
        conn: &mut conn,
        batch: cli.batch,
        buf_table: None,
        buf: Vec::with_capacity(cli.batch),
        inserted: 0,
    };
    for f in [FILE_SCHEMA, FILE_DATA, FILE_SYSTEM] {
        let n = import_file(cli, &mut imp, f).await?;
        imp.inserted += n;
        tracing::info!(file = f, rows = n, "imported");
    }
    imp.flush().await?;
    tracing::info!(rows = imp.inserted, "import complete");
    Ok(())
}

#[async_std::main]
async fn main() -> Result<(), DynError> {
    enginer_observability::init("info");
    let cli = parse_cli();
    match cli.command.as_str() {
        "export" => run_export(&cli).await,
        "import" => run_import(&cli).await,
        _ => unreachable!(),
    }
}
