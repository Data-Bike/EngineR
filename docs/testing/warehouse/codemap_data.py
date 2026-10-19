"""Code-map data for EngineR: node + edge definitions.

Node tuple: (cm_id, kind, name, path, method, cap, summary, detail, sort, tags)
Edge tuple: (subject_id, predicate, object_id) — ids are cm_id or kb_id values;
the driver resolves them to public_ids.

kind vocabulary: repo app crate package module route tool table migration
                 page component composable middleware test config doc
edge vocabulary: cm_contains cm_depends cm_exposes cm_maps_to cm_calls
                 cm_uses cm_persists cm_defines cm_tests cm_implements
"""

NODES = []
EDGES = []


def N(cm_id, kind, name, path, summary, detail="", method="", cap="", sort=0,
      tags=""):
    NODES.append(dict(cm_id=cm_id, kind=kind, name=name, path=path,
                      summary=summary, detail=detail, method=method, cap=cap,
                      sort=sort, tags=tags))


def E(s, pred, o):
    EDGES.append((s, pred, o))


# =========================================================================
# Top level
# =========================================================================
N("cm.repo", "repo", "EngineR monorepo", ".",
  "Rust workspace (3 apps + 22 crates) + Nuxt 4 admin SPA + typed TS "
  "client + sqlx migrations + normative docs/. Root of the code map.",
  "Workspace members per Cargo.toml: apps/{api,worker,mcp} + 22 crates/*. "
  "legacy-prototype/ excluded. Postgres schemas core/meta/sec/ops/audit/"
  "integration/report. AGPL-3.0, rust-toolchain 1.95-ish.",
  sort=0, tags="root,workspace")

N("app.api", "app", "enginer-api", "apps/api",
  "Rocket 0.5 HTTP JSON API on :8010 — the only public surface; all PDP/PEP "
  "enforcement happens here. Mounts /health, /api/v1, /internal/security/v1.",
  "rocket_build (lib.rs): CorsFairing + AuditFairing (tamper-evident "
  "audit.event http_request rows for mutations + entity views), uniform "
  "catchers, CapabilityGuard PEP on every domain route. main.rs: "
  "observability init, ApiConfig, AsyncStdDbExecutor spawn, ENGINER_SEED_DEMO "
  "demo seed, local in-process SecurityClient (ADR-008) backed by the PG "
  "snapshot provider.",
  sort=10, tags="rocket,http,api,pep")

N("app.mcp", "app", "enginer-mcp", "apps/mcp",
  "MCP Streamable-HTTP server on :8020 — thin authenticated bridge exposing "
  "the HTTP API as ~95 JSON-RPC tools (protocol 2025-06-18).",
  "Bearer egr_* token validated via GET /me/context (30s cache); tools/list "
  "filtered by token capabilities; tools/call → prepare_call (path/query/"
  "body/if_match/data_base64, path-safety check) → reqwest forward with the "
  "same token — authoritative PDP stays API-side. All writes go through "
  "tools/call only.",
  sort=20, tags="mcp,jsonrpc,bridge")

N("app.worker", "app", "enginer-worker", "apps/worker",
  "Background worker: ops.outbox drain loop + monitor scheduler (docs/02 §7, "
  "ADR-014). No HTTP port.",
  "Tokio loop every 2s: enqueue due integration.monitor runs → claim up to "
  "10 ops.outbox rows FOR UPDATE SKIP LOCKED → per-event tenant RLS "
  "context → dispatch: check.execute → complete core.check_instance with "
  "worker evidence; monitor.run → advance integration.monitor schedule; "
  "unknown types logged+marked. Mark dispatched_at, attempts++.",
  sort=30, tags="worker,outbox,scheduler")

N("pkg.web-client", "package", "@enginer/api-client (web/)", "web",
  "Typed TypeScript API client package: ApiClient fetch wrapper + endpoint "
  "groups + DTO types mirroring Rust contracts.",
  "ApiClient: timeout via AbortController, X-Locale/X-Timezone/X-Request-Id, "
  "CSRF + If-Match + Idempotency-Key headers, 401 refresh singleflight, "
  "typed ApiClientError. 13 endpoint classes + types/ mirror of "
  "envelope/error/entity/query/schema/security/skr/view. ~68 Vitest tests.",
  sort=40, tags="typescript,client,fetch")

N("pkg.nuxt-ui", "package", "Nuxt 4 admin SPA (nuxt/)", "nuxt",
  "Production admin UI served by enginer-web:3000; Vue 3 + Nuxt UI 4 + "
  "Pinia + vue-i18n (8 locales, ar/he RTL).",
  "SPA; apiBaseUrl=/api/v1 same-origin (nitro routeRules proxy → "
  "ENGINER_API_URL). Auth: session cookie + /me/context Pinia store, route "
  "middleware, capability UX guards. Metadata-driven UI: views from "
  "/schemas/types/{id}/view drive forms/lists.",
  sort=50, tags="nuxt,vue,spa,pinia,i18n")

N("pkg.skr-core", "package", "SKR core schema bundle", "schema-packages/skr-core",
  "Signed JSON schema bundle: 15 domain types, ~60 predicates, 7 identity "
  "policies, 9 views, 4 workflows, Excel mapping profile, ru/en i18n.",
  "person, legal_entity, holding_view, address, contact_point, document, "
  "license, request, check, risk_factor, assessment, decision, task, "
  "contract, collateral. Loaded via crates/schema-registry/src/bundle.rs → "
  "SchemaDraft → validate → publish. 9 structural Rust tests.",
  sort=60, tags="bundle,skr,schema,seed-data")

N("cfg.compose", "config", "docker-compose stack", "docker-compose.yml",
  "postgres:6543, api:8010, mcp:8020, web:3000, worker (no port). "
  "ENGINER_SEED_DEMO=true, ENGINER_DEMO_PASSWORD=enginer-demo.",
  "api uses role enginer_app, worker uses enginer_worker; ENGINER_API_URL "
  "wires mcp→api and web(nitro)→api.", sort=70, tags="docker,compose,ops")

N("cfg.enginer-toml", "config", "enginer.*.toml + Rocket.toml",
  "config/ Rocket.toml",
  "ApiConfig files: enginer.docker.toml / enginer.local.toml "
  "(http/database/security sections); Rocket.toml carries the secret_key.",
  "Selected via ENGINER_CONFIG env; database url_secret_ref overridden by "
  "ENGINER_DATABASE_URL.", sort=71, tags="config,toml")

N("doc.agents", "doc", "AGENTS.md", "AGENTS.md",
  "Agent notes: layout, runbook, credentials, phase status — mirror of this "
  "code map in prose.", sort=72, tags="docs,agents")

N("ui.i18n", "module", "nuxt i18n locales", "nuxt/i18n",
  "8 locales ru/en/ar(RTL)/he(RTL)/fr/de/es/zh + schema.ts key registry; "
  "parity asserted by Vitest.",
  "Every locale carries the same error-code keys; RTL handled via dir "
  "attribute switching (docs/19).", sort=73, tags="i18n,locales,rtl")

# =========================================================================
# Crates
# =========================================================================
def CRATE(cm_id, dirn, name, summary, detail, specs, sort):
    N(cm_id, "crate", name, dirn, summary, detail, sort=sort,
      tags="crate,rust")
    for s in specs:
        E(cm_id, "cm_implements", s)


CRATE("crate.domain", "crates/domain", "enginer-domain",
  "Pure domain types, newtype ids and invariants (docs/03, 04). No Rocket/"
  "sqlx/Nuxt deps — serde/uuid/chrono/blake2 only.",
  "Entity aggregate: monotonic revision, no self-merge, retired rejects "
  "mutations except restore, merge→entity_alias+canonical_entity_id. "
  "Statement = subject-predicate-object unitary atom (edge or canonical "
  "scalar — exactly one). CanonicalScalar 14 kinds. IdentityPolicy/"
  "IdentityKey/compute_identity_key (collision: reject|return_existing|"
  "create_candidate). Newtype ids (TenantId/EntityId/PublicEntityId/"
  "TypeId/PredicateId/StatementId/...). EntityClass Data|Schema|Policy|"
  "Workflow|System. fact_fingerprint BLAKE2; ProvenanceRef/SourceRef; "
  "ValidPeriod/RecordedTime; TypeDefinition/PredicateDefinition/"
  "SchemaRevision meta-kernel types.",
  ["kb.docs.03", "kb.docs.04", "kb.docs.05"], 100)

CRATE("crate.contracts", "crates/contracts", "enginer-contracts",
  "Stable HTTP DTOs: ApiEnvelope/ApiMeta/ApiLinks, ApiError+ApiErrorCode "
  "wire codes (docs/06); Excel import pipeline (docs/15).",
  "import.rs: MappingProfile/ColumnMapping/CellParser (15 cell kinds: "
  "text/date/money/phone/email/integer/decimal/boolean/inn/ogrn/kpp/okved/"
  "opf/person_name), ConflictAction, EmptySemantics, ParsedRow/Cell, "
  "DryRunRowResult/Summary, PlannedStatement, RowOutcome; parse_cell "
  "normalizers (INN 10/12, OGRN 13/15, KPP 9, E.164 RU, DD.MM.YYYY→ISO). "
  "23 Rust tests.",
  ["kb.docs.06", "kb.docs.15"], 110)

CRATE("crate.application", "crates/application", "enginer-application",
  "Application layer: use cases + ports + request context (docs/02 §3, "
  "docs/05 §5).",
  "Ports (BoxFuture, object-safe): EntityRepository, StatementRepository, "
  "IdentityKeyRepository, IdentityPolicyRepository, SchemaRepository, "
  "Outbox, AuditSink, Clock, IdGenerator. use_cases: CreateEntity (resolve "
  "identity policies → compute keys → insert; unique-conflict → "
  "deduplicated — SELECT-before-INSERT forbidden), RetireEntity, "
  "MergeEntity. RequestContext/AuthStrength; ApplicationError; test_mocks.",
  ["kb.docs.02", "kb.docs.04"], 120)

CRATE("crate.observability", "crates/observability", "enginer-observability",
  "Tracing/metrics/audit helpers (docs/21): tracing_init::init, "
  "AuditEvent/AuditAction/AuditOutcome model.",
  "", ["kb.docs.21"], 130)

CRATE("crate.db-runtime", "crates/db-runtime-async-std", "enginer-db-runtime",
  "AsyncStdDbExecutor: async-std-owned PgPool bridged to Tokio/Rocket via "
  "bounded flume channel + oneshot (docs/02 §6, ADR-007).",
  "Invariants: pool created/closed in async-std runtime; bounded queue + "
  "cancellation; tx closure never leaves the runtime (owned results); job "
  "panic isolated; graceful shutdown joins; readiness false when DB down "
  "or queue saturated. Deadline, DbExecutorConfig, DbError.",
  ["kb.docs.02", "kb.docs.05"], 140)

CRATE("crate.persistence", "crates/persistence-postgres", "enginer-persistence",
  "PostgreSQL persistence adapter (docs/10): repositories + RLS context. "
  "sqlx parameterized only; no dynamic DDL (ADR-006).",
  "RlsContext.install_tx/install_conn/reset_conn — set_config "
  "app.tenant_id/principal_id/purpose/policy_revision; tx-local by default. "
  "PgEntityRepository, PgStatementRepository, PgIdentityKeyRepository, "
  "PgIdentityPolicyRepository, PgSchemaRepository, "
  "PgSecuritySnapshotProvider (PDP snapshot: memberships+policy rules).",
  ["kb.docs.10"], 150)

CRATE("crate.security-domain", "crates/security-domain",
  "enginer-security-domain",
  "Security domain model: principals, capabilities, data scopes, decisions "
  "(docs/08, 09).",
  "Capability/CapabilityAction wire strings; SecurityDecision + "
  "DecisionEffect + Obligations (QueryObligations/DataObligations/"
  "MutationObligations) + FieldMask/FieldPolicy/MandatoryFilter; "
  "PolicyRule/PolicyRuleEffect/ResourceSelector; Principal/GroupClass "
  "(capability|data)/PrincipalGroupMembership.",
  ["kb.docs.08", "kb.docs.09"], 160)

CRATE("crate.security-pdp", "crates/security-pdp", "enginer-security-pdp",
  "Policy Decision Point — pure evaluation engine (docs/08 §4, docs/09). "
  "Default deny (ADR-011), fail closed.",
  "PdpEngine::evaluate(snapshot, request): 1 verify tenant/session/auth "
  "strength → 2 collect memberships on policy snapshot → 3 capability → "
  "4 applicable data rules → 5 drop false-condition/expired → 6 "
  "deny-overrides → 7 allowed set + obligations → 8 constrain request → "
  "9 decision audit. PdpSnapshot, AuthorizationRequest/Resource, PdpError.",
  ["kb.docs.08", "kb.docs.09"], 170)

CRATE("crate.security-api", "crates/security-api", "enginer-security-api",
  "Security contract + LocalSecurityClient + internal Rocket controllers "
  "(docs/08 §1, docs/09, ADR-008).",
  "SecurityClient trait; LocalSecurityClient calls the PDP in-process (no "
  "self-HTTP) with a SnapshotProvider; contract DTOs: ActionDecisionRequest/"
  "Response, QueryDecisionRequest/Response, DecisionEffectDto, "
  "SecurityContextDto. Internal routes under /internal/security/v1 for "
  "mTLS peers.",
  ["kb.docs.08", "kb.docs.09"], 180)

CRATE("crate.schema-registry", "crates/schema-registry",
  "enginer-schema-registry",
  "Schema registry: draft → validate → publish lifecycle + signed bundle "
  "loader (docs/12, docs/03 §10). Published revisions immutable (ADR-004).",
  "SchemaDraft (mutable copy → frozen on publish, content hash), "
  "SchemaDraftError; validate.rs: ValidationReport/Finding/Severity — "
  "aliases/IDs, domain/range/cardinality, inheritance cycles, inverse/"
  "symmetric coherence, identity normalizers, Unicode confusables "
  "(docs/12 §2); bundle.rs: LoadedBundle → SchemaDraft, referential "
  "integrity (domain/range, predicate slots). 19 Rust tests.",
  ["kb.docs.12", "kb.docs.03"], 190)

CRATE("crate.identity", "crates/identity", "enginer-identity",
  "Identity normalizers + key material (docs/04 §6-7, §13). Idempotent: "
  "normalize(normalize(x)) == normalize(x).",
  "normalize_text (NFC+casefold), normalize_text_search_key, "
  "normalize_email, normalize_phone (E.164); NormalizedText/Email/Phone "
  "keep raw + canonical + normalizer version.",
  ["kb.docs.04"], 200)

CRATE("crate.query-engine", "crates/query-engine", "enginer-query-engine",
  "Query engine: AST + security rewrite + parameterized SQL compiler "
  "(docs/07). Pure functions; capability/PDP/execution are caller-side.",
  "ast.rs: QueryAst/WhereExpr/Operator/FieldRef/SelectField/SortKey/"
  "RelationDirection/SystemField. rewrite.rs SecurityRewriter: intersect "
  "select∩allowed_predicate_ids, drop denied types/predicates, inject "
  "mandatory_filters, reject sort/facet on hidden fields, clamp page.size, "
  "reject forbidden aggregates. compile.rs: SqlFragment ($N binds — user "
  "literals never in SQL text) over core.entity+core.statement.",
  ["kb.docs.07"], 210)

CRATE("crate.connectors", "crates/connectors", "enginer-connectors",
  "Connector SDK (docs/14): source-adapter trait, descriptors, instances, "
  "observations, rate limiting.",
  "Connector trait (descriptor/validate_credentials/health_check/search/"
  "fetch/monitor); ConnectorDescriptor/ConnectorKind (internal|api|"
  "file_import|internet|manual)/ConnectorCapabilities/LegalMetadata; "
  "ConnectorInstance(+Config: env, base_url, proxy, secrets, rps, "
  "concurrency, timeout, retry, cache, legal basis, trust); TrustRank "
  "0-100; SourceObservation/ProvenanceMeta/MappingResult/"
  "IdentityResolution; RateLimiter token bucket; CircuitBreaker "
  "closed/open/half-open; backoff_delay. 14 Rust tests.",
  ["kb.docs.14"], 220)

CRATE("crate.risk-engine", "crates/risk-engine", "enginer-risk-engine",
  "Risk engine: criteria DSL AST + factors + scores + assessment + "
  "decision (docs/14 §8-12). AST is data, evaluated by the engine.",
  "Expr/ExprKind 18 node kinds + evaluate() w/ EvaluationContext; Criterion "
  "(severity/weight/evidence_selector/valid_interval/tests); Factor "
  "(dedup_key, Active/Resolved/Suppressed/Overridden/Expired); "
  "WeightedAggregator+ScoreConfig; RiskLevel 0-100; Decision "
  "(Approve/Reject/Escalate/Defer/ConditionalApprove, requires_approval "
  "High/Critical); Assessment aggregate (Draft/Approved/Superseded, "
  "watermark/fingerprint, uncertainty). 31 Rust tests.",
  ["kb.docs.14"], 230)

CRATE("crate.workflow", "crates/workflow", "enginer-workflow",
  "Declarative state machine + job lifecycle + tasks + timers + reactions "
  "(docs/13).",
  "StateMachineDefinition/State/StateTransition/TransitionGuard/"
  "StateMachine.transition (validates, records history, terminal). Job/"
  "JobState queued→leased→running→waiting_external|waiting_user→succeeded|"
  "partially_succeeded|failed|cancelled + cooperative cancel. Task/"
  "TaskStatus/TaskPriority/AssignmentStrategy. Schedule/ScheduleConfig/"
  "MisfirePolicy/EscalationTimer/TimerKind. ReactionConfig/"
  "ReactionTriggerKind/ActionKind/Reaction (cooldown, dedup, idempotency, "
  "compensation, requires_approval). 45 Rust tests.",
  ["kb.docs.13"], 240)

CRATE("crate.files", "crates/files", "enginer-files",
  "Staged upload protocol + file scanning + object-storage abstraction "
  "(docs/15 §1-3).",
  "StagedUpload (UploadInit/Chunk/Complete/Abort; idempotent chunk "
  "re-upload, checksum+size verify, progress). FileScanResult/"
  "FileScanStatus/FileSignature/ScanFinding/FileScanner trait "
  "(detect_format, scan: size/zip-bomb/formula/macro/links/MIME/AV). "
  "ObjectMetadata/StorageClass/ObjectStorage trait (store/retrieve/"
  "delete/exists/head; TTL, legal hold). 23 Rust tests.",
  ["kb.docs.15"], 250)

CRATE("crate.reporting", "crates/reporting", "enginer-reporting",
  "Datasets + report definitions + templates + render (docs/15 §8-11).",
  "Dataset/DatasetColumn/DatasetQuery/DatasetStatus/DatasetSnapshot "
  "(pinned schema+policy revisions, content hash). ReportDefinition "
  "(sections/tables/charts/summaries, locale/tz, allowed formats/"
  "audiences, watermark, requires_snapshot, schedule). TemplateDefinition/"
  "TemplateVersion/TemplateLocale/FieldFormat. render: TemplateRenderer, "
  "RenderContext/Output, escape_formula_injection. 25 Rust tests.",
  ["kb.docs.15"], 260)

CRATE("crate.security", "crates/security", "enginer-security",
  "Alternative self-contained security stack (docs/08): ABAC rules, "
  "capabilities, classification, local PDP, tamper chain.",
  "abac.rs: DataRule/DataPermission/ResourceSelector/Effect; "
  "capability.rs: Capability/CapabilityGroup/CapabilityGroupKind/"
  "CapabilityKind; classification.rs: ClassificationLevel/FieldPolicy/"
  "FieldPolicyKind/MaskStrategy; pdp.rs: PdpClient/LocalPdpClient/"
  "PolicySnapshot/DecisionContext/Decision; audit.rs: AuditEntry/"
  "AuditKind/TamperChain. 51 Rust tests.",
  ["kb.docs.08"], 270)

CRATE("crate.audit-persistence", "crates/audit-persistence",
  "enginer-audit-persistence",
  "Audit repository + BLAKE2b tamper-evident hash chain (docs/08 §11,15). "
  "Insert-only from app role; separate retention role.",
  "ChainHasher/ChainState: each entry hash includes previous hash; "
  "checkpoints persist latest hash; verify recomputes end-to-end. "
  "AuditRepository trait + PgAuditRepository (audit.event, "
  "audit.chain_checkpoint, audit.retention_policy). 22 Rust tests.",
  ["kb.docs.08"], 280)

CRATE("crate.integration", "crates/integration", "enginer-integration",
  "End-to-end check-flow orchestrator wiring Phase-5 crates "
  "(docs/13 §8, docs/14 §4-12).",
  "CheckFlowOrchestrator: check request → connector fetch → mapping/"
  "provenance statements → criteria evaluation → factors → score → "
  "assessment → decision → workflow instance → tasks → reactions. Ports: "
  "Connector/Criterion/Factor/Assessment/Decision/Workflow/Task/Audit. "
  "11 Rust tests.",
  ["kb.docs.13", "kb.docs.14"], 290)

CRATE("crate.test-support", "crates/test-support", "enginer-test-support",
  "Cross-crate invariant tests (docs/05 §3, docs/22): security pipeline, "
  "schema registry, identity normalizers end-to-end.",
  "", ["kb.docs.05", "kb.docs.22"], 300)

# =========================================================================
# Crate dependency edges (from Cargo.toml manifests)
# =========================================================================
for s, o in [
    ("app.api", "crate.application"), ("app.api", "crate.audit-persistence"),
    ("app.api", "crate.connectors"), ("app.api", "crate.contracts"),
    ("app.api", "crate.db-runtime"), ("app.api", "crate.domain"),
    ("app.api", "crate.files"), ("app.api", "crate.identity"),
    ("app.api", "crate.observability"), ("app.api", "crate.persistence"),
    ("app.api", "crate.query-engine"), ("app.api", "crate.reporting"),
    ("app.api", "crate.risk-engine"), ("app.api", "crate.schema-registry"),
    ("app.api", "crate.security"), ("app.api", "crate.security-api"),
    ("app.api", "crate.security-domain"), ("app.api", "crate.security-pdp"),
    ("app.api", "crate.workflow"),
    ("app.mcp", "crate.observability"),
    ("app.worker", "crate.application"), ("app.worker", "crate.db-runtime"),
    ("app.worker", "crate.domain"), ("app.worker", "crate.observability"),
    ("app.worker", "crate.persistence"), ("app.worker", "crate.security-domain"),
    ("crate.contracts", "crate.domain"),
    ("crate.application", "crate.contracts"), ("crate.application", "crate.domain"),
    ("crate.application", "crate.observability"),
    ("crate.observability", "crate.domain"),
    ("crate.db-runtime", "crate.domain"), ("crate.db-runtime", "crate.observability"),
    ("crate.persistence", "crate.application"), ("crate.persistence", "crate.db-runtime"),
    ("crate.persistence", "crate.domain"), ("crate.persistence", "crate.observability"),
    ("crate.persistence", "crate.security-api"), ("crate.persistence", "crate.security-domain"),
    ("crate.persistence", "crate.security-pdp"),
    ("crate.security-domain", "crate.domain"),
    ("crate.security-pdp", "crate.domain"), ("crate.security-pdp", "crate.security-domain"),
    ("crate.security-api", "crate.contracts"), ("crate.security-api", "crate.domain"),
    ("crate.security-api", "crate.security-domain"), ("crate.security-api", "crate.security-pdp"),
    ("crate.schema-registry", "crate.application"), ("crate.schema-registry", "crate.domain"),
    ("crate.identity", "crate.domain"),
    ("crate.query-engine", "crate.domain"), ("crate.query-engine", "crate.security-domain"),
    ("crate.connectors", "crate.contracts"), ("crate.connectors", "crate.domain"),
    ("crate.risk-engine", "crate.domain"),
    ("crate.workflow", "crate.domain"),
    ("crate.files", "crate.domain"),
    ("crate.reporting", "crate.domain"),
    ("crate.security", "crate.domain"),
    ("crate.audit-persistence", "crate.db-runtime"),
    ("crate.audit-persistence", "crate.observability"),
    ("crate.audit-persistence", "crate.security"),
    ("crate.integration", "crate.connectors"), ("crate.integration", "crate.contracts"),
    ("crate.integration", "crate.domain"), ("crate.integration", "crate.files"),
    ("crate.integration", "crate.reporting"), ("crate.integration", "crate.risk-engine"),
    ("crate.integration", "crate.security"), ("crate.integration", "crate.workflow"),
    ("crate.test-support", "crate.domain"), ("crate.test-support", "crate.identity"),
    ("crate.test-support", "crate.schema-registry"), ("crate.test-support", "crate.security-api"),
    ("crate.test-support", "crate.security-domain"), ("crate.test-support", "crate.security-pdp"),
    ("pkg.nuxt-ui", "pkg.web-client"),
]:
    E(s, "cm_depends", o)


# =========================================================================
# App modules
# =========================================================================
def MOD(cm_id, name, path, summary, detail="", parent=None, tables=(),
        specs=(), tags="module,rust", sort=0):
    N(cm_id, "module", name, path, summary, detail, sort=sort, tags=tags)
    if parent:
        E(parent, "cm_contains", cm_id)
    for t in tables:
        E(cm_id, "cm_persists", "table." + t)
    for s in specs:
        E(cm_id, "cm_implements", s)


# --- apps/api --------------------------------------------------------------
MOD("mod.api.lib", "enginer_api lib.rs", "apps/api/src/lib.rs",
    "rocket_build: mounts /health, /api/v1 (auth, schema admin, entity, "
    "audit, skr, files, users + /version + OPTIONS preflight), "
    "/internal/security/v1 when security_client present; CorsFairing + "
    "AuditFairing; uniform JSON error catchers.",
    "AuditFairing: after mutating /api/v1 calls (and GET /entities/{uuid} "
    "views) appends audit.event kind=http_request attributed to the "
    "resolved principal. Catchers 400/401/403/404/422/500 → "
    "{error:{code,message_key,request_id}} envelope.",
    parent="app.api", specs=("kb.docs.02", "kb.docs.06"))

MOD("mod.api.main", "enginer-api main.rs", "apps/api/src/main.rs",
    "Entrypoint: observability init → ApiConfig load → AsyncStdDbExecutor "
    "spawn → optional demo seed → in-process SecurityClient (PDP + PG "
    "snapshot provider via scoped thread) → rocket_build().launch().",
    "", parent="app.api")

MOD("mod.api.config", "ApiConfig", "apps/api/src/config.rs",
    "Typed config (docs/23 §4): http{public_origin,request_timeout_ms,"
    "max_json_bytes}, database{url_secret_ref,max_connections,"
    "acquire_timeout_ms,statement_timeout_ms,db_executor_queue}, "
    "security{policy_cache_ttl_seconds,fail_closed,internal_mtls_identity}.",
    "", parent="app.api")

MOD("mod.api.cors", "CorsFairing", "apps/api/src/cors.rs",
    "Permissive dev CORS (docs/16 §13): echoes Origin, allows credentials, "
    "OPTIONS → 204. Production CORS is on the reverse proxy.",
    "", parent="app.api")

MOD("mod.api.state", "AppState", "apps/api/src/state.rs",
    "Shared state: config + Option<AsyncStdDbExecutor> + "
    "Option<SecurityClient>.", "", parent="app.api")

MOD("mod.api.authn", "authn.rs — session/token resolution", "apps/api/src/authn.rs",
    "Session auth + authz plumbing (docs/08 §2-4, docs/09): AuthContext, "
    "find_credential (argon2id PHC in sec.principal_credential, pre-auth "
    "outside RLS), resolve_session/resolve_bearer (sec.session / "
    "sec.api_token), decide_action → SecurityClient PDP, begin_rls.",
    "PrincipalProfile for /me/context (capabilities, policy_revision, "
    "ui_rules). Bearer egr_* wins over cookie; clock-skew tolerance window "
    "on validity checks.",
    parent="app.api",
    tables=("sec.principal_credential", "sec.session", "sec.api_token",
            "sec.principal", "sec.policy_revision", "sec.policy_rule",
            "sec.principal_group_membership"),
    specs=("kb.docs.08", "kb.docs.09"))

MOD("mod.api.auth_routes", "auth_routes.rs — /auth/* + /me/context",
    "apps/api/src/auth_routes.rs",
    "POST /auth/login (login+password → argon2 verify → create sec.session "
    "with csrf_token, audits login), POST /auth/logout, GET /me/context "
    "(profile+capabilities+policy_revision+ui_rules), POST /auth/refresh.",
    "", parent="app.api",
    tables=("sec.principal_credential", "sec.session"))

MOD("mod.api.dataperm", "dataperm.rs — data-scope PEP", "apps/api/src/dataperm.rs",
    "DataPerm: compiles sec.policy_rule rows of effective groups "
    "(memberships + ancestors via parent_id) into a decision set; special "
    "roles superadmin/superuser override; can_url + can_capability "
    "entry points used by CapabilityGuard.",
    "Action grammar: data.<type|field|entity|relation|relation_instance|"
    "all>.<read|create|update|delete|*>, system.url (resource.path prefix), "
    "data.* legacy wildcard. Defaults: type/field/relation DENY; entity/"
    "relation-instance ALLOW unless denied; URL open unless blanket deny "
    "→ then needs allow rule. Deny always overrides.",
    parent="app.api",
    tables=("sec.policy_rule", "sec.principal_group_membership"),
    specs=("kb.docs.08",))

MOD("mod.api.workspace_routes", "workspace_routes.rs — PEP + wire records",
    "apps/api/src/workspace_routes.rs",
    "CapabilityGuard FromRequest PEP: resolves Bearer-token OR session "
    "cookie → CSRF echo on cookie mutations → DataPerm.can_url → "
    "required_capabilities(method,path) any-of via can_capability then PDP "
    "decide_action fallback → deny audits 'denied_request'. Plus "
    "ApiEnvelope + wire records (Entity/Relation/History/Graph/Query/"
    "SchemaType) + build_type_view.",
    "required_capabilities: central method+path → capability map "
    "(entity.*, schema.draft/publish, user.* +users.manage umbrella, "
    "check.view/run, risk.view/resolve, assessment.view, report.view/run/"
    "export, monitor.manage, connector.manage, audit.read, security.manage, "
    "admin.pages.access, query.advanced).",
    parent="app.api", specs=("kb.docs.08",))

MOD("mod.api.entity_pg", "entity_pg.rs — entity workspace routes",
    "apps/api/src/entity_pg.rs",
    "PG-backed entity surface: list/search/get/create/update/retire/merge, "
    "relations, history, graph BFS, POST /entities/query AST. Every handler "
    "runs in an RLS-scoped tx via rls_tx; results cross the executor "
    "boundary as serde_json::Value.",
    "object_to_wire CanonicalScalar→JSON; identity dedup → outcome "
    "deduplicated with same public_id; revision optimistic concurrency "
    "(409 on mismatch); graph builds nodes/edges with cycle markers.",
    parent="app.api",
    tables=("core.entity", "core.entity_alias", "core.entity_revision",
            "core.statement", "core.identity_key", "meta.identity_policy",
            "meta.localized_text", "meta.predicate_definition",
            "meta.schema_revision", "meta.type_definition", "ops.outbox",
            "sec.principal"),
    specs=("kb.docs.04", "kb.docs.10"))

MOD("mod.api.schema_admin_routes", "schema_admin_routes.rs — schema lifecycle",
    "apps/api/src/schema_admin_routes.rs",
    "Draft → validate → impact → publish → retire endpoints + published "
    "catalog reads (types/predicates/type views). If-Match etag "
    "concurrency on draft_update/publish.",
    "draft_update replaces whole catalog arrays; publish freezes revision "
    "and derives identity policies; retire only the current published "
    "revision.",
    parent="app.api",
    tables=("core.entity", "meta.identity_policy", "meta.localized_text",
            "meta.predicate_definition", "meta.schema_revision",
            "meta.type_definition"),
    specs=("kb.docs.12",))

MOD("mod.api.skr_pg", "skr_pg.rs — SKR operational routes",
    "apps/api/src/skr_pg.rs",
    "Checks, assessments, factors, tasks, workflows, requests, monitors, "
    "connectors, dashboard — all PG-backed under RLS.",
    "POST /checks also enqueues ops.outbox 'check.execute' for the worker; "
    "monitors.schedule ↔ integration.monitor; connectors → "
    "integration.connector; dashboard/stats aggregates core.task/request/"
    "check_instance/risk_factor.",
    parent="app.api",
    tables=("core.check_instance", "core.entity", "core.report_instance",
            "core.request", "core.risk_assessment", "core.risk_factor",
            "core.task", "core.workflow_instance", "integration.connector",
            "integration.monitor", "meta.type_definition",
            "meta.workflow_definition", "ops.outbox",
            "sec.principal_credential"),
    specs=("kb.docs.13", "kb.docs.14"))

MOD("mod.api.files_pg", "files_pg.rs — files/uploads/documents/reports",
    "apps/api/src/files_pg.rs",
    "Staged uploads (init/chunk octet-stream/complete/abort/status), file "
    "objects (meta/download/delete), entity documents, report instances. "
    "Blob bytes in core.file_object.content (bytea); chunks persisted in "
    "core.upload_chunk. SHA-256/BLAKE2b verification on complete.",
    "", parent="app.api",
    tables=("core.entity", "core.entity_document", "core.file_object",
            "core.report_instance", "core.upload_chunk",
            "core.upload_session", "sec.principal_credential"),
    specs=("kb.docs.15",))

MOD("mod.api.audit_pg", "audit_pg.rs — audit + permission groups",
    "apps/api/src/audit_pg.rs",
    "GET /audit filter+cursor, checkpoint get/create, POST /audit/verify "
    "(recompute BLAKE2b chain via enginer_audit_persistence::ChainHasher), "
    "retention get/put, /permissions/groups CRUD; record_audit_event "
    "helper used by guard + fairing.",
    "", parent="app.api",
    tables=("audit.event", "audit.chain_checkpoint",
            "audit.retention_policy", "sec.group",
            "sec.principal_group_membership", "sec.policy_revision",
            "sec.policy_rule", "sec.principal_credential"),
    specs=("kb.docs.08",))

MOD("mod.api.users_pg", "users_pg.rs — user administration",
    "apps/api/src/users_pg.rs",
    "User CRUD + activity feed + per-user API tokens (egr_*). Group "
    "resolution helpers for effective/inherited membership.",
    "Activity feed merges core.entity.created_by, core.entity_revision and "
    "audit.event. Token create returns the raw token once (hash stored).",
    parent="app.api",
    tables=("audit.event", "core.entity", "core.entity_revision",
            "meta.type_definition", "sec.api_token", "sec.principal",
            "sec.principal_credential", "sec.principal_group_membership",
            "sec.session"))

MOD("mod.api.seed", "seed.rs — demo dataset", "apps/api/src/seed.rs",
    "ENGINER_SEED_DEMO=true → idempotent seed: demo tenant, personas "
    "(admin/security_officer/manager/risk_analyst), Argon2 credentials, "
    "capability+data groups, published policy, demo entities and ops rows.",
    "Passwords: ENGINER_DEMO_PASSWORD shared or "
    "ENGINER_DEMO_PASSWORD_<LOGIN> per user; default enginer-demo.",
    parent="app.api",
    tables=("core.tenant", "sec.principal", "sec.principal_credential",
            "sec.group", "sec.principal_group_membership",
            "sec.policy_revision", "sec.policy_rule", "core.entity",
            "core.statement", "core.identity_key", "core.entity_revision",
            "core.check_instance", "core.risk_factor",
            "core.risk_assessment", "core.task", "core.workflow_instance",
            "core.request", "core.file_object", "core.report_instance",
            "integration.connector", "integration.monitor",
            "meta.schema_revision", "meta.type_definition",
            "meta.predicate_definition", "meta.identity_policy",
            "meta.localized_text", "meta.workflow_definition"),
    specs=("kb.docs.18",))

MOD("mod.api.health", "health.rs — probes", "apps/api/src/health.rs",
    "GET /health/live + GET /health/ready (unauthenticated).",
    "", parent="app.api")

# --- apps/mcp ---------------------------------------------------------------
MOD("mod.mcp.main", "enginer-mcp main.rs", "apps/mcp/src/main.rs",
    "Rocket routes: POST /mcp (JSON-RPC single or batch), GET /mcp → 405, "
    "DELETE /mcp → 200 (stateless), GET /health. Bearer request guard "
    "extracts the token; validation delegated to the API.",
    "", parent="app.mcp")

MOD("mod.mcp.lib", "enginer-mcp lib.rs — RPC + forwarding",
    "apps/mcp/src/lib.rs",
    "McpState{api_base, reqwest::Client, token→context 30s cache}; "
    "validate_token via GET /me/context; handle_rpc dispatch "
    "(initialize/ping/tools/list/tools/call/notifications); call_tool + "
    "pure prepare_call.",
    "prepare_call: path args reject '/', '..', '?', '#', empty; "
    "data_base64 → raw octet-stream body; if_match → If-Match header; "
    "entities.update fields{alias:value} → statement array; missing "
    "required/unknown args → tool error. Capability pre-check → JSON-RPC "
    "-32602; upstream API errors → result.isError.",
    parent="app.mcp", specs=("kb.docs.06",))

MOD("mod.mcp.tools", "tools.rs — tool catalog", "apps/mcp/src/tools.rs",
    "Declarative TOOLS: ~95 ToolDef{name,desc,method,path-template,"
    "capability,params[]} with Param{name,loc:Path|Query|Body,ty,required,"
    "desc}; tool_descriptor → inputSchema with x-location; "
    "tools_for_capabilities filters by token caps (users.manage umbrella "
    "for user.*).",
    "", parent="app.mcp")

# --- apps/worker -------------------------------------------------------------
MOD("mod.worker.main", "worker main.rs — outbox drain", "apps/worker/src/main.rs",
    "2s loop: schedule due monitors → claim ≤10 ops.outbox FOR UPDATE "
    "SKIP LOCKED → per-event set_config tenant → dispatch → mark "
    "dispatched_at.",
    "dispatch(): check.execute → complete core.check_instance w/ worker "
    "evidence; monitor.run → advance integration.monitor by frequency "
    "(hourly/weekly/monthly/daily) + last_result JSON; unknown types "
    "logged and marked dispatched.",
    parent="app.worker",
    tables=("ops.outbox", "core.check_instance", "integration.monitor"),
    specs=("kb.docs.02", "kb.docs.13"))


# =========================================================================
# Crate modules (tiny error.rs files are folded into their crate node)
# =========================================================================
def CMOD(cm_id, name, path, summary, parent, tables=(), specs=(), sort=0):
    N(cm_id, "module", name, path, summary, sort=sort, tags="module,rust")
    E(parent, "cm_contains", cm_id)
    for t in tables:
        E(cm_id, "cm_persists", "table." + t)
    for s in specs:
        E(cm_id, "cm_implements", s)


CMOD("mod.domain.entity", "domain/entity.rs", "crates/domain/src/entity.rs",
     "Entity aggregate root: status Active/Retired, revision monotonicity, "
     "no self-merge, merge moves loser ids → entity_alias + "
     "canonical_entity_id (idempotent).", "crate.domain")
CMOD("mod.domain.identity", "domain/identity.rs", "crates/domain/src/identity.rs",
     "IdentityPolicy (scope, key predicate components, normalizer, "
     "CollisionAction, ResurrectionPolicy) + IdentityKey + "
     "compute_identity_key; unique index + ON CONFLICT resolves races.",
     "crate.domain")
CMOD("mod.domain.statement", "domain/statement.rs", "crates/domain/src/statement.rs",
     "Statement subject-predicate→object; StatementObject exactly one of "
     "Entity|Scalar (manual serde); status/supersede semantics.",
     "crate.domain")
CMOD("mod.domain.scalar", "domain/scalar.rs", "crates/domain/src/scalar.rs",
     "CanonicalScalar + ScalarKind: text, boolean, integer, decimal, date, "
     "instant, uuid, uri, email, phone, money, measurement, json, blob — "
     "canonical NFC/normalized forms.", "crate.domain")
CMOD("mod.domain.schema", "domain/schema.rs", "crates/domain/src/schema.rs",
     "Meta-kernel: TypeDefinition/PredicateDefinition/SchemaRevision(state "
     "draft|published|retired), EntityClass; stable aliases — renames never "
     "migrate facts (ADR-003); revisions immutable (ADR-004).", "crate.domain")
CMOD("mod.domain.ids", "domain/ids.rs", "crates/domain/src/ids.rs",
     "Newtype ids: TenantId/EntityId/PublicEntityId/TypeId/PredicateId/"
     "StatementId/PrincipalId/SchemaRevisionId/DecisionId/ContentHash...",
     "crate.domain")
CMOD("mod.domain.time", "domain/time.rs", "crates/domain/src/time.rs",
     "ValidPeriod (valid_from/valid_to bitemporal) + RecordedTime.",
     "crate.domain")
CMOD("mod.domain.fingerprint", "domain/fingerprint.rs", "crates/domain/src/fingerprint.rs",
     "fact_fingerprint — BLAKE2 canonical hash of a fact tuple (tenant, "
     "subject, predicate, object, valid_from) for dedup/audit.",
     "crate.domain")
CMOD("mod.domain.provenance", "domain/provenance.rs", "crates/domain/src/provenance.rs",
     "ProvenanceRef/SourceRef — statement origin tracking.", "crate.domain")

CMOD("mod.contracts.envelope", "contracts/envelope.rs", "crates/contracts/src/envelope.rs",
     "ApiEnvelope{data,meta,links}, ApiMeta, ApiLinks — stable wire shape.",
     "crate.contracts")
CMOD("mod.contracts.error", "contracts/error.rs", "crates/contracts/src/error.rs",
     "ApiError + ApiErrorCode — stable error codes mapped to "
     "errors.* i18n keys.", "crate.contracts")
CMOD("mod.contracts.import", "contracts/import.rs", "crates/contracts/src/import.rs",
     "Excel import pipeline: MappingProfile/ColumnMapping/CellParser(15 "
     "kinds)/ConflictAction/EmptySemantics/ParsedRow/PlannedStatement/"
     "DryRun* — parse_cell normalizers.", "crate.contracts",
     specs=("kb.docs.15",))

CMOD("mod.application.context", "application/context.rs", "crates/application/src/context.rs",
     "RequestContext (tenant/principal/session/auth_strength/"
     "groups_revision) + AuthStrength.", "crate.application")
CMOD("mod.application.ports", "application/ports.rs", "crates/application/src/ports.rs",
     "Repository/service ports (BoxFuture, Arc<dyn>): EntityRepository, "
     "StatementRepository, IdentityKeyRepository, "
     "IdentityPolicyRepository, SchemaRepository, Outbox, AuditSink, "
     "Clock, IdGenerator.", "crate.application")
CMOD("mod.application.usecases", "application/use_cases/*", "crates/application/src/use_cases",
     "CreateEntity (identity dedup flow), RetireEntity, MergeEntity — "
     "atomic entity+statements+identity-keys; outbox events.",
     "crate.application", specs=("kb.docs.04",))
CMOD("mod.application.mocks", "application/test_mocks.rs", "crates/application/src/test_mocks.rs",
     "In-memory port mocks used by use-case unit tests.", "crate.application")

CMOD("mod.obs.audit", "observability/audit.rs", "crates/observability/src/audit.rs",
     "AuditEvent/AuditAction/AuditOutcome — structured audit record model.",
     "crate.observability")
CMOD("mod.obs.tracing", "observability/tracing_init.rs", "crates/observability/src/tracing_init.rs",
     "init(): structured tracing subscriber setup (RUST_LOG-aware).",
     "crate.observability")

CMOD("mod.dbruntime.executor", "db-runtime lib.rs", "crates/db-runtime-async-std/src/lib.rs",
     "AsyncStdDbExecutor.spawn/run + DbExecutorConfig + Deadline + DbError "
     "— flume queue to async-std-owned PgPool.", "crate.db-runtime")

CMOD("mod.pers.entity_repo", "persistence/entity_repo.rs", "crates/persistence-postgres/src/entity_repo.rs",
     "PgEntityRepository/PgIdentityKeyRepository/PgIdentityPolicyRepository "
     "— entity + alias + revision rows, identity-key insert/dedup.",
     "crate.persistence",
     tables=("core.entity", "core.entity_alias", "core.identity_key",
             "meta.identity_policy"))
CMOD("mod.pers.statement_repo", "persistence/statement_repo.rs", "crates/persistence-postgres/src/statement_repo.rs",
     "PgStatementRepository — append-only statement writes, "
     "supersede/validity windows.", "crate.persistence",
     tables=("core.statement",))
CMOD("mod.pers.schema_repo", "persistence/schema_repo.rs", "crates/persistence-postgres/src/schema_repo.rs",
     "PgSchemaRepository — meta.schema_revision read/persist.",
     "crate.persistence", tables=("meta.schema_revision",))
CMOD("mod.pers.security_repo", "persistence/security_repo.rs", "crates/persistence-postgres/src/security_repo.rs",
     "PgSecuritySnapshotProvider — PDP snapshot (memberships + policy "
     "rules + revision).", "crate.persistence",
     tables=("sec.policy_revision", "sec.policy_rule",
             "sec.principal_group_membership"))
CMOD("mod.pers.rls", "persistence/rls.rs", "crates/persistence-postgres/src/rls.rs",
     "RlsContext: install_tx (tx-local set_config) / install_conn + "
     "reset_conn for session-level; policies read only app.* settings.",
     "crate.persistence", specs=("kb.docs.10",))

CMOD("mod.secdom.capabilities", "security-domain/capabilities.rs", "crates/security-domain/src/capabilities.rs",
     "Capability + CapabilityAction wire strings (entity.read, "
     "schema.publish, users.manage, ...).", "crate.security-domain")
CMOD("mod.secdom.decision", "security-domain/decision.rs", "crates/security-domain/src/decision.rs",
     "SecurityDecision/DecisionEffect + Obligations: allowed sets, "
     "FieldMask/FieldPolicy, MandatoryFilter, query/data/mutation "
     "obligations.", "crate.security-domain")
CMOD("mod.secdom.policy", "security-domain/policy.rs", "crates/security-domain/src/policy.rs",
     "PolicyRule/PolicyRuleEffect/ResourceSelector — sec.policy_rule "
     "model.", "crate.security-domain")
CMOD("mod.secdom.principal", "security-domain/principal.rs", "crates/security-domain/src/principal.rs",
     "Principal/GroupClass(capability|data)/PrincipalGroupMembership.",
     "crate.security-domain")

CMOD("mod.pdp.engine", "security-pdp/engine.rs", "crates/security-pdp/src/engine.rs",
     "PdpEngine.evaluate — the pure 9-step decision pipeline; "
     "AuthorizationRequest/Resource, PdpSnapshot, PdpError (fail closed).",
     "crate.security-pdp", specs=("kb.docs.08",))

CMOD("mod.secapi.client", "security-api/client.rs", "crates/security-api/src/client.rs",
     "SecurityClient trait + LocalSecurityClient — in-process PDP calls, "
     "no self-HTTP (ADR-008).", "crate.security-api")
CMOD("mod.secapi.contract", "security-api/contract.rs", "crates/security-api/src/contract.rs",
     "Contract DTOs: ActionDecisionRequest/Response, "
     "QueryDecisionRequest/Response, DecisionEffectDto, "
     "SecurityContextDto — same for local + mTLS transports.",
     "crate.security-api")
CMOD("mod.secapi.routes", "security-api/routes.rs", "crates/security-api/src/routes.rs",
     "Internal Rocket controllers for mTLS peers; local_client() helper "
     "wires LocalSecurityClient.", "crate.security-api")

CMOD("mod.sr.draft", "schema-registry/draft.rs", "crates/schema-registry/src/draft.rs",
     "SchemaDraft: mutable working copy; publish freezes + content hash → "
     "immutable SchemaRevision; SchemaDraftError.", "crate.schema-registry")
CMOD("mod.sr.validate", "schema-registry/validate.rs", "crates/schema-registry/src/validate.rs",
     "ValidationReport/Finding/Severity: aliases, domain/range/"
     "cardinality, inheritance cycles, inverse/symmetric coherence, "
     "normalizers, confusables (docs/12 §2).", "crate.schema-registry")
CMOD("mod.sr.bundle", "schema-registry/bundle.rs", "crates/schema-registry/src/bundle.rs",
     "Bundle loader: JSON bundle files → LoadedBundle → SchemaDraft; "
     "referential integrity checks; 10 Rust tests.", "crate.schema-registry")

CMOD("mod.ident.normalizers", "identity/normalizers.rs", "crates/identity/src/normalizers.rs",
     "normalize_text (NFC+casefold, idempotent), "
     "normalize_text_search_key, normalize_email, normalize_phone.",
     "crate.identity")

CMOD("mod.qe.ast", "query-engine/ast.rs", "crates/query-engine/src/ast.rs",
     "QueryAst/WhereExpr/Operator/FieldRef/SelectField/SortKey/"
     "RelationDirection/SystemField + syntactic limits + QueryError.",
     "crate.query-engine")
CMOD("mod.qe.rewrite", "query-engine/rewrite.rs", "crates/query-engine/src/rewrite.rs",
     "SecurityRewriter: PDP QueryObligations → rewritten AST (allowed "
     "sets, mandatory_filters, hidden-field sort/facet rejection, "
     "page-size clamp, aggregate bans).", "crate.query-engine")
CMOD("mod.qe.compile", "query-engine/compile.rs", "crates/query-engine/src/compile.rs",
     "SQL compiler → CompiledQuery/SqlFragment: $N binds, identifiers "
     "only from catalog, over core.entity+core.statement.",
     "crate.query-engine")

CMOD("mod.conn.sdk", "connectors/sdk.rs", "crates/connectors/src/sdk.rs",
     "Connector trait + ConnectorContext + SearchQuery/SearchResponse — "
     "the source-adapter contract.", "crate.connectors")
CMOD("mod.conn.descriptor", "connectors/descriptor.rs", "crates/connectors/src/descriptor.rs",
     "ConnectorDescriptor/ConnectorKind/ConnectorCapabilities/"
     "LegalMetadata.", "crate.connectors")
CMOD("mod.conn.instance", "connectors/instance.rs", "crates/connectors/src/instance.rs",
     "ConnectorInstance/ConnectorInstanceConfig (env, secrets, rps, "
     "retry, cache, legal basis) + RetentionPolicy + TrustRank.",
     "crate.connectors")
CMOD("mod.conn.observation", "connectors/observation.rs", "crates/connectors/src/observation.rs",
     "SourceObservation/ProvenanceMeta (content hash, trust rank, "
     "purpose)/MappingResult/MappedPredicate/IdentityResolution/"
     "RawPayloadPolicy.", "crate.connectors")
CMOD("mod.conn.rate_limit", "connectors/rate_limit.rs", "crates/connectors/src/rate_limit.rs",
     "RateLimiter (token bucket), CircuitBreaker (closed/open/half-open), "
     "backoff_delay.", "crate.connectors")

CMOD("mod.risk.ast", "risk-engine/ast.rs", "crates/risk-engine/src/ast.rs",
     "Criteria DSL: Expr/ExprKind 18 nodes, Value, ComparisonOp/BinaryOp/"
     "LogicalOp/UnaryOp, pure evaluate() + EvaluationContext.",
     "crate.risk-engine")
CMOD("mod.risk.criterion", "risk-engine/criterion.rs", "crates/risk-engine/src/criterion.rs",
     "Criterion (severity/weight/explanation_key/evidence_selector/"
     "valid_interval/owner/tests) + CriterionSeverity weights "
     "0/10/25/50/100.", "crate.risk-engine")
CMOD("mod.risk.factor", "risk-engine/factor.rs", "crates/risk-engine/src/factor.rs",
     "Factor (dedup_key, status Active/Resolved/Suppressed/Overridden/"
     "Expired, suppression/override, evidence refs).", "crate.risk-engine")
CMOD("mod.risk.score", "risk-engine/score.rs", "crates/risk-engine/src/score.rs",
     "WeightedAggregator + ScoreConfig (avg|max, penalties, cap) + "
     "ScoreResult + RiskLevel (0-100).", "crate.risk-engine")
CMOD("mod.risk.assessment", "risk-engine/assessment.rs", "crates/risk-engine/src/assessment.rs",
     "Assessment aggregate (Draft/Approved/Superseded): target/as_of, "
     "model/version, input watermark/fingerprint, factor refs, component "
     "scores, Uncertainty.", "crate.risk-engine")
CMOD("mod.risk.decision", "risk-engine/decision.rs", "crates/risk-engine/src/decision.rs",
     "Decision (Approve/Reject/Escalate/Defer/ConditionalApprove) "
     "system|manual; requires_approval for High/Critical.",
     "crate.risk-engine")

CMOD("mod.wf.state_machine", "workflow/state_machine.rs", "crates/workflow/src/state_machine.rs",
     "StateMachineDefinition/State/StateTransition/TransitionGuard/"
     "StateMachine — validated transitions, history, terminal detection.",
     "crate.workflow")
CMOD("mod.wf.job", "workflow/job.rs", "crates/workflow/src/job.rs",
     "Job/JobState/JobStatus: lease/start/heartbeat/progress/cursor/"
     "wait/resume/succeed/fail/cancel; cooperative cancellation + "
     "irreversible_step.", "crate.workflow")
CMOD("mod.wf.task", "workflow/task.rs", "crates/workflow/src/task.rs",
     "Task/TaskStatus/TaskPriority/AssignmentStrategy: assign/accept/"
     "complete/cancel/overdue/escalate.", "crate.workflow")
CMOD("mod.wf.timer", "workflow/timer.rs", "crates/workflow/src/timer.rs",
     "Schedule/ScheduleConfig (IANA tz, calendar expr, MisfirePolicy, "
     "concurrency, quiet period) + EscalationTimer/TimerKind.",
     "crate.workflow")
CMOD("mod.wf.reaction", "workflow/reaction.rs", "crates/workflow/src/reaction.rs",
     "ReactionConfig/ReactionTriggerKind (12 triggers)/ActionKind (10 "
     "actions)/Reaction/ReactionResult — cooldown, dedup_key, "
     "idempotency, retries, compensation.", "crate.workflow")

CMOD("mod.files.upload", "files/upload.rs", "crates/files/src/upload.rs",
     "StagedUpload: init/receive_chunk (idempotent)/complete (checksum+"
     "size)/abort/progress; UploadStatus.", "crate.files")
CMOD("mod.files.scan", "files/scan.rs", "crates/files/src/scan.rs",
     "FileScanResult/Status/FileSignature/ScanFinding/ScanSeverity/"
     "ScanConfig/FileScanner trait.", "crate.files")
CMOD("mod.files.storage", "files/storage.rs", "crates/files/src/storage.rs",
     "ObjectMetadata/StorageClass/StorageConfig/StorageUrl/"
     "ObjectStorage trait; TTL, legal hold.", "crate.files")

CMOD("mod.rep.dataset", "reporting/dataset.rs", "crates/reporting/src/dataset.rs",
     "Dataset/DatasetColumn(9 types)/DatasetQuery/DatasetStatus/"
     "DatasetSnapshot (pinned revisions, content hash).",
     "crate.reporting")
CMOD("mod.rep.definition", "reporting/definition.rs", "crates/reporting/src/definition.rs",
     "ReportDefinition (sections/tables/charts/summaries, formats, "
     "audiences, watermark, page/row limits, schedule) + ExportFormat + "
     "ReportAudience.", "crate.reporting")
CMOD("mod.rep.template", "reporting/template.rs", "crates/reporting/src/template.rs",
     "TemplateDefinition/TemplateVersion/TemplateLocale/FieldFormat/"
     "TemplateFormat — masked field patterns.", "crate.reporting")
CMOD("mod.rep.render", "reporting/render.rs", "crates/reporting/src/render.rs",
     "TemplateRenderer + RenderContext/Output + "
     "escape_formula_injection (spreadsheet safety).", "crate.reporting")

CMOD("mod.sec.pdp", "security/pdp.rs", "crates/security/src/pdp.rs",
     "PdpClient/LocalPdpClient/PolicySnapshot/Principal/SessionInfo/"
     "DecisionContext/Decision — self-contained PEP+PDP.",
     "crate.security")
CMOD("mod.sec.abac", "security/abac.rs", "crates/security/src/abac.rs",
     "DataRule/DataPermission/ResourceSelector/Effect/AttributeValue.",
     "crate.security")
CMOD("mod.sec.capability", "security/capability.rs", "crates/security/src/capability.rs",
     "Capability/CapabilityGroup/CapabilityGroupKind/CapabilityKind.",
     "crate.security")
CMOD("mod.sec.classification", "security/classification.rs", "crates/security/src/classification.rs",
     "ClassificationLevel/FieldPolicy/FieldPolicyKind/MaskStrategy — "
     "field masking policies.", "crate.security")
CMOD("mod.sec.audit", "security/audit.rs", "crates/security/src/audit.rs",
     "AuditEntry/AuditKind/AuditQuery/AuditRetention/TamperChain.",
     "crate.security")

CMOD("mod.ap.chain", "audit-persistence/chain.rs", "crates/audit-persistence/src/chain.rs",
     "ChainHasher/ChainState — BLAKE2b chain where each event hash "
     "includes the previous hash.", "crate.audit-persistence")
CMOD("mod.ap.repository", "audit-persistence/repository.rs", "crates/audit-persistence/src/repository.rs",
     "AuditRepository trait + PgAuditRepository + AuditRecord/"
     "ChainCheckpoint/ChainVerificationResult/RetentionPolicyRecord.",
     "crate.audit-persistence",
     tables=("audit.event", "audit.chain_checkpoint",
             "audit.retention_policy"))

CMOD("mod.integ.check_flow", "integration/check_flow.rs", "crates/integration/src/check_flow.rs",
     "CheckFlowOrchestrator — full pipeline: request→connector→mapping→"
     "risk→decision→workflow→reactions; CheckFlowInput/Context/State/"
     "Result.", "crate.integration")
CMOD("mod.integ.ports", "integration/ports.rs", "crates/integration/src/ports.rs",
     "Ports: ConnectorPort/CriterionPort/FactorPort/AssessmentPort/"
     "DecisionPort/WorkflowPort/TaskPort/AuditPort.", "crate.integration")

CMOD("mod.ts.invariants", "test-support/invariants.rs", "crates/test-support/src/invariants.rs",
     "Cross-crate invariant tests: security pipeline + schema registry + "
     "identity normalizers end-to-end.", "crate.test-support")


# =========================================================================
# DB tables (PostgreSQL, schemas core/meta/sec/ops/audit/integration)
# =========================================================================
TABLES = [
    ("core.tenant", "Tenants — root isolation boundary for every RLS policy."),
    ("core.entity", "Entity headers: internal i64 id, public uuid, type_id, "
     "class, canonical_entity_id (merge winner), display_name, status, "
     "revision, created_by. RLS tenant_isolation."),
    ("core.entity_alias", "Public ids of merged-away entities → canonical "
     "entity (merge loser lookup)."),
    ("core.entity_revision", "Append-only revision snapshots per entity "
     "(history feed + optimistic concurrency)."),
    ("core.statement", "Append-only facts: subject–predicate→object "
     "(edge or scalar), status active/superseded, valid_from/valid_to, "
     "provenance, fingerprint. statement_active_fact_unique = md5-hashed "
     "dedup index (mig 24)."),
    ("core.identity_key", "Computed identity keys per entity+policy — "
     "dedup/merge resolution via unique constraint."),
    ("core.check_instance", "Check instances: entity, kind, status "
     "pending/running/completed/failed, evidence jsonb, completed_at."),
    ("core.risk_factor", "Risk factors: entity, criterion, level/severity, "
     "evidence, resolved flag."),
    ("core.risk_assessment", "Risk assessments: entity, status, score/"
     "level, decision, rationale, approved_by."),
    ("core.task", "Workflow tasks: entity/workflow refs, kind, title, "
     "priority, status, assignee, due_date, resolution."),
    ("core.workflow_instance", "Workflow instances: definition ref, "
     "entity, current state, variables jsonb."),
    ("core.request", "Requests/processes: entity, product, limit, "
     "urgency, status, due_date."),
    ("core.file_object", "Stored file objects incl. content bytea, "
     "content_type, size, sha256 hash."),
    ("core.entity_document", "Entity↔document links: kind, title, "
     "file ref, qualifiers/validity/source."),
    ("core.report_instance", "Generated reports: entity, template, "
     "format, status, file ref, params."),
    ("core.upload_session", "Staged upload sessions: expected size/hash/"
     "chunk_size, received_chunks, status."),
    ("core.upload_chunk", "Per-chunk staged bytes (bytea) + per-chunk "
     "hash — survives restarts."),
    ("meta.schema_revision", "Schema registry: draft/published/retired "
     "revisions w/ etag version + semantic_version + content hash; "
     "unique partial index one published/tenant."),
    ("meta.type_definition", "Type definitions per revision: stable_alias, "
     "label, entity_class, fields jsonb."),
    ("meta.predicate_definition", "Predicate definitions: stable_alias, "
     "scalar_kind or is_edge + domain/range, labels."),
    ("meta.localized_text", "Localized labels/descriptions per locale."),
    ("meta.identity_policy", "Identity policies derived at publish from "
     "identity:true fields."),
    ("meta.workflow_definition", "Workflow definitions (state machine "
     "JSON) referenced by core.workflow_instance."),
    ("sec.principal", "Principals (users/service accounts): login display, "
     "status, email, locked_until."),
    ("sec.group", "Groups/roles: class capability|data, alias, parent_id "
     "(role tree), special ''|superadmin|superuser."),
    ("sec.principal_group_membership", "Principal↔group memberships with "
     "valid_from/valid_to + groups_revision."),
    ("sec.policy_revision", "Policy revisions; one published per tenant."),
    ("sec.policy_rule", "Authorization rules: effect allow|deny, action "
     "(capability or data.* grammar), resource jsonb selector, priority."),
    ("sec.decision", "PDP decision audit records."),
    ("sec.session", "UI sessions: csrf_token, user_agent, ip, "
     "last_seen_at — RLS disabled (pre-auth credential)."),
    ("sec.principal_credential", "Globally-unique login + argon2id PHC "
     "hash — pre-auth directory, outside RLS."),
    ("sec.api_token", "API tokens egr_*: token_hash unique, principal, "
     "name, expiry/revocation."),
    ("ops.outbox", "Reliable outbox events: event_type, payload, "
     "available_at, dispatched_at, attempts; worker_drain policy lets "
     "enginer_worker see all tenants."),
    ("ops.inbox", "Inbox for inbound dedup of external events."),
    ("ops.job", "Durable jobs: scheduled_at, priority, status."),
    ("ops.projection_watermark", "Projection/read-model progress "
     "watermarks."),
    ("audit.event", "Tamper-evident audit: kind, actor, action, resource, "
     "effect, occurred_at, payload, chain seq + prev_hash/hash columns. "
     "Insert-only for app role."),
    ("audit.chain_checkpoint", "Persisted latest chain hash per tenant for "
     "fast verify."),
    ("audit.retention_policy", "Per-tenant audit retention (days, "
     "archive action); enginer_retention role deletes."),
    ("integration.connector", "Connector instances: kind, display_name, "
     "enabled, config jsonb, trust_rank."),
    ("integration.external_id_mapping", "External source ids ↔ entities."),
    ("integration.sync_state", "Per-connector sync cursors/watermarks."),
    ("integration.monitor", "Monitor schedules: entity, frequency, "
     "next_run_at, last_run_at, last_result jsonb; worker_schedule policy "
     "for enginer_worker."),
]
for tname, tdesc in TABLES:
    N("table." + tname, "table", tname, "", tdesc, sort=400,
      tags="postgres," + tname.split(".")[0])
E("cm.repo", "cm_contains", "table.core.tenant")  # anchor (rest linked via migrations)


# =========================================================================
# Migrations
# =========================================================================
MIGRATIONS = [
    ("20260803000001_baseline", "Baseline: schemas core/meta/sec/ops/audit/"
     "integration/report; extensions pgcrypto/citext/pg_trgm; roles "
     "enginer_app/enginer_worker/enginer_migrate; schema usage grants.", ()),
    ("20260803000002_core_entity", "core.tenant/entity/entity_alias/"
     "entity_revision + tenant RLS policies + indexes.",
     ("core.tenant", "core.entity", "core.entity_alias",
      "core.entity_revision")),
    ("20260803000003_meta_registry", "meta.schema_revision/type_definition/"
     "predicate_definition/localized_text + one-published unique index + "
     "RLS.",
     ("meta.schema_revision", "meta.type_definition",
      "meta.predicate_definition", "meta.localized_text")),
    ("20260803000004_sec_security", "sec.principal/group/membership/"
     "policy_revision/policy_rule/decision/session + RLS loop.",
     ("sec.principal", "sec.group", "sec.principal_group_membership",
      "sec.policy_revision", "sec.policy_rule", "sec.decision",
      "sec.session")),
    ("20260803000005_ops_outbox", "ops.outbox/inbox/job/"
     "projection_watermark + pending indexes + RLS.",
     ("ops.outbox", "ops.inbox", "ops.job", "ops.projection_watermark")),
    ("20260803000006_audit_event", "audit.event + 4 indexes + RLS; "
     "INSERT,SELECT only for app roles (tamper evidence).",
     ("audit.event",)),
    ("20260803000007_integration", "integration.connector/"
     "external_id_mapping/sync_state + RLS.",
     ("integration.connector", "integration.external_id_mapping",
      "integration.sync_state")),
    ("20260803000008_core_statement", "core.statement + identity_key + "
     "meta.identity_policy + fact-dedup unique index (superseded by mig "
     "24) + RLS.",
     ("core.statement", "core.identity_key", "meta.identity_policy")),
    ("20260803000009_audit_chain", "audit.event +chain columns (seq/"
     "prev_hash/hash) + chain_checkpoint + retention_policy + "
     "enginer_retention role grants.",
     ("audit.chain_checkpoint", "audit.retention_policy")),
    ("20261018000010_sec_credentials_session", "sec.principal_credential "
     "(login unique) + sec.session +csrf_token/user_agent/ip_address/"
     "last_seen_at; session RLS DISABLED (pre-auth credential).",
     ("sec.principal_credential",)),
    ("20261018000011_core_id_sequences", "Sequence grants entity_id_seq/"
     "statement_id_seq; core.entity +type_id + index.", ()),
    ("20261018000012_sec_admin_grants", "GRANT DELETE on sec.membership/"
     "policy_rule/group to enginer_app.", ()),
    ("20261018000013_core_skr_ops", "SKR operational tables: check_instance/"
     "risk_factor/risk_assessment/task/workflow_instance/request/"
     "file_object/entity_document/report_instance + integration.monitor + "
     "RLS + grants.",
     ("core.check_instance", "core.risk_factor", "core.risk_assessment",
      "core.task", "core.workflow_instance", "core.request",
      "core.file_object", "core.entity_document", "core.report_instance",
      "integration.monitor")),
    ("20261018000014_core_uploads", "core.upload_session/upload_chunk + "
     "RLS.", ("core.upload_session", "core.upload_chunk")),
    ("20261018000015_ops_outbox_worker", "Policy worker_drain ON "
     "ops.outbox → enginer_worker sees all tenants (permissive OR).",
     ()),
    ("20261018000016_monitor_worker", "Policy worker_schedule ON "
     "integration.monitor → enginer_worker sees all tenants.", ()),
    ("20261018000017_audit_http_kind", "audit.event kind CHECK widened: "
     "adds http_request/denied_request kinds.", ()),
    ("20261018000018_meta_schema_revision_delete", "GRANT DELETE on "
     "meta.schema_revision (draft delete).", ()),
    ("20261018000019_sec_role_tree", "sec.group +parent_id + "
     "group_cycle_check trigger (cycle guard) + membership dedup + seeds "
     "superadmin/superuser + users.manage rule.", ()),
    ("20261018000020_sec_special_roles", "sec.group +special flag; "
     "per-tenant deterministic superadmin/superuser rows; grants granular "
     "user.* capabilities to admins.", ()),
    ("20261018000021_sec_principal_email", "sec.principal +email + "
     "lower() index.", ()),
    ("20261018000022_sec_api_token", "sec.api_token (egr_* tokens, "
     "token_hash unique).", ("sec.api_token",)),
    ("20261018000023_meta_workflow_definition", "meta.workflow_definition "
     "table.", ("meta.workflow_definition",)),
    ("20261018000024_statement_fact_hash_index", "Replaces "
     "statement_active_fact_unique: btree on raw object → md5(object::text) "
     "— fixes 2704-byte index tuple cap for long scalar facts.",
     ()),
]
for mig, desc, tabs in MIGRATIONS:
    mid = "mig." + mig
    N(mid, "migration", mig, "migrations/" + mig + ".sql", desc,
      sort=500, tags="sqlx,migration")
    E("cm.repo", "cm_contains", mid)
    for t in tabs:
        E(mid, "cm_defines", "table." + t)


# =========================================================================
# API routes — (method, path, capability, module, summary, tables)
# Path templates use <name> (Rocket style); slug normalization maps them.
# =========================================================================
ROUTES = [
    ("GET", "/health/live", "(none)", "mod.api.health",
     "Liveness probe — always 200 when the process is up.", ()),
    ("GET", "/health/ready", "(none)", "mod.api.health",
     "Readiness probe — DB executor health.", ()),

    ("GET", "/api/v1/version", "(none)", "mod.api.lib",
     "Service name/version/api_version.", ()),
    ("OPTIONS", "/api/v1/<_..>", "(none)", "mod.api.lib",
     "CORS preflight catch-all → 204.", ()),

    ("POST", "/api/v1/auth/login", "(pre-auth)", "mod.api.auth_routes",
     "Login: argon2 verify vs sec.principal_credential → create "
     "sec.session + csrf_token cookie; audits login.",
     ("sec.principal_credential", "sec.session", "audit.event")),
    ("POST", "/api/v1/auth/logout", "(session)", "mod.api.auth_routes",
     "Logout: close the session row.", ("sec.session",)),
    ("GET", "/api/v1/me/context", "(session)", "mod.api.auth_routes",
     "Principal profile: identity, capabilities, policy_revision, "
     "ui_rules (URL allow/deny) — also used by enginer-mcp to validate "
     "tokens.", ("sec.principal", "sec.session", "sec.policy_rule",
                  "sec.principal_group_membership")),
    ("POST", "/api/v1/auth/refresh", "(session)", "mod.api.auth_routes",
     "Rotate/extend session (401 refresh flow).", ("sec.session",)),

    ("GET", "/api/v1/entities", "entity.read", "mod.api.entity_pg",
     "List/search entities: ?type&search&limit&cursor.",
     ("core.entity", "core.statement", "meta.type_definition")),
    ("GET", "/api/v1/entities/<public_id>", "entity.read", "mod.api.entity_pg",
     "Entity detail: header + materialized fields from active statements.",
     ("core.entity", "core.statement")),
    ("POST", "/api/v1/entities", "entity.create", "mod.api.entity_pg",
     "Create entity: required-field validation, identity-key dedup → "
     "outcome created|deduplicated; writes outbox event.",
     ("core.entity", "core.statement", "core.identity_key",
      "meta.identity_policy", "ops.outbox")),
    ("PUT", "/api/v1/entities/<public_id>", "entity.edit", "mod.api.entity_pg",
     "Update entity fields/statements; revision = optimistic concurrency "
     "(409 on mismatch).", ("core.entity", "core.statement")),
    ("POST", "/api/v1/entities/<public_id>/retire", "entity.retire", "mod.api.entity_pg",
     "Retire entity (status→retired, outbox event).",
     ("core.entity", "ops.outbox")),
    ("POST", "/api/v1/entities/<public_id>/merge", "entity.merge", "mod.api.entity_pg",
     "Merge loser into winner: loser public ids → entity_alias, "
     "canonical_entity_id→winner.", ("core.entity", "core.entity_alias")),
    ("GET", "/api/v1/entities/<public_id>/relations", "entity.read", "mod.api.entity_pg",
     "List relations ?direction&as_of — edge statements both ways.",
     ("core.statement", "core.entity")),
    ("POST", "/api/v1/entities/<public_id>/relations", "entity.edit", "mod.api.entity_pg",
     "Create relation (edge statement); duplicate active → 409.",
     ("core.statement",)),
    ("DELETE", "/api/v1/entities/<public_id>/relations/<relation_id>", "entity.edit",
     "mod.api.entity_pg", "Retract a relation statement.",
     ("core.statement",)),
    ("GET", "/api/v1/entities/<public_id>/history", "entity.read", "mod.api.entity_pg",
     "Revision history: per-revision predicate diffs.",
     ("core.entity_revision", "core.statement")),
    ("POST", "/api/v1/entities/<public_id>/graph", "entity.read", "mod.api.entity_pg",
     "Neighborhood graph BFS: {max_depth,direction,locale,predicate_id} → "
     "nodes+edges with cycle markers (feeds GraphView).",
     ("core.statement", "core.entity")),
    ("POST", "/api/v1/entities/query", "query.advanced", "mod.api.entity_pg",
     "Advanced query AST ({from,where,select,sort,page,filters,search}) → "
     "security rewrite → parameterized SQL → rows+cursor.",
     ("core.entity", "core.statement", "meta.type_definition")),

    ("GET", "/api/v1/schemas/drafts", "schema.draft", "mod.api.schema_admin_routes",
     "List schema drafts.", ("meta.schema_revision",)),
    ("GET", "/api/v1/schemas/drafts/<id>", "schema.draft", "mod.api.schema_admin_routes",
     "Draft detail: types+predicates arrays + etag.",
     ("meta.schema_revision", "meta.type_definition",
      "meta.predicate_definition")),
    ("POST", "/api/v1/schemas/drafts", "schema.draft", "mod.api.schema_admin_routes",
     "Create draft; optional base_revision (inherits latest published "
     "when omitted).", ("meta.schema_revision",)),
    ("PUT", "/api/v1/schemas/drafts/<id>", "schema.draft", "mod.api.schema_admin_routes",
     "Replace draft catalog (types/predicates/changes); If-Match etag "
     "required; bumps version.", ("meta.schema_revision",)),
    ("DELETE", "/api/v1/schemas/drafts/<id>", "schema.draft", "mod.api.schema_admin_routes",
     "Delete a draft (meta.schema_revision DELETE grant, mig 18).",
     ("meta.schema_revision",)),
    ("POST", "/api/v1/schemas/drafts/<id>/validate", "schema.draft", "mod.api.schema_admin_routes",
     "Structural validation → findings.", ("meta.schema_revision",)),
    ("POST", "/api/v1/schemas/drafts/<id>/impact", "schema.draft", "mod.api.schema_admin_routes",
     "Impact analysis: additive vs breaking changes.",
     ("meta.schema_revision",)),
    ("POST", "/api/v1/schemas/drafts/<id>/publish", "schema.publish", "mod.api.schema_admin_routes",
     "Publish (step-up cap): freezes revision, writes type/predicate/"
     "identity-policy/localized rows, auto-retires previous published.",
     ("meta.schema_revision", "meta.type_definition",
      "meta.predicate_definition", "meta.identity_policy",
      "meta.localized_text")),
    ("GET", "/api/v1/schemas/published", "entity.read", "mod.api.schema_admin_routes",
     "Current published revision + full catalog.",
     ("meta.schema_revision",)),
    ("GET", "/api/v1/schemas/revisions/<id>", "entity.read", "mod.api.schema_admin_routes",
     "Get a specific revision.", ("meta.schema_revision",)),
    ("POST", "/api/v1/schemas/revisions/<id>/retire", "schema.publish", "mod.api.schema_admin_routes",
     "Retire — only works on the CURRENT published revision.",
     ("meta.schema_revision",)),
    ("GET", "/api/v1/schemas/revisions/<id>/migration-status", "entity.read",
     "mod.api.schema_admin_routes", "Physical migration status of a "
     "published revision.", ("meta.schema_revision",)),
    ("GET", "/api/v1/schemas/types", "entity.read", "mod.api.schema_admin_routes",
     "Published type catalog (entity counts included).",
     ("meta.type_definition", "core.entity")),
    ("GET", "/api/v1/schemas/types/<type_id>", "entity.read", "mod.api.schema_admin_routes",
     "Type detail + fields.", ("meta.type_definition",)),
    ("GET", "/api/v1/schemas/types/<type_id>/view", "entity.read",
     "mod.api.schema_admin_routes",
     "Synthesized view definition ?view_type=form|list — drives the "
     "metadata-driven UI (EntityFormRenderer/EntityListRenderer).",
     ("meta.type_definition",)),
    ("GET", "/api/v1/schemas/predicates", "entity.read", "mod.api.schema_admin_routes",
     "Published predicate catalog.", ("meta.predicate_definition",)),

    ("GET", "/api/v1/checks", "check.view", "mod.api.skr_pg",
     "Check instances ?target_entity_id&status&kind&limit&cursor.",
     ("core.check_instance",)),
    ("GET", "/api/v1/checks/<check_id>", "check.view", "mod.api.skr_pg",
     "Check detail.", ("core.check_instance",)),
    ("GET", "/api/v1/checks/<check_id>/evidence", "check.view", "mod.api.skr_pg",
     "Check evidence items.", ("core.check_instance",)),
    ("POST", "/api/v1/checks", "check.run", "mod.api.skr_pg",
     "Create check instance + enqueue ops.outbox 'check.execute' "
     "(worker completes it).", ("core.check_instance", "ops.outbox")),
    ("GET", "/api/v1/entities/<public_id>/checks", "check.view", "mod.api.skr_pg",
     "Checks for one entity.", ("core.check_instance",)),
    ("GET", "/api/v1/assessments", "assessment.view", "mod.api.skr_pg",
     "Assessment catalog ?status&limit&cursor.", ("core.risk_assessment",)),
    ("GET", "/api/v1/entities/<public_id>/assessment", "assessment.view", "mod.api.skr_pg",
     "Latest assessment for entity.", ("core.risk_assessment",)),
    ("POST", "/api/v1/entities/<public_id>/assessment", "assessment.view", "mod.api.skr_pg",
     "Create/record assessment (decision+rationale; force flag).",
     ("core.risk_assessment", "core.risk_factor")),
    ("GET", "/api/v1/entities/<public_id>/factors", "risk.view", "mod.api.skr_pg",
     "Risk factors of entity.", ("core.risk_factor",)),
    ("POST", "/api/v1/entities/<public_id>/factors/<factor_id>/resolve", "risk.resolve",
     "mod.api.skr_pg", "Resolve a risk factor (resolution note).",
     ("core.risk_factor",)),
    ("GET", "/api/v1/entities/<public_id>/tasks", "entity.read", "mod.api.skr_pg",
     "Tasks linked to entity.", ("core.task",)),
    ("POST", "/api/v1/entities/<public_id>/tasks", "entity.edit", "mod.api.skr_pg",
     "Create task on entity.", ("core.task",)),
    ("POST", "/api/v1/tasks/<task_id>/start", "entity.edit", "mod.api.skr_pg",
     "Task pending→in_progress.", ("core.task",)),
    ("POST", "/api/v1/tasks/<task_id>/complete", "entity.edit", "mod.api.skr_pg",
     "Task→completed + resolution.", ("core.task",)),
    ("POST", "/api/v1/tasks/<task_id>/cancel", "entity.edit", "mod.api.skr_pg",
     "Task→cancelled.", ("core.task",)),
    ("POST", "/api/v1/workflows", "entity.create", "mod.api.skr_pg",
     "Start workflow instance from meta.workflow_definition; seeds tasks.",
     ("core.workflow_instance", "meta.workflow_definition", "core.task")),
    ("GET", "/api/v1/workflows/<instance_id>", "entity.read", "mod.api.skr_pg",
     "Workflow instance detail.", ("core.workflow_instance",)),
    ("POST", "/api/v1/workflows/<instance_id>/transition", "entity.edit",
     "mod.api.skr_pg", "Trigger transition {trigger,to}.",
     ("core.workflow_instance",)),
    ("GET", "/api/v1/workflows/<instance_id>/tasks", "entity.read", "mod.api.skr_pg",
     "Tasks of a workflow instance.", ("core.task",)),
    ("GET", "/api/v1/requests", "entity.read", "mod.api.skr_pg",
     "Requests/processes ?status&limit&cursor.", ("core.request",)),
    ("POST", "/api/v1/requests", "entity.create", "mod.api.skr_pg",
     "Create request (target entity, product, limit, urgency).",
     ("core.request",)),
    ("GET", "/api/v1/monitors", "monitor.manage", "mod.api.skr_pg",
     "Monitor schedules + last_result.", ("integration.monitor",)),
    ("POST", "/api/v1/monitors/<id>/run", "monitor.manage", "mod.api.skr_pg",
     "Enqueue immediate monitor run (outbox).",
     ("integration.monitor", "ops.outbox")),
    ("GET", "/api/v1/connectors", "connector.manage", "mod.api.skr_pg",
     "Connector instances.", ("integration.connector",)),
    ("GET", "/api/v1/connectors/<id>", "connector.manage", "mod.api.skr_pg",
     "Connector detail.", ("integration.connector",)),
    ("POST", "/api/v1/connectors", "connector.manage", "mod.api.skr_pg",
     "Register connector instance.", ("integration.connector",)),
    ("POST", "/api/v1/connectors/<id>/enable", "connector.manage", "mod.api.skr_pg",
     "Enable connector.", ("integration.connector",)),
    ("POST", "/api/v1/connectors/<id>/disable", "connector.manage", "mod.api.skr_pg",
     "Disable connector.", ("integration.connector",)),
    ("POST", "/api/v1/connectors/<id>/health", "connector.manage", "mod.api.skr_pg",
     "Connector health check.", ("integration.connector",)),
    ("PATCH", "/api/v1/connectors/<id>", "connector.manage", "mod.api.skr_pg",
     "Patch connector instance (name/enabled/config keys).",
     ("integration.connector",)),
    ("DELETE", "/api/v1/connectors/<id>", "connector.manage", "mod.api.skr_pg",
     "Delete connector instance.", ("integration.connector",)),
    ("GET", "/api/v1/dashboard/stats", "entity.read", "mod.api.skr_pg",
     "Aggregate counters for the dashboard (tasks/requests/checks/"
     "factors/monitors).",
     ("core.task", "core.request", "core.check_instance",
      "core.risk_factor", "integration.monitor")),

    ("POST", "/api/v1/files/upload/init", "entity.edit", "mod.api.files_pg",
     "Staged upload init: expected size/hash/chunk_size → upload_session.",
     ("core.upload_session",)),
    ("PUT", "/api/v1/files/upload/<upload_id>/chunks/<index>", "entity.edit",
     "mod.api.files_pg", "Raw octet-stream chunk PUT; idempotent by "
     "chunk hash.", ("core.upload_chunk",)),
    ("POST", "/api/v1/files/upload/<upload_id>/complete", "entity.edit", "mod.api.files_pg",
     "Verify final hash+size → materialize core.file_object → object_id.",
     ("core.upload_session", "core.file_object", "core.upload_chunk")),
    ("POST", "/api/v1/files/upload/<upload_id>/abort", "entity.edit", "mod.api.files_pg",
     "Abort staged upload.", ("core.upload_session",)),
    ("GET", "/api/v1/files/upload/<upload_id>", "entity.edit", "mod.api.files_pg",
     "Upload status/progress.", ("core.upload_session",)),
    ("GET", "/api/v1/files/<object_id>", "entity.edit", "mod.api.files_pg",
     "File object metadata.", ("core.file_object",)),
    ("GET", "/api/v1/files/<object_id>/download", "entity.edit", "mod.api.files_pg",
     "Raw file bytes.", ("core.file_object",)),
    ("DELETE", "/api/v1/files/<object_id>", "entity.edit", "mod.api.files_pg",
     "Delete file object.", ("core.file_object",)),
    ("GET", "/api/v1/entities/<public_id>/documents", "entity.read", "mod.api.files_pg",
     "Entity documents.", ("core.entity_document",)),
    ("POST", "/api/v1/entities/<public_id>/documents", "entity.edit", "mod.api.files_pg",
     "Attach document: kind/title/content_type + bytes → file_object + "
     "entity_document link.", ("core.entity_document", "core.file_object")),
    ("GET", "/api/v1/entities/<public_id>/documents/<document_id>/download",
     "entity.read", "mod.api.files_pg", "Download entity document.",
     ("core.entity_document", "core.file_object")),
    ("DELETE", "/api/v1/entities/<public_id>/documents/<document_id>",
     "entity.edit", "mod.api.files_pg", "Remove entity document link.",
     ("core.entity_document",)),
    ("GET", "/api/v1/reports", "report.view", "mod.api.files_pg",
     "Report instances.", ("core.report_instance",)),
    ("POST", "/api/v1/reports", "report.run", "mod.api.files_pg",
     "Generate report (entity, template, format).",
     ("core.report_instance",)),

    ("GET", "/api/v1/audit", "audit.read", "mod.api.audit_pg",
     "Audit query ?kind&actor&action&resource_kind&resource_id&effect&"
     "from&to&limit&cursor.", ("audit.event",)),
    ("GET", "/api/v1/audit/checkpoint", "audit.read", "mod.api.audit_pg",
     "Latest chain checkpoint.", ("audit.chain_checkpoint",)),
    ("POST", "/api/v1/audit/checkpoint", "security.manage", "mod.api.audit_pg",
     "Persist a new chain checkpoint.", ("audit.chain_checkpoint",)),
    ("POST", "/api/v1/audit/verify", "security.manage", "mod.api.audit_pg",
     "Verify hash chain end-to-end (ChainHasher recompute).",
     ("audit.event",)),
    ("GET", "/api/v1/audit/retention", "audit.read", "mod.api.audit_pg",
     "Tenant retention policy.", ("audit.retention_policy",)),
    ("PUT", "/api/v1/audit/retention", "security.manage", "mod.api.audit_pg",
     "Set retention policy.", ("audit.retention_policy",)),
    ("GET", "/api/v1/permissions/groups", "admin.pages.access", "mod.api.audit_pg",
     "Groups + memberships + policy rules.",
     ("sec.group", "sec.principal_group_membership", "sec.policy_rule")),
    ("POST", "/api/v1/permissions/groups", "security.manage", "mod.api.audit_pg",
     "Create group (class data|capability) + rules.", ("sec.group",)),
    ("PUT", "/api/v1/permissions/groups/<group_id>", "security.manage",
     "mod.api.audit_pg", "Update group: principals (logins), rules, "
     "parent.", ("sec.group", "sec.principal_group_membership",
                   "sec.policy_rule")),
    ("DELETE", "/api/v1/permissions/groups/<group_id>", "security.manage",
     "mod.api.audit_pg", "Delete group.", ("sec.group",)),

    ("GET", "/api/v1/users/groups", "user.read|users.manage", "mod.api.users_pg",
     "Role/group list for the user-admin UI.", ("sec.group",)),
    ("GET", "/api/v1/users", "user.read|users.manage", "mod.api.users_pg",
     "Users ?q&group_id&inherited&ip&status&limit.", ("sec.principal",)),
    ("POST", "/api/v1/users", "user.create|users.manage", "mod.api.users_pg",
     "Create principal + credential.", ("sec.principal",
                                         "sec.principal_credential")),
    ("GET", "/api/v1/users/<principal_id>", "user.read|users.manage", "mod.api.users_pg",
     "User detail + groups + status.", ("sec.principal",)),
    ("PUT", "/api/v1/users/<principal_id>", "user.edit|users.manage", "mod.api.users_pg",
     "Update user (status/groups/email...).", ("sec.principal",)),
    ("DELETE", "/api/v1/users/<principal_id>", "user.delete|users.manage",
     "mod.api.users_pg", "Deactivate user.", ("sec.principal",)),
    ("GET", "/api/v1/users/<principal_id>/activity", "user.read|users.manage",
     "mod.api.users_pg", "Activity feed: entities created, revisions, "
     "audit events.", ("audit.event", "core.entity",
                        "core.entity_revision")),
    ("POST", "/api/v1/users/<principal_id>/tokens", "user.edit|users.manage",
     "mod.api.users_pg", "Issue API token (egr_*; raw shown once, hash "
     "stored).", ("sec.api_token",)),
    ("GET", "/api/v1/users/<principal_id>/tokens", "user.read|users.manage",
     "mod.api.users_pg", "List tokens.", ("sec.api_token",)),
    ("DELETE", "/api/v1/users/<principal_id>/tokens/<token_id>", "user.edit|users.manage",
     "mod.api.users_pg", "Revoke token.", ("sec.api_token",)),

    ("POST", "/internal/security/v1/decisions/action", "(internal-mtls)",
     "mod.secapi.routes", "PDP action decision for peer services.",
     ("sec.policy_rule",)),
    ("POST", "/internal/security/v1/decisions/query", "(internal-mtls)",
     "mod.secapi.routes", "PDP query decision (query obligations).",
     ("sec.policy_rule",)),
    ("GET", "/internal/security/v1/policies/revision", "(internal-mtls)",
     "mod.secapi.routes", "Current published policy revision.",
     ("sec.policy_revision",)),
]


def _slug(method, path):
    segs = [s for s in path.strip("/").split("/") if s]
    out = [method.lower()]
    for s in segs:
        s = s.strip("<>").strip("{}")
        if s in ("..", "_.."):
            s = "rest"
        out.append(s.replace("<", "").replace(">", "") or "x")
    return ".".join(out)


def _norm(path):
    """Normalize a path template: placeholders -> 'p'."""
    import re
    return re.sub(r"[<{][^>}]*[>}]", "p", path)


ROUTE_IDS = {}   # (method, normalized_path) -> cm_id
for method, path, cap, module, summary, tables in ROUTES:
    rid = "route." + _slug(method, path)
    ROUTE_IDS[(method, _norm(path))] = rid
    N(rid, "route", f"{method} {path}", path, summary,
      method=method, cap=cap, sort=600, tags="endpoint,api")
    E(module, "cm_exposes", rid)
    for t in tables:
        E(rid, "cm_persists", "table." + t)


# =========================================================================
# MCP tools — (name, method, path, capability) extracted from tools.rs
# =========================================================================
TOOLS = [
    ("context.get", "GET", "/me/context", ""),
    ("dashboard.stats", "GET", "/dashboard/stats", "entity.read"),
    ("entities.list", "GET", "/entities", "entity.read"),
    ("entities.get", "GET", "/entities/{id}", "entity.read"),
    ("entities.create", "POST", "/entities", "entity.create"),
    ("entities.update", "PUT", "/entities/{id}", "entity.edit"),
    ("entities.retire", "POST", "/entities/{id}/retire", "entity.retire"),
    ("entities.merge", "POST", "/entities/{id}/merge", "entity.merge"),
    ("entities.query", "POST", "/entities/query", "query.advanced"),
    ("entities.graph", "POST", "/entities/{id}/graph", "entity.read"),
    ("entities.history", "GET", "/entities/{id}/history", "entity.read"),
    ("relations.list", "GET", "/entities/{id}/relations", "entity.read"),
    ("relations.create", "POST", "/entities/{id}/relations", "entity.edit"),
    ("relations.delete", "DELETE", "/entities/{id}/relations/{relation_id}", "entity.edit"),
    ("documents.list", "GET", "/entities/{id}/documents", "entity.read"),
    ("documents.upload", "POST", "/entities/{id}/documents", "entity.edit"),
    ("documents.delete", "DELETE", "/entities/{id}/documents/{doc_id}", "entity.edit"),
    ("documents.download", "GET", "/entities/{id}/documents/{doc_id}/download", "entity.read"),
    ("files.get", "GET", "/files/{id}", "entity.edit"),
    ("files.delete", "DELETE", "/files/{id}", "entity.edit"),
    ("files.upload_init", "POST", "/files/upload/init", "entity.edit"),
    ("files.upload_status", "GET", "/files/upload/{id}", "entity.edit"),
    ("files.upload_abort", "POST", "/files/upload/{id}/abort", "entity.edit"),
    ("files.upload_chunk", "PUT", "/files/upload/{id}/chunks/{index}", "entity.edit"),
    ("files.upload_complete", "POST", "/files/upload/{id}/complete", "entity.edit"),
    ("files.download", "GET", "/files/{id}/download", "entity.edit"),
    ("tasks.list", "GET", "/entities/{id}/tasks", "entity.read"),
    ("tasks.create", "POST", "/entities/{id}/tasks", "entity.edit"),
    ("tasks.start", "POST", "/tasks/{id}/start", "entity.edit"),
    ("tasks.complete", "POST", "/tasks/{id}/complete", "entity.edit"),
    ("tasks.cancel", "POST", "/tasks/{id}/cancel", "entity.edit"),
    ("factors.list", "GET", "/entities/{id}/factors", "risk.view"),
    ("factors.resolve", "POST", "/entities/{id}/factors/{factor_id}/resolve", "risk.resolve"),
    ("assessment.get", "GET", "/entities/{id}/assessment", "assessment.view"),
    ("assessment.create", "POST", "/entities/{id}/assessment", "assessment.view"),
    ("assessments.list", "GET", "/assessments", "assessment.view"),
    ("checks.list", "GET", "/checks", "check.view"),
    ("checks.get", "GET", "/checks/{id}", "check.view"),
    ("checks.evidence", "GET", "/checks/{id}/evidence", "check.view"),
    ("checks.create", "POST", "/checks", "check.run"),
    ("entity_checks.list", "GET", "/entities/{id}/checks", "check.view"),
    ("schema.types", "GET", "/schemas/types", "entity.read"),
    ("schema.type_get", "GET", "/schemas/types/{alias}", "entity.read"),
    ("schema.type_view", "GET", "/schemas/types/{alias}/view", "entity.read"),
    ("schema.predicates", "GET", "/schemas/predicates", "entity.read"),
    ("schema.published", "GET", "/schemas/published", "entity.read"),
    ("schema.drafts", "GET", "/schemas/drafts", "schema.draft"),
    ("schema.draft_get", "GET", "/schemas/drafts/{id}", "schema.draft"),
    ("schema.draft_create", "POST", "/schemas/drafts", "schema.draft"),
    ("schema.draft_update", "PUT", "/schemas/drafts/{id}", "schema.draft"),
    ("schema.draft_delete", "DELETE", "/schemas/drafts/{id}", "schema.draft"),
    ("schema.draft_validate", "POST", "/schemas/drafts/{id}/validate", "schema.draft"),
    ("schema.draft_impact", "POST", "/schemas/drafts/{id}/impact", "schema.draft"),
    ("schema.draft_publish", "POST", "/schemas/drafts/{id}/publish", "schema.publish"),
    ("schema.revision_get", "GET", "/schemas/revisions/{id}", "entity.read"),
    ("schema.revision_retire", "POST", "/schemas/revisions/{id}/retire", "schema.publish"),
    ("schema.revision_migration", "GET", "/schemas/revisions/{id}/migration-status", "entity.read"),
    ("requests.list", "GET", "/requests", "entity.read"),
    ("requests.create", "POST", "/requests", "entity.create"),
    ("workflows.get", "GET", "/workflows/{id}", "entity.read"),
    ("workflows.tasks", "GET", "/workflows/{id}/tasks", "entity.read"),
    ("workflows.create", "POST", "/workflows", "entity.create"),
    ("workflows.transition", "POST", "/workflows/{id}/transition", "entity.edit"),
    ("monitors.list", "GET", "/monitors", "monitor.manage"),
    ("monitors.run", "POST", "/monitors/{id}/run", "monitor.manage"),
    ("reports.list", "GET", "/reports", "report.view"),
    ("reports.generate", "POST", "/reports", "report.run"),
    ("connectors.list", "GET", "/connectors", "connector.manage"),
    ("connectors.get", "GET", "/connectors/{id}", "connector.manage"),
    ("connectors.update", "PATCH", "/connectors/{id}", "connector.manage"),
    ("connectors.delete", "DELETE", "/connectors/{id}", "connector.manage"),
    ("connectors.create", "POST", "/connectors", "connector.manage"),
    ("connectors.enable", "POST", "/connectors/{id}/enable", "connector.manage"),
    ("connectors.disable", "POST", "/connectors/{id}/disable", "connector.manage"),
    ("connectors.health", "POST", "/connectors/{id}/health", "connector.manage"),
    ("audit.query", "GET", "/audit", "audit.read"),
    ("audit.checkpoint_get", "GET", "/audit/checkpoint", "audit.read"),
    ("audit.checkpoint_create", "POST", "/audit/checkpoint", "security.manage"),
    ("audit.verify", "POST", "/audit/verify", "security.manage"),
    ("audit.retention_get", "GET", "/audit/retention", "audit.read"),
    ("audit.retention_set", "PUT", "/audit/retention", "security.manage"),
    ("permissions.groups", "GET", "/permissions/groups", "admin.pages.access"),
    ("permissions.group_create", "POST", "/permissions/groups", "security.manage"),
    ("permissions.group_update", "PUT", "/permissions/groups/{id}", "security.manage"),
    ("permissions.group_delete", "DELETE", "/permissions/groups/{id}", "security.manage"),
    ("users.role_groups", "GET", "/users/groups", "user.read"),
    ("users.list", "GET", "/users", "user.read"),
    ("users.get", "GET", "/users/{id}", "user.read"),
    ("users.create", "POST", "/users", "user.create"),
    ("users.update", "PUT", "/users/{id}", "user.edit"),
    ("users.delete", "DELETE", "/users/{id}", "user.delete"),
    ("users.activity", "GET", "/users/{id}/activity", "user.read"),
    ("users.tokens", "GET", "/users/{id}/tokens", "user.read"),
    ("users.token_create", "POST", "/users/{id}/tokens", "user.edit"),
    ("users.token_revoke", "DELETE", "/users/{id}/tokens/{token_id}", "user.edit"),
]

# PATCH /connectors/<id> exists on the API even though it wasn't in the
# rocket attribute grep — skr_pg mounts it; ensure a route node exists.
for name, method, path, cap in TOOLS:
    tid = "tool." + name
    N(tid, "tool", name, "/api/v1" + path,
      f"MCP tool → {method} /api/v1{path}.",
      method=method, cap=cap or "(session)", sort=700, tags="mcp,tool")
    E("mod.mcp.tools", "cm_contains", tid)
    rid = ROUTE_IDS.get((method, _norm("/api/v1" + path)))
    if rid:
        E(tid, "cm_maps_to", rid)


def CALL(subj, method, path):
    """UI/client → API route edge (path like /entities/{id} or /health/*)."""
    rid = ROUTE_IDS.get((method, _norm(path)))
    if rid is None and not path.startswith("/api/v1") \
            and not path.startswith("/health"):
        rid = ROUTE_IDS.get((method, _norm("/api/v1" + path)))
    if rid:
        E(subj, "cm_calls", rid)
    else:
        raise KeyError(f"no route for {method} {path}")


# =========================================================================
# Web client (web/) modules
# =========================================================================
N("mod.web.client", "module", "ApiClient.ts", "web/src/client/ApiClient.ts",
  "Typed fetch wrapper: envelope parse, X-Request-Id/X-Locale/X-Timezone, "
  "CSRF on mutations, If-Match, Idempotency-Key, AbortController timeout, "
  "401 refresh singleflight, ApiClientError.", sort=800, tags="typescript,client")
E("pkg.web-client", "cm_contains", "mod.web.client")

N("mod.web.types", "module", "types/* — DTO mirror", "web/src/types",
  "TS mirrors of Rust contracts: envelope, error codes, entity, query AST, "
  "schema, security, skr, view, audit, check, connector, file.",
  sort=801, tags="typescript,types")
E("pkg.web-client", "cm_contains", "mod.web.types")

WEB_ENDPOINTS = [
    ("mod.web.ep.entity", "EntityEndpoints.ts", "web/src/endpoints/EntityEndpoints.ts",
     "entities.*: create/list/get/query/relations/history/retire/merge/graph.",
     [("POST", "/entities"), ("GET", "/entities"), ("GET", "/entities/{id}"),
      ("POST", "/entities/query"), ("GET", "/entities/{id}/relations"),
      ("POST", "/entities/{id}/relations"),
      ("DELETE", "/entities/{id}/relations/{relation_id}"),
      ("PUT", "/entities/{id}"), ("GET", "/entities/{id}/history"),
      ("POST", "/entities/{id}/retire"), ("POST", "/entities/{id}/merge"),
      ("POST", "/entities/{id}/graph")]),
    ("mod.web.ep.schema", "SchemaEndpoints.ts", "web/src/endpoints/SchemaEndpoints.ts",
     "schemas.*: types, type views, drafts CRUD + lifecycle, published, "
     "revisions, predicates.",
     [("GET", "/schemas/types"), ("GET", "/schemas/types/{id}"),
      ("GET", "/schemas/types/{id}/view"), ("GET", "/schemas/drafts"),
      ("GET", "/schemas/drafts/{id}"), ("POST", "/schemas/drafts"),
      ("PUT", "/schemas/drafts/{id}"), ("DELETE", "/schemas/drafts/{id}"),
      ("POST", "/schemas/drafts/{id}/validate"),
      ("POST", "/schemas/drafts/{id}/impact"),
      ("POST", "/schemas/drafts/{id}/publish"),
      ("GET", "/schemas/published"), ("GET", "/schemas/revisions/{id}"),
      ("POST", "/schemas/revisions/{id}/retire"),
      ("GET", "/schemas/revisions/{id}/migration-status"),
      ("GET", "/schemas/predicates")]),
    ("mod.web.ep.checks", "CheckEndpoints.ts", "web/src/endpoints/CheckEndpoints.ts",
     "checks/assessments/factors/tasks/workflows: requestCheck, list/get, "
     "evidence, assessment get/create/trigger, factors, resolve, task "
     "lifecycle, workflow start/get/transition/tasks.",
     [("POST", "/checks"), ("GET", "/checks"), ("GET", "/checks/{id}"),
      ("GET", "/checks/{id}/evidence"), ("GET", "/entities/{id}/checks"),
      ("GET", "/assessments"), ("GET", "/entities/{id}/assessment"),
      ("POST", "/entities/{id}/assessment"),
      ("GET", "/entities/{id}/factors"),
      ("POST", "/entities/{id}/factors/{factor_id}/resolve"),
      ("POST", "/workflows"), ("GET", "/workflows/{id}"),
      ("POST", "/workflows/{id}/transition"),
      ("GET", "/workflows/{id}/tasks"), ("POST", "/tasks/{id}/complete"),
      ("POST", "/tasks/{id}/cancel"), ("POST", "/tasks/{id}/start"),
      ("GET", "/entities/{id}/tasks"), ("POST", "/entities/{id}/tasks")]),
    ("mod.web.ep.files", "FileEndpoints.ts", "web/src/endpoints/FileEndpoints.ts",
     "Staged upload (init/chunk/complete/abort/status) + file objects + "
     "entity documents.",
     [("POST", "/files/upload/init"),
      ("PUT", "/files/upload/{id}/chunks/{index}"),
      ("POST", "/files/upload/{id}/complete"),
      ("POST", "/files/upload/{id}/abort"), ("GET", "/files/upload/{id}"),
      ("GET", "/files/{id}"), ("DELETE", "/files/{id}"),
      ("GET", "/entities/{id}/documents"),
      ("POST", "/entities/{id}/documents"),
      ("GET", "/entities/{id}/documents/{doc_id}/download"),
      ("DELETE", "/entities/{id}/documents/{doc_id}")]),
    ("mod.web.ep.audit", "AuditEndpoints.ts", "web/src/endpoints/AuditEndpoints.ts",
     "audit.*: query, checkpoint get/create, verifyChain, retention "
     "get/set.",
     [("GET", "/audit"), ("GET", "/audit/checkpoint"),
      ("POST", "/audit/checkpoint"), ("POST", "/audit/verify"),
      ("GET", "/audit/retention"), ("PUT", "/audit/retention")]),
    ("mod.web.ep.security", "SecurityEndpoints.ts", "web/src/endpoints/SecurityEndpoints.ts",
     "permissions.*: group list/create/update/delete.",
     [("GET", "/permissions/groups"), ("POST", "/permissions/groups"),
      ("PUT", "/permissions/groups/{id}"),
      ("DELETE", "/permissions/groups/{id}")]),
    ("mod.web.ep.users", "UserEndpoints.ts", "web/src/endpoints/UserEndpoints.ts",
     "users.*: role groups, list/create/get/update/remove, activity, "
     "tokens list/create/revoke.",
     [("GET", "/users/groups"), ("GET", "/users"), ("POST", "/users"),
      ("GET", "/users/{id}"), ("PUT", "/users/{id}"),
      ("DELETE", "/users/{id}"), ("GET", "/users/{id}/activity"),
      ("GET", "/users/{id}/tokens"), ("POST", "/users/{id}/tokens"),
      ("DELETE", "/users/{id}/tokens/{token_id}")]),
    ("mod.web.ep.connectors", "ConnectorEndpoints.ts", "web/src/endpoints/ConnectorEndpoints.ts",
     "connectors.*: list/get/create/update/enable/disable/healthCheck/"
     "delete.",
     [("GET", "/connectors"), ("GET", "/connectors/{id}"),
      ("POST", "/connectors"), ("PATCH", "/connectors/{id}"),
      ("POST", "/connectors/{id}/enable"), ("POST", "/connectors/{id}/disable"),
      ("POST", "/connectors/{id}/health"), ("DELETE", "/connectors/{id}")]),
    ("mod.web.ep.monitors", "MonitorEndpoints.ts", "web/src/endpoints/MonitorEndpoints.ts",
     "monitors.*: list, run.",
     [("GET", "/monitors"), ("POST", "/monitors/{id}/run")]),
    ("mod.web.ep.reports", "ReportEndpoints.ts", "web/src/endpoints/ReportEndpoints.ts",
     "reports.*: list, create.",
     [("GET", "/reports"), ("POST", "/reports")]),
    ("mod.web.ep.requests", "RequestEndpoints.ts", "web/src/endpoints/RequestEndpoints.ts",
     "requests.*: list, create.",
     [("GET", "/requests"), ("POST", "/requests")]),
    ("mod.web.ep.dashboard", "DashboardEndpoints.ts", "web/src/endpoints/DashboardEndpoints.ts",
     "dashboard.stats.", [("GET", "/dashboard/stats")]),
    ("mod.web.ep.health", "HealthEndpoints.ts", "web/src/endpoints/HealthEndpoints.ts",
     "health live/ready probes.",
     [("GET", "/health/live"), ("GET", "/health/ready")]),
]
for eid, name, path, summ, calls in WEB_ENDPOINTS:
    N(eid, "module", name, path, summ, sort=810, tags="typescript,endpoints")
    E("pkg.web-client", "cm_contains", eid)
    for m, p in calls:
        CALL(eid, m, p)


# =========================================================================
# Nuxt UI — pages, components, composables, middleware
# =========================================================================
def PG(cm_id, name, path, summary, calls=(), uses=(), caps="", sort=900):
    N(cm_id, "page", name, path, summary, cap=caps, sort=sort,
      tags="nuxt,vue,page")
    E("pkg.nuxt-ui", "cm_contains", cm_id)
    for m, p in calls:
        CALL(cm_id, m, p)
    for u in uses:
        E(cm_id, "cm_uses", u)


PG("page.index", "GET / — dashboard", "nuxt/app/pages/index.vue",
   "Home dashboard: StatCard grid fed by /dashboard/stats (my tasks, "
   "overdue, new requests, checks in progress, critical factors, "
   "monitors due) + quick actions. mw: auth.",
   calls=[("GET", "/dashboard/stats")], uses=["cmp.StatCard"])
PG("page.login", "GET /login", "nuxt/app/pages/login.vue",
   "Login form (username+password) → authStore.login → POST /auth/login; "
   "redirect query honored. No auth middleware.",
   calls=[("POST", "/auth/login")])
PG("page.objects.index", "GET /objects — type catalog", "nuxt/app/pages/objects/index.vue",
   "Published types as cards (icon/display_name/count/status) → "
   "/objects/{typeAlias}. mw: auth + entity.read.",
   calls=[("GET", "/schemas/types")], caps="entity.read")
PG("page.objects.list", "GET /objects/{typeAlias} — entity list",
   "nuxt/app/pages/objects/[typeAlias]/index.vue",
   "Entity list per type: search + advanced where-AST filters via "
   "entities.query; type view drives columns; create button → /create. "
   "mw: auth + entity.read.",
   calls=[("POST", "/entities/query"), ("GET", "/schemas/types/{id}/view")],
   caps="entity.read")
PG("page.objects.create", "GET /objects/{typeAlias}/create",
   "nuxt/app/pages/objects/[typeAlias]/create.vue",
   "Entity creation: view-driven EntityFormRenderer + "
   "DuplicateSuggestionDialog (exact/fuzzy via entities.query, identity "
   "modes) + localStorage draft + unsaved-changes guard. mw: auth + "
   "entity.create.",
   calls=[("POST", "/entities"), ("POST", "/entities/query"),
          ("GET", "/schemas/types/{id}/view")],
   uses=["cmp.EntityFormRenderer", "cmp.DuplicateSuggestionDialog"],
   caps="entity.create")
PG("page.entities.id", "GET /entities/{publicId} — entity card",
   "nuxt/app/pages/entities/[publicId].vue",
   "Entity card: header (status/revision) + retire dialog + 8 tabs "
   "(overview, relations, documents, history, checks, risks, tasks, "
   "graph). mw: auth + entity.read.",
   calls=[("GET", "/entities/{id}"), ("POST", "/entities/{id}/retire")],
   uses=["cmp.EntityOverview", "cmp.RelationsTab", "cmp.DocumentsTab",
         "cmp.HistoryTab", "cmp.ChecksTab", "cmp.RisksTab", "cmp.TasksTab",
         "cmp.GraphView"], caps="entity.read")
PG("page.entities.edit", "GET /entities/{publicId}/edit",
   "nuxt/app/pages/entities/[publicId]/edit.vue",
   "Edit entity via EntityFormRenderer (initial values = current fields); "
   "revision concurrency. mw: auth + entity.edit.",
   calls=[("GET", "/entities/{id}"), ("PUT", "/entities/{id}"),
          ("GET", "/schemas/types/{id}/view")],
   uses=["cmp.EntityFormRenderer"], caps="entity.edit")
PG("page.admin.index", "GET /admin", "nuxt/app/pages/admin/index.vue",
   "Admin landing — links to schema/types/predicates/users/permissions. "
   "mw: auth + admin.pages.access.", caps="admin.pages.access")
PG("page.admin.schema", "GET /admin/schema — schema lifecycle",
   "nuxt/app/pages/admin/schema.vue",
   "Drafts + published tabs: create/edit drafts, validate, impact, "
   "publish, retire revision — full lifecycle UI. mw: auth + "
   "schema.draft.",
   calls=[("GET", "/schemas/drafts"), ("GET", "/schemas/drafts/{id}"),
          ("POST", "/schemas/drafts"), ("PUT", "/schemas/drafts/{id}"),
          ("DELETE", "/schemas/drafts/{id}"),
          ("POST", "/schemas/drafts/{id}/validate"),
          ("POST", "/schemas/drafts/{id}/impact"),
          ("POST", "/schemas/drafts/{id}/publish"),
          ("GET", "/schemas/published"), ("GET", "/schemas/revisions/{id}"),
          ("POST", "/schemas/revisions/{id}/retire")],
   caps="schema.draft")
PG("page.admin.types", "GET /admin/types", "nuxt/app/pages/admin/types.vue",
   "Type catalog admin: list + per-type view preview. mw: auth + "
   "admin.pages.access.",
   calls=[("GET", "/schemas/types"), ("GET", "/schemas/types/{id}/view")],
   caps="admin.pages.access")
PG("page.admin.predicates", "GET /admin/predicates",
   "nuxt/app/pages/admin/predicates.vue",
   "Predicate catalog admin. mw: auth + admin.pages.access.",
   calls=[("GET", "/schemas/predicates"), ("GET", "/schemas/types")],
   caps="admin.pages.access")
PG("page.admin.users", "GET /admin/users", "nuxt/app/pages/admin/users.vue",
   "User admin: list/create/edit/deactivate, groups, activity, API "
   "tokens. mw: auth + (users.manage|user.read).",
   calls=[("GET", "/users"), ("POST", "/users"), ("GET", "/users/{id}"),
          ("PUT", "/users/{id}"), ("DELETE", "/users/{id}"),
          ("GET", "/users/groups"), ("GET", "/users/{id}/activity"),
          ("GET", "/users/{id}/tokens"), ("POST", "/users/{id}/tokens"),
          ("DELETE", "/users/{id}/tokens/{token_id}")],
   caps="users.manage|user.read")
PG("page.admin.permissions", "GET /admin/permissions",
   "nuxt/app/pages/admin/permissions.vue",
   "Permission group editor (class data|capability, rules, principals). "
   "mw: auth + admin.pages.access.",
   calls=[("GET", "/permissions/groups"), ("POST", "/permissions/groups"),
          ("PUT", "/permissions/groups/{id}"),
          ("DELETE", "/permissions/groups/{id}")],
   caps="admin.pages.access")
PG("page.audit", "GET /audit", "nuxt/app/pages/audit/index.vue",
   "Audit explorer: filtered query, chain verify, checkpoints, retention "
   "policy editor. mw: auth + audit.read.",
   calls=[("GET", "/audit"), ("GET", "/audit/checkpoint"),
          ("POST", "/audit/checkpoint"), ("POST", "/audit/verify"),
          ("GET", "/audit/retention"), ("PUT", "/audit/retention")],
   caps="audit.read")
PG("page.checks.index", "GET /checks", "nuxt/app/pages/checks/index.vue",
   "Check instances list + request-check dialog. mw: auth + check.view.",
   calls=[("GET", "/checks"), ("POST", "/checks")], caps="check.view")
PG("page.checks.id", "GET /checks/{id}", "nuxt/app/pages/checks/[id].vue",
   "Check detail + evidence items. mw: auth + check.view.",
   calls=[("GET", "/checks/{id}"), ("GET", "/checks/{id}/evidence")],
   caps="check.view")
PG("page.docs", "GET /docs", "nuxt/app/pages/docs/index.vue",
   "In-app documentation page. mw: auth.")
PG("page.integrations", "GET /integrations — connectors",
   "nuxt/app/pages/integrations/index.vue",
   "Connector list + create/enable/disable/health actions. mw: auth + "
   "connector.manage.",
   calls=[("GET", "/connectors"), ("POST", "/connectors"),
          ("POST", "/connectors/{id}/enable"),
          ("POST", "/connectors/{id}/disable"),
          ("POST", "/connectors/{id}/health")],
   caps="connector.manage")
PG("page.monitoring", "GET /monitoring", "nuxt/app/pages/monitoring/index.vue",
   "Monitors table (frequency/next-run/last-result) + run-now + summary "
   "cards. mw: auth + monitor.manage.",
   calls=[("GET", "/monitors"), ("POST", "/monitors/{id}/run")],
   caps="monitor.manage")
PG("page.reports", "GET /reports", "nuxt/app/pages/reports/index.vue",
   "Report instances table + generate dialog (templates/formats). mw: "
   "auth + report.view.",
   calls=[("GET", "/reports"), ("POST", "/reports")], caps="report.view")
PG("page.requests", "GET /requests", "nuxt/app/pages/requests/index.vue",
   "Requests/processes list + create dialog. mw: auth + entity.read.",
   calls=[("GET", "/requests"), ("POST", "/requests")], caps="entity.read")
PG("page.risks", "GET /risks", "nuxt/app/pages/risks/index.vue",
   "Risk assessments catalog. mw: auth + risk.view.",
   calls=[("GET", "/assessments")], caps="risk.view")


def CP(cm_id, name, path, summary, calls=(), uses=(), sort=950):
    N(cm_id, "component", name, path, summary, sort=sort,
      tags="nuxt,vue,component")
    E("pkg.nuxt-ui", "cm_contains", cm_id)
    for m, p in calls:
        CALL(cm_id, m, p)
    for u in uses:
        E(cm_id, "cm_uses", u)


CP("cmp.AppHeader", "AppHeader.vue", "nuxt/app/components/AppHeader.vue",
   "Top bar: brand, workspace/user menu, locale switcher (8 locales), "
   "logout.")
CP("cmp.AppSidebar", "AppSidebar.vue", "nuxt/app/components/AppSidebar.vue",
   "Collapsible nav sidebar filtered by capabilities + URL rules; "
   "skip-link a11y.")
CP("cmp.shell", "app.vue + layouts + error.vue",
   "nuxt/app/{app.vue,layouts/*,error.vue}",
   "Default layout (header+sidebar+skip-link) and auth layout; global "
   "error page; app.vue primes auth context on mount.")
CP("cmp.StatCard", "StatCard.vue", "nuxt/app/components/common/StatCard.vue",
   "Dashboard stat card (label/value/icon/link).")
CP("cmp.EntityFormRenderer", "EntityFormRenderer.vue",
   "nuxt/app/components/entity/EntityFormRenderer.vue",
   "View-driven form: sections/collapsible, validation errors, "
   "unsaved-changes guard, save-draft; emits submit FormSubmission.",
   uses=["cmp.FormField", "cmp.MultiValueField"])
CP("cmp.FormField", "FormField.vue", "nuxt/app/components/entity/FormField.vue",
   "Field-kind registry (~22 kinds) → concrete input widgets.")
CP("cmp.MultiValueField", "MultiValueField.vue", "nuxt/app/components/entity/MultiValueField.vue",
   "Multi-value editor with min/max cardinality constraints.")
CP("cmp.EntityAutocomplete", "EntityAutocomplete.vue",
   "nuxt/app/components/entity/EntityAutocomplete.vue",
   "Entity picker with prefix search via entities.list.",
   calls=[("GET", "/entities"), ("GET", "/entities/{id}")])
CP("cmp.EntityListRenderer", "EntityListRenderer.vue",
   "nuxt/app/components/entity/EntityListRenderer.vue",
   "View-driven table: columns, filters, facets, sort, bulk selection, "
   "row actions.")
CP("cmp.EntityOverview", "EntityOverview.vue", "nuxt/app/components/entity/EntityOverview.vue",
   "Overview tab: fields rendered from the type's form view.",
   calls=[("GET", "/schemas/types/{id}/view")])
CP("cmp.DuplicateSuggestionDialog", "DuplicateSuggestionDialog.vue",
   "nuxt/app/components/entity/DuplicateSuggestionDialog.vue",
   "Dedup dialog: exact/fuzzy matches, identity mode handling "
   "(open existing | create distinct | create candidate).")
CP("cmp.RelationsTab", "RelationsTab.vue", "nuxt/app/components/entity/RelationsTab.vue",
   "Relations tab: sections by predicate, direction labels, "
   "effective/historical switch, add/link existing, qualifiers, "
   "validity, source, cursor pagination.",
   calls=[("GET", "/entities/{id}/relations"),
          ("POST", "/entities/{id}/relations"),
          ("DELETE", "/entities/{id}/relations/{relation_id}"),
          ("POST", "/entities/query"), ("GET", "/schemas/predicates"),
          ("GET", "/schemas/types")])
CP("cmp.DocumentsTab", "DocumentsTab.vue", "nuxt/app/components/entity/DocumentsTab.vue",
   "Documents tab: upload dialog (kinds/icons), qualifiers, validity, "
   "source, metadata, download/preview/remove.",
   calls=[("GET", "/entities/{id}/documents"),
          ("POST", "/entities/{id}/documents"),
          ("GET", "/entities/{id}/documents/{doc_id}/download"),
          ("DELETE", "/entities/{id}/documents/{doc_id}")])
CP("cmp.HistoryTab", "HistoryTab.vue", "nuxt/app/components/entity/HistoryTab.vue",
   "History tab: revision timeline with diffs, compare two revisions, "
   "reason display.", calls=[("GET", "/entities/{id}/history")])
CP("cmp.ChecksTab", "ChecksTab.vue", "nuxt/app/components/entity/ChecksTab.vue",
   "Checks tab: check instances w/ status icons, evidence items, "
   "step-up auth, data-denied states (8 kinds, 6 statuses).",
   calls=[("GET", "/entities/{id}/checks"), ("POST", "/checks")])
CP("cmp.RisksTab", "RisksTab.vue", "nuxt/app/components/entity/RisksTab.vue",
   "Risks tab: factors w/ level badges, assessment summary, decision/"
   "rationale, approval pending, resolve action, new-assessment dialog.",
   calls=[("GET", "/entities/{id}/assessment"),
          ("POST", "/entities/{id}/assessment"),
          ("POST", "/entities/{id}/factors/{factor_id}/resolve")],
   uses=["cmp.RiskLevelBadge"])
CP("cmp.RiskLevelBadge", "RiskLevelBadge.vue", "nuxt/app/components/entity/RiskLevelBadge.vue",
   "Risk level badge (4 levels + icons).")
CP("cmp.TasksTab", "TasksTab.vue", "nuxt/app/components/entity/TasksTab.vue",
   "Tasks tab: priority sort, blocking badge, overdue indicator, "
   "start/complete/cancel actions, create dialog (7 kinds, 6 statuses).",
   calls=[("GET", "/entities/{id}/tasks"), ("POST", "/entities/{id}/tasks"),
          ("POST", "/tasks/{id}/start"), ("POST", "/tasks/{id}/complete"),
          ("POST", "/tasks/{id}/cancel")])
CP("cmp.GraphView", "GraphView.vue", "nuxt/app/components/graph/GraphView.vue",
   "SVG force-directed bubble layout (d3-force v3): positions Map + tick "
   "ref (never node.x/y), forceManyBody+forceLink+forceCollide+BFS level "
   "columns, pinned nodes persist, >20 equivalent relations collapse to "
   "cluster:* virtual nodes, parallel-edge bowing, label hiding >30 "
   "edges, progressive expansion + cycle markers; tree alternative via "
   "GraphTreeNode. Data: POST /entities/{id}/graph.",
   calls=[("POST", "/entities/{id}/graph")], uses=["cmp.GraphTreeNode"],
   )
E("cmp.GraphView", "cm_implements", "kb.docs.17")
CP("cmp.GraphTreeNode", "GraphTreeNode.vue", "nuxt/app/components/graph/GraphTreeNode.vue",
   "Accessible tree fallback for GraphView (expandable nodes).")


def CO(cm_id, name, path, summary, calls=(), uses=(), sort=980):
    N(cm_id, "composable", name, path, summary, sort=sort,
      tags="nuxt,composable")
    E("pkg.nuxt-ui", "cm_contains", cm_id)
    for m, p in calls:
        CALL(cm_id, m, p)
    for u in uses:
        E(cm_id, "cm_uses", u)


CO("co.useApi", "useApi.ts", "nuxt/app/composables/useApi.ts",
   "Creates EnginerClient per call site: baseUrl /api/v1, locale, "
   "timezone, csrf from auth store; onRefresh → redirect /login. "
   "Components never call $fetch directly (docs/16 §4).",
   uses=["pkg.web-client"])
CO("co.useAuth", "useAuth.ts", "nuxt/app/composables/useAuth.ts",
   "Pinia store: user context (capabilities, ui_rules, csrf_token), "
   "fetchContext singleflight, login/logout, hasCapability, canUrl.",
   calls=[("GET", "/me/context"), ("POST", "/auth/login"),
          ("POST", "/auth/logout")])
CO("co.useCapability", "useCapability.ts", "nuxt/app/composables/useCapability.ts",
   "UX capability guard helper around the auth store (server still "
   "re-checks).")
CO("co.useSchemaMutation", "useSchemaMutation.ts", "nuxt/app/composables/useSchemaMutation.ts",
   "draft→update→validate→impact→publish helper for catalog mutations; "
   "no direct type CRUD — every change produces an auditable revision.",
   calls=[("POST", "/schemas/drafts"), ("GET", "/schemas/drafts/{id}"),
          ("PUT", "/schemas/drafts/{id}"),
          ("POST", "/schemas/drafts/{id}/validate"),
          ("POST", "/schemas/drafts/{id}/impact"),
          ("POST", "/schemas/drafts/{id}/publish")],
   uses=["co.useApi"])
CO("co.useStagedUpload", "useStagedUpload.ts", "nuxt/app/composables/useStagedUpload.ts",
   "Staged upload driver: hash (webcrypto), chunked parallel upload "
   "(1MB default), progress, abort, returns object_id.",
   calls=[("POST", "/files/upload/init"),
          ("PUT", "/files/upload/{id}/chunks/{index}"),
          ("POST", "/files/upload/{id}/complete"),
          ("POST", "/files/upload/{id}/abort"),
          ("GET", "/files/upload/{id}")],
   uses=["co.useApi"])

N("mw.auth", "middleware", "middleware/auth.ts", "nuxt/app/middleware/auth.ts",
  "Route guard: ensure auth store (singleflight fetchContext), redirect "
  "→/login, enforce ui_rules URL allow/deny (403).", sort=990,
  tags="nuxt,middleware")
E("pkg.nuxt-ui", "cm_contains", "mw.auth")
E("mw.auth", "cm_uses", "co.useAuth")
N("mw.capability", "middleware", "middleware/capability.ts",
  "nuxt/app/middleware/capability.ts",
  "capability(cap|[caps]) factory: any-of capability check → 403; UX "
  "only, server enforces.", sort=991, tags="nuxt,middleware")
E("pkg.nuxt-ui", "cm_contains", "mw.capability")
E("mw.capability", "cm_uses", "co.useAuth")


# =========================================================================
# Tests
# =========================================================================
def TS(cm_id, name, path, summary, subjects=(), sort=1000):
    N(cm_id, "test", name, path, summary, sort=sort, tags="tests")
    E("cm.repo", "cm_contains", cm_id)
    for s in subjects:
        E(cm_id, "cm_tests", s)


TS("test.api-pg", "pg_scenarios.rs — live-DB API scenarios",
   "apps/api/tests/pg_scenarios.rs",
   "20 live-DB scenarios via Rocket local Client + real PG "
   "(ENGINER_TEST_DATABASE_URL): auth+CSRF, where-AST query, identity "
   "dedup, revision conflicts, relation graph, full schema lifecycle, "
   "capability denial. Skipped without env.",
   ["app.api", "mod.api.entity_pg", "mod.api.workspace_routes",
    "mod.api.auth_routes"])
TS("test.api-ws", "workspace_scenarios.rs (ignored)",
   "apps/api/tests/workspace_scenarios.rs",
   "Legacy workspace scenarios — #[ignore]d, needs a DB-backed harness.",
   ["app.api"])
TS("test.mcp-tools", "tools_test.rs — MCP catalog unit tests",
   "apps/mcp/tests/tools_test.rs",
   "Tool-name uniqueness, path-shape/param coverage, prepare_call "
   "translation, capability filtering — no server needed.",
   ["app.mcp", "mod.mcp.tools", "mod.mcp.lib"])
TS("test.rust-units", "In-file #[cfg(test)] unit tests (~355 total)",
   "crates/*/src",
   "Workspace unit tests embedded in src files: domain invariants, "
   "query-engine compile/rewrite, application use cases via test_mocks, "
   "db-runtime, persistence, security-domain/pdp/api, schema-registry "
   "(19), contracts import (23), connectors (14), risk-engine (31), "
   "workflow (45), files (23), reporting (25), security (51), "
   "audit-persistence (22), integration (11).",
   ["crate.domain", "crate.query-engine", "crate.application",
    "crate.schema-registry", "crate.risk-engine", "crate.workflow",
    "crate.security"])
TS("test.contracts-import", "contracts import_test.rs",
   "crates/contracts/tests/import_test.rs",
   "CellParser/parse_cell coverage for the Excel import pipeline.",
   ["mod.contracts.import"])
TS("test.connectors-sdk", "connector_sdk.rs", "crates/connectors/tests/connector_sdk.rs",
   "Connector SDK contract tests.", ["crate.connectors"])
TS("test.files", "files_test.rs", "crates/files/tests/files_test.rs",
   "Staged upload + scan + storage tests.", ["crate.files"])
TS("test.reporting", "reporting_test.rs", "crates/reporting/tests/reporting_test.rs",
   "Dataset/definition/template/render tests.", ["crate.reporting"])
TS("test.risk-engine", "risk_engine.rs", "crates/risk-engine/tests/risk_engine.rs",
   "Criteria DSL evaluation + aggregation tests.", ["crate.risk-engine"])
TS("test.sr-bundle", "bundle_loader.rs + skr_bundle.rs",
   "crates/schema-registry/tests",
   "Bundle loader tests + SKR bundle structural integrity (refs, dup "
   "ids, edge/scalar invariants, identity predicates).",
   ["crate.schema-registry", "pkg.skr-core"])
TS("test.security", "security_test.rs", "crates/security/tests/security_test.rs",
   "enginer-security PDP/ABAC/audit tests.", ["crate.security"])
TS("test.workflow", "workflow_test.rs", "crates/workflow/tests/workflow_test.rs",
   "State machine/job/task/timer/reaction tests.", ["crate.workflow"])
TS("test.integration-flow", "check_flow_test.rs", "crates/integration/tests/check_flow_test.rs",
   "CheckFlowOrchestrator end-to-end.", ["crate.integration"])
TS("test.web-vitest", "web/tests Vitest (~68)", "web/tests",
   "ApiClient.test.ts (headers/401-refresh/errors), Endpoints, "
   "ErrorCodes, UserEndpoints, phase5-endpoints.", ["pkg.web-client"])
TS("test.nuxt-vitest", "nuxt/tests Vitest (~121)", "nuxt/tests",
   "StatCard, FormField(11), MultiValueField(7), "
   "DuplicateSuggestionDialog(7), EntityAutocomplete, "
   "EntityFormRenderer, GraphView, RiskLevelBadge, ErrorCodes, "
   "AuthAndUrl, i18n locale parity across 8 locales.",
   ["pkg.nuxt-ui", "cmp.FormField", "cmp.GraphView", "ui.i18n"])
TS("test.nuxt-e2e", "nuxt/e2e Playwright (~69)", "nuxt/e2e",
   "Persona matrix (admin/security_officer/manager/risk_analyst), auth, "
   "authz, accessibility, i18n, checks, audit, integrations, "
   "schema-admin, schema-types + graph_visual_check.mjs (zero bubble "
   "overlap assertion).", ["pkg.nuxt-ui", "cmp.GraphView"])
TS("test.mcp-suite", "docs/testing/mcp_test_runner.py (~1241 cases)",
   "docs/testing",
   "General live-MCP campaign against the docker stack; "
   "mcp_test_cases.md/report; PUML maps.", ["app.mcp", "app.api"])
TS("test.wh-suite", "wh_test_runner.py (1257 cases A..S,T,X)",
   "docs/testing/warehouse/wh_test_runner.py",
   "Warehouse-domain live-MCP campaign incl. generated negatives; "
   "wh_test_cases.md/report + wh_agent_prompts.md 100-prompt run.",
   ["app.mcp", "app.api"])
TS("test.kb-scripts", "KB provisioning scripts",
   "docs/testing/warehouse",
   "bootstrap.py (token provisioning → .wh_tokens.json), "
   "provision_v2.py (reference draft→publish), kb_provision.py + "
   "kb_docs_provision.py + this codemap_provision.py — all writes via "
   "MCP tools/call.", ["app.mcp"])


# =========================================================================
# Remaining contains/uses edges
# =========================================================================
for top in ("app.api", "app.mcp", "app.worker", "pkg.web-client",
            "pkg.nuxt-ui", "pkg.skr-core", "cfg.compose",
            "cfg.enginer-toml", "doc.agents", "ui.i18n"):
    E("cm.repo", "cm_contains", top)
E("cm.repo", "cm_implements", "kb.layout")
E("app.api", "cm_implements", "kb.docs.02")
E("app.api", "cm_implements", "kb.docs.06")
E("app.mcp", "cm_implements", "kb.docs.02")
E("app.worker", "cm_implements", "kb.docs.13")
E("pkg.web-client", "cm_implements", "kb.docs.16")
E("pkg.nuxt-ui", "cm_implements", "kb.docs.16")
E("pkg.nuxt-ui", "cm_implements", "kb.docs.17")
E("ui.i18n", "cm_implements", "kb.docs.19")
E("pkg.skr-core", "cm_implements", "kb.docs.03")
E("mw.auth", "cm_implements", "kb.docs.16")
E("co.useApi", "cm_implements", "kb.docs.16")
E("mod.api.workspace_routes", "cm_implements", "kb.docs.08")



