//! `enginer-dump` CLI — thin wrapper over the `enginer_dump` library.

use std::path::PathBuf;

const DEFAULT_BATCH: usize = 500;

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
                    .and_then(|v| v.parse::<usize>().ok())
                    .filter(|n| *n > 0)
                    .unwrap_or_else(|| {
                        eprintln!("error: --batch must be a positive integer");
                        usage();
                    });
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
    Cli {
        command,
        dsn,
        dir,
        batch,
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    enginer_observability::init("info");
    let cli = parse_cli();
    match cli.command.as_str() {
        "export" => enginer_dump::export(&cli.dsn, &cli.dir).await,
        "import" => enginer_dump::import(&cli.dsn, &cli.dir, cli.batch).await,
        _ => unreachable!(),
    }
}
