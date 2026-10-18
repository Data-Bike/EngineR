"""Project knowledge-base bootstrap — publishes a `kb_article` schema and
writes one entity per knowledge topic. Everything goes through MCP
(tools/call) — no raw SQL, no raw HTTP API.

Run:  python docs/testing/warehouse/kb_provision.py
"""
import json
import os
import sys
import time
import urllib.request
import urllib.error

HERE = os.path.dirname(os.path.abspath(__file__))
MCP = os.environ.get("ENGINER_MCP", "http://localhost:8020/mcp")
TOK = json.load(open(os.path.join(HERE, ".wh_tokens.json")))["admin"]
HDR = {"Content-Type": "application/json",
       "Accept": "application/json, text/event-stream",
       "Authorization": "Bearer " + TOK}
_idn = [0]


def rpc(method, params=None):
    _idn[0] += 1
    body = {"jsonrpc": "2.0", "id": _idn[0], "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(MCP, data=json.dumps(body).encode(),
                                 headers=HDR)
    try:
        raw = urllib.request.urlopen(req, timeout=60).read().decode()
    except urllib.error.HTTPError as e:
        return {"_http": e.code,
                "_body": e.read()[:600].decode("utf-8", "replace")}
    for line in raw.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip()
            break
    return json.loads(raw)


def call(tool, args=None):
    d = rpc("tools/call", {"name": tool, "arguments": args or {}})
    if "error" in d:
        return {"_rpc_err": d["error"]}
    r = d.get("result") or {}
    txt = (r.get("content") or [{}])[0].get("text", "")
    try:
        out = json.loads(txt)
    except Exception:
        out = {"_text": txt[:3000]}
    if r.get("isError"):
        if isinstance(out, dict):
            out["_isError"] = True
        else:
            out = {"_isError": True, "_text": txt[:3000]}
    return out


def failed(resp):
    return bool(resp.get("_rpc_err") or resp.get("_http")
                or resp.get("_isError"))


def show(resp, n=600):
    return json.dumps(resp, ensure_ascii=False)[:n]


# ---------------------------------------------------------------- schema

KB_TYPE = {
    "type_id": "kb_article", "stable_alias": "kb_article",
    "entity_class": "data", "label": "KB Article",
    "fields": [
        {"kind": "text", "alias": "kb_id", "label": "KB id",
         "identity": True, "required": True},
        {"kind": "text", "alias": "kb_title", "label": "Title",
         "required": True},
        {"kind": "text", "alias": "kb_category", "label": "Category",
         "required": True},
        {"kind": "text", "alias": "kb_summary", "label": "Summary"},
        {"kind": "text", "alias": "kb_content", "label": "Content"},
        {"kind": "integer", "alias": "kb_sort", "label": "Sort order"},
        {"kind": "text", "alias": "kb_tags",
         "label": "Tags (comma-separated)"},
    ],
}

KB_SCALARS = [
    {"stable_alias": "kb_id", "scalar_kind": "text", "label": "KB id",
     "participates_in_identity": True},
    {"stable_alias": "kb_title", "scalar_kind": "text", "label": "Title"},
    {"stable_alias": "kb_category", "scalar_kind": "text",
     "label": "Category"},
    {"stable_alias": "kb_summary", "scalar_kind": "text",
     "label": "Summary"},
    {"stable_alias": "kb_content", "scalar_kind": "text",
     "label": "Content"},
    {"stable_alias": "kb_sort", "scalar_kind": "integer",
     "label": "Sort order"},
    {"stable_alias": "kb_tags", "scalar_kind": "text",
     "label": "Tags (comma-separated)"},
]

KB_EDGES = [
    {"stable_alias": "kb_related", "is_edge": True,
     "domain": "kb_article", "range": "kb_article",
     "label": "Related article", "inverse_label": "Related articles"},
]

# ---------------------------------------------------------------- articles

ARTICLES = [
    dict(kb_id="kb.onboarding", sort=0, category="meta",
         title="How to query this knowledge base",
         summary="Entry point: every project fact is a kb_article entity; "
                 "query via entities.list / entities.get / entities.query / "
                 "entities.graph through MCP.",
         tags="kb,onboarding,mcp,howto",
         content="""This database holds a project knowledge base for the EngineR repo.
All knowledge is stored as entities of type `kb_article` (edge predicate
`kb_related` links related articles). Work through MCP only:
POST http://localhost:8020/mcp, JSON-RPC 2.0, Bearer token from
docs/testing/warehouse/.wh_tokens.json (see kb.mcp.contract for the
handshake).

kb_article fields: kb_id (identity, required), kb_title (required),
kb_category (required), kb_summary, kb_content (full text), kb_sort
(integer), kb_tags (comma-separated string).

Suggested queries:
- List everything:  entities.list {"type": "kb_article", "limit": 100}
- Free-text search: entities.list {"type": "kb_article", "search": "docker"}
- Read one:         entities.get {"id": "<public_id from list>"}
- By category:      entities.query {"query": {"filter":
                    {"field": "kb_category", "op": "eq", "value": "mcp"}}}
  (ops supported by the filter object: eq | prefix | contains)
- Neighbourhood:    entities.graph {"id": "<public_id>", "direction": "both",
                    "max_depth": 2} — follows kb_related edges.

kb_id catalog (stable identity keys, use entities.query with op "eq" on
field kb_id to fetch one directly):
  kb.onboarding (meta)       — this article
  kb.layout (layout)         — repo directory map
  kb.stack.run (ops)         — docker compose services, ports, rebuilds
  kb.creds (security)        — DB/logins/tokens, never-commit list
  kb.db.access (data)        — psql access, schemas, migration order
  kb.db.state (data)         — current wiped-DB state and what was republished
  kb.mcp.contract (mcp)      — endpoint, handshake, request/response shape
  kb.mcp.gotchas (mcp)       — arg names, error channels, response shapes
  kb.schema.lifecycle (schema) — draft->validate->impact->publish, retire
  kb.tests (testing)         — cargo/vitest/e2e/MCP test commands
  kb.frontend (frontend)     — Nuxt 4 structure, i18n, pages
  kb.graph (frontend)        — GraphView force-bubble architecture
  kb.conventions (conventions) — code style rules + documented deviations
  kb.pitfalls (pitfalls)     — known traps (docker exec -i, nul file, ...)
  kb.docs.index (reference)  — index of docs/ normative specs + test dirs
  kb.phases (status)         — implementation phase status / test counts
"""),

    dict(kb_id="kb.layout", sort=10, category="layout",
         title="Repository layout",
         summary="Rust workspace + apps/api + apps/worker + apps/mcp; "
                 "web/ typed client; nuxt/ admin UI; docs/ specs; "
                 "legacy-prototype excluded.",
         tags="layout,workspace,rust,nuxt",
         content="""EngineR is a Rust workspace + Nuxt UI monorepo.

- apps/api — Rocket HTTP API (enginer-api), port 8010 in docker.
  Route modules: authn.rs, entity_pg.rs, schema_admin_routes.rs,
  audit_pg.rs, skr_pg.rs, files_pg.rs, workspace_routes.rs (shared
  scaffolding: CapabilityGuard/PEP, ApiEnvelope), seed.rs (demo seed).
- apps/worker — enginer-worker: ops.outbox drain loop + monitor scheduler.
- apps/mcp — MCP bridge binary (enginer-mcp), exposes the HTTP API as
  ~95 MCP tools on :8020.
- crates/ — domain, application, contracts, observability, db-runtime,
  persistence, security (domain/pdp/api), schema-registry, identity,
  query-engine, test-support, workflow, risk-engine, files, reporting,
  connectors, integration, audit-persistence.
- migrations/ — sqlx migrations for PostgreSQL schemas
  core / meta / sec / ops / audit (+ integration tables).
- web/ — typed API client @enginer/api-client (TypeScript, Vitest).
- nuxt/ — Nuxt 4 admin UI (SPA, Vue 3, Nuxt UI 4, i18n, Pinia).
- docs/ — normative specifications, source of truth for design
  (see kb.docs.index); docs/testing/ holds MCP test campaigns.
- schema-packages/skr-core/ — SKR domain bundle (15 types, ~60
  predicates, identity policies, views, workflows, Excel mapping).
- legacy-prototype/ — old Rocket 0.5-rc prototype, EXCLUDED from the
  workspace; do not build or reference it.
"""),

    dict(kb_id="kb.stack.run", sort=20, category="ops",
         title="Running the stack (docker compose)",
         summary="docker compose up -d starts postgres/api/mcp/web/worker; "
                 "ports 6543/8010/8020/3000.",
         tags="docker,compose,ports,ops",
         content="""Start/stop:
  docker compose up -d            # postgres, api, mcp, web, worker
  docker compose build api        # rebuild after Rust changes in apps/api
  docker compose build web        # rebuild after nuxt/ changes
  docker compose up -d postgres   # DB only (for cargo tests)

Services (docker-compose.yml):
  postgres  enginer-postgres  localhost:6543 -> container 5432 (postgres:17-alpine, healthy via pg_isready)
  api       enginer-api       http://localhost:8010  (ROCKET_PORT=8010, ENGINER_CONFIG=/app/config/enginer.docker.toml)
  mcp       enginer-mcp       http://localhost:8020/mcp (ENGINER_API_URL=http://enginer-api:8010)
  web       enginer-web       http://localhost:3000  (Nuxt production build)
  worker    enginer-worker    no port (outbox drain + monitor scheduler)

Health: curl http://localhost:8010/health/live (Rocket health route).
Local (non-docker) run: ENGINER_CONFIG=config/enginer.local.toml
cargo run -p enginer-api  -> http://localhost:8000.

Compose env of note: ENGINER_SEED_DEMO=true (idempotent demo seed at API
startup: tenant, personas, groups+policy, demo entities/ops rows),
ENGINER_DEMO_PASSWORD=enginer-demo, ENGINER_DATABASE_URL uses role
enginer_app for api and enginer_worker for worker.
"""),

    dict(kb_id="kb.creds", sort=30, category="security",
         title="Credentials, tokens and secrets",
         summary="Postgres roles, demo personas (password enginer-demo), "
                 "MCP bearer tokens in .wh_tokens.json — gitignored.",
         tags="credentials,tokens,passwords,security",
         content="""PostgreSQL (host localhost:6543, db `enginer`):
  superuser for admin/psql: enginer_admin / enginer_admin
  app runtime role:         enginer_app / enginer_app
  worker role:              enginer_worker / enginer_worker
  Full DSN: postgres://enginer_admin:enginer_admin@localhost:6543/enginer

Demo UI/API logins (seeded personas; ENGINER_SEED_DEMO=true):
  admin, security_officer, manager, risk_analyst
  Password for ALL: value of ENGINER_DEMO_PASSWORD env = `enginer-demo`
  in docker-compose. Login form field name is `username` (not `email`).

MCP/API bearer tokens for QA agents: generated by
docs/testing/warehouse/bootstrap.py into
docs/testing/warehouse/.wh_tokens.json — keys `admin` (full ~95 tools),
`analyst`, `operator` (capability-filtered subsets), plus *_pid
principal ids. Header: Authorization: Bearer egr_...

NEVER COMMIT: *.wh_tokens.json, *.wh_cookie.txt, nuxt/test-results/,
nuxt/playwright-report/, stray *.png screenshots, ad-hoc build-*.txt or
*.json API dumps at repo root.
"""),

    dict(kb_id="kb.db.access", sort=40, category="data",
         title="Database access and migration order",
         summary="psql via `docker exec -i enginer-postgres`; schemas "
                 "core/meta/sec/ops/audit/integration; migration list.",
         tags="postgres,psql,migrations,rls",
         content="""Connect: docker exec -i enginer-postgres psql -U enginer_admin -d enginer
The `-i` flag is REQUIRED — without it stdin/heredocs are silently
dropped (classic trap, see kb.pitfalls). Apply a migration from Windows:
  Get-Content migrations/<file>.sql -Raw | docker exec -i enginer-postgres psql -U enginer_admin -d enginer

PostgreSQL schemas: core (entities/statements/identity keys/tenant/
uploads/file_object), meta (schema registry drafts/revisions), sec
(principals/credentials/sessions/groups/policy rules), ops (outbox),
audit (event hash chain, chain_checkpoint, retention_policy),
integration (monitors/connectors).

Migration order (filenames):
  20260803000001_baseline.sql
  20260803000002_core_entity.sql
  20260803000003_meta_registry.sql
  20260803000004_sec_security.sql
  20260803000005_ops_outbox.sql
  20260803000006_audit_event.sql
  20260803000007_integration.sql
  20260803000008_core_statement.sql
  20260803000009_audit_chain.sql
  20261018000013_core_skr_ops.sql
  20261018000014_core_uploads.sql
  20261018000015_ops_outbox_worker.sql
  20261018000016_monitor_worker.sql
  20261018000017_audit_http_kind.sql
(Migrations already applied inside the docker postgres volume; the list
is for fresh setups.)

Row-level security: every API transaction sets RLS context via
set_config and resets before return; connect as enginer_app to observe
RLS-filtered data (enginer_admin bypasses).

Live-DB scenario tests use
ENGINER_TEST_DATABASE_URL=postgres://enginer_app:enginer_app@localhost:6543/enginer
"""),

    dict(kb_id="kb.db.state", sort=50, category="data",
         title="Current database state",
         summary="Fresh DB: only sec.* (users/credentials/groups/tokens) "
                 "and core.tenant; kb_article is the first published schema.",
         tags="state,empty,schema,baseline",
         content="""State as of this KB bootstrap (2026-10 wipe):
- The domain catalog was EMPTY: schema.published returned {"data": []}.
- Surviving data: sec.* (demo users, Argon2 credentials, capability/data
  groups, API tokens — all valid) and core.tenant. No entities, no types,
  no predicates.
- entities.create fails until at least one schema revision is published.
- This session published the `kb_article` type (+ kb_* scalar predicates
  + kb_related edge) as the first published revision; the published
  revision_id is recorded in docs/testing/warehouse/kb_write_report.md.
- If the catalog needs the SKR domain again, republish via
  docs/testing/warehouse/provision_v2.py (warehouse domain) or load
  schema-packages/skr-core/ (SKR bundle loader:
  crates/schema-registry/src/bundle.rs).
- IMPORTANT (from pg_scenarios semantics): creating a draft WITHOUT
  base_revision normally inherits the latest published revision — an
  empty-base draft + publish would wipe the catalog. Here that behaviour
  was safe because the catalog was already empty; on a populated DB
  always pass base_revision or expect inheritance.
"""),

    dict(kb_id="kb.mcp.contract", sort=60, category="mcp",
         title="MCP bridge contract",
         summary="POST :8020/mcp, JSON-RPC 2.0 streamable HTTP, protocol "
                 "2025-06-18; initialize -> initialized -> tools/list -> "
                 "tools/call.",
         tags="mcp,jsonrpc,protocol,handshake",
         content="""Endpoint: POST http://localhost:8020/mcp (env override
ENGINER_MCP). Streamable HTTP transport, JSON-RPC 2.0, protocol version
2025-06-18. Source of truth for tool names/args = tools/list
inputSchema (includes `required` and `x-location`).

Headers: Content-Type: application/json;
Accept: application/json, text/event-stream;
Authorization: Bearer <egr_... token>.

Session sequence (once per session):
1. initialize — params {"protocolVersion":"2025-06-18",
   "capabilities":{}, "clientInfo":{"name":"...","version":"..."}}
   -> returns serverInfo/capabilities/instructions.
2. notifications/initialized — notification (no id), HTTP 202 empty.
3. tools/list -> {result:{tools:[...]}}; cheap, cache it. Filtered by
   token capabilities: admin sees 95 tools.
4. tools/call {"name":"<tool>","arguments":{...}}.

Transport details:
- Response may be plain JSON OR SSE — if lines start with `data:` take
  that payload.
- GET /mcp -> 405. Batch arrays are supported; JSON-RPC `id` is echoed.
- Unknown method / resources/list / prompts/list -> error.code -32601.
- Missing/bad auth -> HTTP 401 or error.code -32001.

x-location on each inputSchema property: path (URL path arg — must be a
real public_id/catalog id; slashes/semicolons/`..` rejected), query,
body (default when absent), header (e.g. if_match -> If-Match),
binary_body (base64 -> raw bytes for upload chunks).

Python client recipe: docs/testing/warehouse/mcp_probe.py (rpc_raw +
call wrappers); SKILL.md section 8 has a minimal standalone client.
"""),

    dict(kb_id="kb.mcp.gotchas", sort=70, category="mcp",
         title="MCP arg names, error channels, response shapes",
         summary="Two failure channels (error + result.isError); verified "
                 "arg names table; dedup/409/501 semantics.",
         tags="mcp,gotchas,errors,arguments",
         content="""TWO error channels — check BOTH:
1. Top-level JSON-RPC `error`: -32601 unknown method, -32001 bad token,
   -32602 capability denial ("capability '<cap>' not granted") — checked
   BEFORE arg validation, so malformed args still deny first.
2. `result.isError == true`: tool-level upstream API error; the text in
   content[0].text carries the API status (403/404/409/422...).
A revoked token can return HTTP 200 + isError for ~30s (warm context
cache) — treat as expired.

Verified argument names (derive the rest from tools/list inputSchema;
wrong names -> `unknown argument '<name>'` tool error):
  entities.list          filter arg `type` (NOT type_alias); also search/limit/cursor
  entities.get/graph     `id` path arg (entity public_id uuid)
  entities.create        `type_id` (alias works) + `fields` {alias: value}
                         + identity_mode (deduplicate default | reject | create_candidate)
  entities.update        `id` + `fields` (alias->value) or predicate_id+value; `revision` = optimistic concurrency
  entities.query         `query` object (AST); non-object -> 422
  schema.type_get        `alias` (path)
  schema.draft_update    `id`, `display_name`, `semantic_version`, `types`[], `predicates`[], `changes`[], `if_match`
  schema.draft_publish   `id`, `reason`, `if_match`
  relations.create       `id` (subject, path) + `predicate` + `target_entity_id`/`target_public_id`
  permissions.group_update  `principals` are LOGIN strings, not UUIDs
  permissions.group_create  `class`: data|capability
  assessment.get/create  `id` = ENTITY public_id (/entities/{id}/assessment)
  tasks.list             requires a workflow `id`; bare list rejected
  files.upload_complete  returns `object_id` (NOT file_id)

Semantics that look like errors but aren't:
- entities.create dedup -> outcome `deduplicated`, SAME public_id
  (identity match — success). Use run-unique identity values in tests.
- relations.create duplicate active relation -> 409 (idempotent).
- files.download/documents.download return raw bytes/text in
  content[].text, not a JSON envelope.

Response-shape asymmetries: entities.list -> `data[]`;
entities.query -> `data.rows`.
Field values in entities.create: scalars as plain values, entity refs as
{"entity_id": "<uuid>"}. identity=true fields drive dedup identity
policies derived at publish.
"""),

    dict(kb_id="kb.schema.lifecycle", sort=80, category="schema",
         title="Schema draft lifecycle",
         summary="draft_create -> draft_update(if_match) -> validate -> "
                 "impact -> draft_get -> publish; retire only current.",
         tags="schema,draft,publish,etag",
         content="""Enforced order — skipping a step returns a tool error:
1. schema.draft_create {"name": "..."} — optional base_revision
   (revision id to branch from; omitted = inherit latest published).
2. schema.draft_get -> note `revision_id` (draft id) and `etag`.
3. schema.draft_update {"id", "display_name", "semantic_version",
   "types": [...], "predicates": [...], "changes": [...],
   "if_match": <etag>} — REPLACES the whole catalog; merge your changes
   into the arrays returned by draft_get when editing an existing schema.
4. schema.draft_validate {"id"} — structural validation.
5. schema.draft_impact {"id"} — impact analysis.
6. schema.draft_get again — EVERY write bumps the draft version; always
   re-fetch the etag before the next mutating call.
7. schema.draft_publish {"id", "reason", "if_match"} — step-up
   capability (schema.publish); publishes and auto-retires the previous
   published revision.

Type definition shape (draft_update `types` entry):
  {"type_id","stable_alias","entity_class":"data","label",
   "fields":[{"kind":"text|integer|decimal|date|...","alias",
              "label","identity":bool,"required":bool}]}
Predicate defs (`predicates` array):
  scalar: {"stable_alias","scalar_kind","label",
           "participates_in_identity":bool}
  edge:   {"stable_alias","is_edge":true,"domain":"<type_alias>",
           "range":"<type_alias>","label","inverse_label"}
ScalarKind values (crates/domain/src/scalar.rs): text, boolean,
integer, decimal, date, instant, uuid, uri, email, phone, money,
measurement, json, blob.

Retire semantics: schema.revision_retire works ONLY on the CURRENT
published revision — publish a replacement first. To restore a catalog:
draft(base_revision=old) -> validate -> impact -> publish.

Identity policies are derived from identity:true fields at publish
(apply_definitions) — without identity fields, dedup silently doesn't
run for that type.

Reference implementation: docs/testing/warehouse/provision_v2.py.
"""),

    dict(kb_id="kb.tests", sort=90, category="testing",
         title="Test and verification commands",
         summary="cargo fmt/clippy/check/test; web + nuxt vitest; "
                 "Playwright e2e; pg_scenarios; MCP suites 1241/1257.",
         tags="tests,cargo,vitest,playwright,e2e",
         content="""Lint / typecheck / unit tests (no DB needed):
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings   # see pitfall:
      # pre-existing cast_precision_loss failures in crates/risk-engine
  cargo check --workspace --all-targets
  cargo test --workspace     # ~355 Rust tests; db-runtime live-bridge test is #[ignore]

Web typed client (web/):
  cd web && npm run typecheck && npm test        # ~68 Vitest tests

Nuxt admin UI (nuxt/):
  cd nuxt && npm run build && npm test           # ~121 Vitest tests
  Playwright E2E (~69 specs in nuxt/e2e/, mocked API routes; persona
  matrix admin/security_officer/manager/risk_analyst + a11y + i18n).
  Graph check: node nuxt/e2e/graph_visual_check.mjs (logs in
  admin/enginer-demo, asserts zero bubble overlaps, writes screenshots).

Live-DB HTTP scenarios (docker stack required):
  ENGINER_TEST_DATABASE_URL=postgres://enginer_app:enginer_app@localhost:6543/enginer \\
    cargo test -p enginer-api --test pg_scenarios -- --test-threads=1
  (20 scenarios: auth+CSRF, where-AST query, identity dedup, revision
  conflicts, relation graph, full schema lifecycle, capability denial)

Live MCP campaigns (docker stack + .wh_tokens.json required):
  python docs/testing/mcp_test_runner.py            # general suite, ~1241 cases; --area J.requests etc.
  python docs/testing/warehouse/wh_test_runner.py   # warehouse domain, 1257 cases (A..S handwritten,
                                                    #  T deepening, X.* generated negatives)
  Reports/catalogs: docs/testing/mcp_test_report.md, mcp_report_raw.json,
  gen_case_catalog.py; warehouse equivalents under docs/testing/warehouse/
  (wh_test_cases.md, wh_test_report.md, wh_report_raw.json, PUML maps,
  wh_agent_prompts.md 100-prompt run, wh_agent_report.md).
"""),

    dict(kb_id="kb.frontend", sort=100, category="frontend",
         title="Nuxt 4 admin UI structure",
         summary="SPA on Vue 3 + Nuxt UI 4 + Pinia; pages map, 8 i18n "
                 "locales (ar/he RTL), metadata-driven components.",
         tags="nuxt,vue,i18n,pages,ui",
         content="""nuxt/ — Nuxt 4 SPA (production build served by
enginer-web:3000; nitro routeRules proxy API calls to
ENGINER_API_URL=http://enginer-api:8010).

Pages (nuxt/app/pages):
  login.vue (field `username`), index.vue, objects/index.vue +
  objects/[typeAlias]/ (list + create), entities/[publicId].vue (8 tabs:
  overview, relations, documents, history, checks, risks, tasks, ...),
  admin/ (index, schema, types, predicates, users, permissions),
  checks/, risks/, requests/, monitoring/, reports/, integrations/,
  audit/, docs/.

Key composables (nuxt/app/composables): useApi (bridge to
@enginer/api-client), useAuth (Pinia store, /me/context), useCapability
(UX guard), useSchemaMutation, useStagedUpload.

Metadata-driven components (nuxt/app/components/entity):
EntityFormRenderer (sections/collapsible/unsaved-changes guard),
FormField (registry for 22 field kinds), EntityAutocomplete (prefix
search), MultiValueField (cardinality), EntityListRenderer
(filters/facets/sort/bulk/row-actions), DuplicateSuggestionDialog
(exact+fuzzy match, identity modes, localStorage draft), RelationsTab,
DocumentsTab, HistoryTab, ChecksTab, RisksTab+RiskLevelBadge, TasksTab.
graph/: GraphView.vue + GraphTreeNode.vue (see kb.graph).

i18n (nuxt/i18n/locales): 8 locales — ru, en, ar (RTL), he (RTL), fr,
de, es, zh; schema.ts defines keys; each locale carries the same error
code keys; RTL handled via dir attribute switching. Keep locale files
in parity — a Vitest test asserts key parity across all 8.

UI caveats: Reka UI USelect/SelectItem option value must NOT be ''
(throws -> 500 error page); use null as the all/none sentinel.
"""),

    dict(kb_id="kb.graph", sort=110, category="frontend",
         title="GraphView force-bubble architecture",
         summary="d3-force simulation; positions Map + tick ref; pinned "
                 "nodes; BFS level columns; edge bowing/clustering.",
         tags="graphview,d3-force,visualization,nuxt",
         content="""nuxt/app/components/graph/GraphView.vue — SVG
force-directed bubble layout over d3-force v3, plus an accessible tree
alternative (GraphTreeNode.vue).

Forces: forceManyBody repulsion, forceLink springs (distance r1+r2+80),
forceCollide (r+14), weak forceX/forceY pulling toward BFS level columns
from the root (outgoing -> right, incoming -> left). Node radius scales
with visible degree; root is pinned.

CRITICAL rendering rule: simulation positions live in a plain
`positions` Map + a `tick` counter ref — rendering reads
layoutNodes/layoutEdges computed from that map; do NOT use node.x/y.
Never write a *seed* position into `positions` before rebuildSim — the
node gets pinned at the seed and cannot float (bug fixed once).

Expansion: rebuildSim(alpha, pinExisting) — on expandNode/expandCluster
existing nodes stay pinned, only newcomers float; user drag positions
persist in the `pinned` set across reheats. >20 equivalent relations
collapse into a `cluster:*` virtual node expandable via double-click;
progressive node expansion + cycle markers supported.

Edges: parallel edges between the same pair bow on quadratic curves;
labels sit at t~=0.35 from source and are hidden when >30 visible edges
(virtual cluster counts stay).

Visual check: node nuxt/e2e/graph_visual_check.mjs — logs in as
admin/enginer-demo, renders a dense graph, asserts zero bubble overlaps,
writes screenshots.

d3-force packaging gotcha: no ./package.json export — import the module;
don't require('d3-force/package.json') to check the version.
"""),

    dict(kb_id="kb.conventions", sort=120, category="conventions",
         title="Code conventions and documented deviations",
         summary="newtype ids, parameterized SQL only, BoxFuture ports, "
                 "default-deny, no unsafe, decimal-as-string, f64 ban.",
         tags="conventions,style,security,deviations",
         content="""Conventions (enforced workspace-wide):
- IDs are newtypes in enginer-domain::ids; never pass identifiers as
  interchangeable String.
- SQL built with sqlx QueryBuilder + bind parameters, only inside
  enginer-persistence; no format!-built SQL with user values. The query
  engine compiler emits parameterized SQL with $N placeholders — user
  literals never appear in SQL text (docs/07 §6, §14).
- Application ports return BoxFuture for object safety; use cases hold
  Arc<dyn Repository>.
- Security is default-deny; every controller/use case passes through
  PEP (CapabilityGuard: URL check first, then any-of capabilities, PDP
  fallback, fail-closed).
- RLS context (set_config) is set per-transaction and reset on return.
- `unsafe` forbidden workspace-wide.
- Decimal as string over the wire; f64 forbidden for
  money/shares/scores.
- AuditFairing appends a tamper-evident audit.event row (kind
  http_request) for every mutating /api/v1 call.

Documented deviations from docs/:
- sqlx is 0.7.4 (docs say 0.5.13): 0.5's transitive deps don't compile
  on Rust 1.95+; 0.7.4 keeps runtime-async-std-native-tls and the same
  async-std PgPool model. Single-point ADR — see root Cargo.toml header.
- Rocket route path params use &str (not uuid::Uuid): Rocket 0.5 has no
  FromParam for Uuid; handlers parse the string themselves.

Clock-skew tolerance: valid_from/valid_to comparisons add a small
window (now() + interval) in security_repo.rs, engine.rs, dataperm.rs,
authn.rs — the Postgres container clock drifts ~1s vs app clock;
strict <= now() made fresh rows intermittently invisible. New rows are
written with app-clock valid_from.
"""),

    dict(kb_id="kb.pitfalls", sort=130, category="pitfalls",
         title="Known pitfalls",
         summary="docker exec -i, `nul` file, d3-force import, USelect "
                 "'' value, required-field 422, identity derivation, "
                 "clippy risk-engine failures, 501 routes, clock skew.",
         tags="pitfalls,gotchas,windows,clippy",
         content="""- `docker exec` needs `-i`: without it, stdin/heredocs
  to psql are SILENTLY dropped — commands look like they ran but did
  nothing. Always: docker exec -i enginer-postgres psql ...
- Windows: `git add -A` dies if a file literally named `nul` exists at
  repo root — delete it first.
- d3-force has no ./package.json export — import the module normally;
  don't require('d3-force/package.json').
- Reka UI USelect/SelectItem: option value must not be '' (empty
  string) — throws and yields a 500 error page; use null.
- entities.create without a published-schema `required` field -> 422
  (enforced in pg_create_entity; regression covered in pg_scenarios).
- Identity policies are derived from identity:true fields at schema
  publish — custom types without identity fields silently skip dedup.
- entities.create dedup returns outcome=deduplicated with the SAME
  public_id — idempotent runners MUST use run-unique identity values
  (suffix a nonce) or they'll collide across runs.
- Pre-existing `cargo clippy` failures: cast_precision_loss lints in
  crates/risk-engine under Rust 1.95 — unrelated to feature work; don't
  chase them.
- Phase-5 API routes (connectors/checks/workflows/files/audit DTO
  surface) return 501 Not Implemented where persistence isn't wired —
  route structure/PEP gates/DTOs are the deliverable, not a bug.
  (Some areas later wired to PG — audit_pg, skr_pg, files_pg exist.)
- MCP revoked tokens may answer 200 + isError for ~30 s (warm cache).
- Postgres container clock drifts ~1s vs app — validity comparisons use
  a tolerance window (see kb.conventions).
- legacy-prototype/ is excluded from the workspace — don't build it or
  copy its format!-SQL patterns.
- MCP arg-name traps and response-shape asymmetries: see kb.mcp.gotchas.
"""),

    dict(kb_id="kb.docs.index", sort=140, category="reference",
         title="Index of docs/ normative specifications",
         summary="docs/00..27 + 99-references are the design source of "
                 "truth; docs/testing*/ hold MCP campaign artifacts.",
         tags="docs,specs,index,reference",
         content="""docs/ — normative specifications (design source of truth):
  00-vision-and-scope, 01-source-materials-and-requirements,
  02-system-architecture, 03-universal-domain-model,
  04-topology-unitarity-idempotency, 05-rust-workspace, 06-api-contract,
  07-query-engine, 08-security-architecture, 09-security-api,
  10-postgresql-schema, 11-performance-and-scaling, 12-schema-evolution,
  13-events-jobs-and-automation, 14-connectors-and-risk-engine,
  15-import-export-and-reporting, 16-nuxt-architecture,
  17-ui-screen-specification, 18-admin-console, 19-internationalization,
  20-threat-model, 21-observability-and-operations, 22-testing-and-quality,
  23-deployment-and-configuration, 24-current-code-migration,
  25-delivery-plan-and-acceptance, 26-decisions-and-tradeoffs,
  27-glossary, 99-references, README.md.

docs/testing/ — general MCP test campaign: mcp_test_runner.py (~1241
cases), mcp_test_cases.md, mcp_test_report.md, mcp_report_raw.json,
gen_case_catalog.py, PUML maps mcp_test_0{1,2,3}_*.puml.

docs/testing/warehouse/ — warehouse-domain campaign: bootstrap.py
(token provisioning), provision.py / provision_fresh.py /
provision_v2.py (schema+data loaders — provision_v2.py is the
reference for the draft->publish flow), wh_test_runner.py (1257 cases),
wh_test_cases.md, wh_test_report.md, wh_report_raw.json,
gen_wh_catalog.py, PUML maps wh_test_0{1,2,3}_*.puml,
wh_agent_prompts.md + wh_agent_report.md (100-prompt agent run),
mcp_probe.py (scratch MCP client — reuse its rpc_raw/call wrappers),
.wh_tokens.json (gitignored bearer tokens), this KB's kb_provision.py +
kb_write_report.md.
"""),

    dict(kb_id="kb.phases", sort=150, category="status",
         title="Implementation status snapshot",
         summary="Phases 0-3 complete; security/audit/workflow/risk/files "
                 "crates done; ~355 Rust + ~190 Vitest + ~69 e2e tests.",
         tags="status,phases,progress",
         content="""Per AGENTS.md (2026-10):
- Phase 0 Foundation: complete — workspace, CI, config, Rocket
  health/error, DbExecutor, PG migrations, observability, dev stack.
- Phase 1 Security/meta-kernel: complete — tenant/identity/session, two
  group classes (capability|data), PDP/PEP/RLS/audit, schema
  draft/publish.
- Phase 2 Entity graph: complete — Entity aggregate, IdentityPolicy/Key,
  Create/Retire/Merge use cases, repositories with RLS, query engine
  (AST + security rewrite + SQL compiler), entity routes, worker outbox.
- Phase 3 Nuxt core/admin: complete — @enginer/api-client (typed,
  401-refresh singleflight, If-Match/Idempotency-Key/CSRF) + Nuxt 4
  admin UI (entity list/card/create, schema admin, i18n x8,
  metadata-driven renderers, GraphView).
- Domain crates all landed: workflow (45 tests), risk-engine (31),
  files (23), reporting (25), connectors (14), integration
  CheckFlowOrchestrator (11), audit-persistence BLAKE2b chain (22),
  security PDP/ABAC/audit (51).
- Phase 5 API surface: 35 endpoints across schema_admin/connector/
  check/file routes — some still 501 where persistence isn't wired;
  audit_pg/skr_pg/files_pg are PG-backed. Authorization subsystem
  (sec.*) fully live with dataperm + CapabilityGuard.
- Test counts at last verification: ~355-364 Rust tests, 68 web Vitest,
  121 nuxt Vitest, ~69 Playwright E2E; MCP live suites ~1241 + 1257
  cases. legacy apps/api/tests/workspace_scenarios.rs is #[ignore]d
  (needs a DB-backed harness).
"""),
]

RELATIONS = [
    ("kb.onboarding", "kb.layout"),
    ("kb.onboarding", "kb.mcp.contract"),
    ("kb.onboarding", "kb.tests"),
    ("kb.stack.run", "kb.creds"),
    ("kb.stack.run", "kb.db.access"),
    ("kb.mcp.contract", "kb.mcp.gotchas"),
    ("kb.mcp.contract", "kb.schema.lifecycle"),
    ("kb.schema.lifecycle", "kb.db.state"),
    ("kb.schema.lifecycle", "kb.mcp.gotchas"),
    ("kb.frontend", "kb.graph"),
    ("kb.pitfalls", "kb.conventions"),
    ("kb.pitfalls", "kb.mcp.gotchas"),
    ("kb.docs.index", "kb.tests"),
]


def publish_schema():
    pub = call("schema.published")
    print("schema.published:", show(pub, 300))

    d = call("schema.draft_create", {"name": "kb-schema"})
    if failed(d):
        print("draft_create FAILED:", show(d)); sys.exit(1)
    draft = d.get("data") or {}
    did = str(draft.get("revision_id") or draft.get("id"))
    etag = draft.get("etag")
    print("draft:", did, "etag:", etag)

    types = list(draft.get("types") or [])
    preds = list(draft.get("predicates") or [])
    existing_t = {t.get("stable_alias") or t.get("type_id") for t in types}
    existing_p = {p.get("stable_alias") for p in preds}
    if "kb_article" not in existing_t:
        types.append(KB_TYPE)
    for p in KB_SCALARS + KB_EDGES:
        if p["stable_alias"] not in existing_p:
            preds.append(p)
    print(f"draft has {len(types)} types, {len(preds)} predicates")

    u = call("schema.draft_update", {
        "id": did, "display_name": "Project knowledge base",
        "semantic_version": "1.0.0",
        "description": "kb_article type + kb_related edge for the "
                       "project knowledge base",
        "types": types, "predicates": preds,
        "changes": [{"kind": "add", "resource_kind": "schema",
                     "resource_id": "kb-schema",
                     "summary": "Initial KB schema"}],
        "if_match": etag})
    if failed(u):
        print("draft_update FAILED:", show(u)); sys.exit(1)

    v = call("schema.draft_validate", {"id": did})
    print("validate:", show(v, 400))
    if failed(v):
        sys.exit(1)
    im = call("schema.draft_impact", {"id": did})
    print("impact:", show(im, 400))
    if failed(im):
        sys.exit(1)
    g = call("schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag
    p = call("schema.draft_publish", {
        "id": did, "reason": "Publish project knowledge-base schema",
        "if_match": etag})
    if failed(p):
        print("publish FAILED:", show(p)); sys.exit(1)
    pub_rev = (p.get("data") or {}).get("revision_id", did)
    print("PUBLISHED revision:", pub_rev)
    return str(pub_rev)


def create_articles():
    ids = {}
    for a in ARTICLES:
        fields = {
            "kb_id": a["kb_id"],
            "kb_title": a["title"],
            "kb_category": a["category"],
            "kb_summary": a["summary"],
            "kb_content": a["content"],
            "kb_sort": a["sort"],
            "kb_tags": a["tags"],
        }
        r = call("entities.create", {"type_id": "kb_article",
                                     "fields": fields})
        if failed(r) and isinstance(fields["kb_sort"], int):
            fields["kb_sort"] = str(fields["kb_sort"])  # retry as string
            r = call("entities.create", {"type_id": "kb_article",
                                         "fields": fields})
        if failed(r):
            print(f"CREATE FAILED {a['kb_id']}:", show(r)); continue
        data = r.get("data") or r
        pid = data.get("public_id") or data.get("id")
        outcome = data.get("outcome", "created")
        ids[a["kb_id"]] = pid
        print(f"{a['kb_id']:>22} -> {pid} ({outcome})")
    return ids


def create_relations(ids):
    ok = 0
    for subj, obj in RELATIONS:
        sid, oid = ids.get(subj), ids.get(obj)
        if not sid or not oid:
            print("skip relation", subj, "->", obj); continue
        r = call("relations.create", {
            "id": sid, "predicate": "kb_related",
            "target_entity_id": oid})
        if failed(r):
            print(f"REL FAILED {subj}->{obj}:", show(r, 300))
        else:
            ok += 1
    print(f"relations created: {ok}/{len(RELATIONS)}")


def verify(ids):
    print("\n== VERIFY ==")
    l = call("entities.list", {"type": "kb_article", "limit": 100})
    rows = (l.get("data") or [])
    print("entities.list count:", len(rows))
    q = call("entities.query", {"query": {"filter": {
        "field": "kb_category", "op": "eq", "value": "mcp"}}})
    qrows = ((q.get("data") or {}).get("rows")
             if isinstance(q.get("data"), dict) else q.get("data")) or []
    print("entities.query category=mcp:", len(qrows))
    pid = ids.get("kb.mcp.contract")
    if pid:
        g = call("entities.get", {"id": pid})
        print("entities.get kb.mcp.contract ok:", not failed(g))
        gr = call("entities.graph", {"id": pid, "direction": "both",
                                     "max_depth": 2})
        print("entities.graph:", show(gr, 500))
    return {"list_count": len(rows), "query_mcp_count": len(qrows)}


def main():
    rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "kb-provision", "version": "1"}})
    rpc("notifications/initialized")
    rev = publish_schema()
    ids = create_articles()
    create_relations(ids)
    stats = verify(ids)
    result = {"published_revision": rev, "ids": ids, "stats": stats}
    with open(os.path.join(HERE, ".kb_result.json"), "w") as f:
        json.dump(result, f, indent=1)
    print("\nDONE", json.dumps(result["stats"]))


if __name__ == "__main__":
    main()
