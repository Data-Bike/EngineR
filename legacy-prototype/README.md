# EngineR — legacy prototype (reference only)

This folder contains the original Rocket 0.5-rc prototype. It is **not** part of the
new Cargo workspace (see root `Cargo.toml` → `exclude = ["legacy-prototype"]`).

It is kept as a reference and mapping source for the migration described in
[`../docs/24-current-code-migration.md`](../docs/24-current-code-migration.md).

What is reused from it (concepts, not code):

- type / field / link / dictionary concepts → mapped to `TypeDefinition` /
  `PredicateDefinition` / edge statements / classifier entities in the new meta-kernel;
- authentication strategy shape → replaced by OIDC + Argon2id + session model;
- SQL model (`src/model/sql/sql_db.sql`, `Model.rise`) → mapping reference for the
  SKR schema bundle and the legacy data extractor (Phase 4).

What is **not** reused:

- string-built SQL helpers (`pool.rs` `get_insert`/`get_update`/...) — injection risk;
- `lazy_static` pool with `block_on` — replaced by `AsyncStdDbExecutor`;
- allow-by-absence permission model — replaced by default-deny PDP/PEP;
- dynamic `CREATE TABLE`/`ALTER TABLE` in request path — replaced by managed migrations;
- plaintext DB password in `config.toml` — replaced by secret refs.
