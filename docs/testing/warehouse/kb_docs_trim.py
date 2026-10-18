"""One-shot patch: compress over-limit kb_content strings in
kb_docs_provision.py so each text statement fits the
statement_active_fact_unique btree limit (~2.4KB practical).
"""
import re
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TARGET = os.path.join(HERE, "kb_docs_provision.py")
CAP = 2250  # bytes — safe below the ~2704 btree index-row limit

NEW = {
"kb.docs.02": """File: docs/02-system-architecture.md.

Style: modular monolith — enginer-api (Rocket: authn/authz +
commands/queries) + enginer-worker (jobs, connectors, projections,
reports, monitoring, reactions); shared domain/application crates.

Contexts: Identity, Security, Schema Registry, Entity Graph, Query,
Workflow, Connector, Risk, Reporting, Presentation, Audit.

Layers: Controller (routes/guards: limits, RequestContext, DTO, ONE
use case, typed error; no SQL/rules) -> Application (use cases own
transaction/idempotency ledger/security decision/outbox; ports are
traits) -> Domain (pure types+invariants) -> Persistence (sqlx ports;
parameterized SQL).

CreateEntity flow: guard (session, CSRF/Origin, size) ->
CommandEnvelope -> PEP (entity.create + data permission) -> published
schema revision -> domain normalize/validate -> identity keys -> ONE
tx (identity key + entity + unique statements) -> audit + outbox ->
idempotency ledger -> response (public id, revision, ETag). Same
request hash replays stored response; different payload same key -> 409.

Query flow: AST -> published ids/operators only -> PDP -> allowed
types/fields/links + row predicates + masks + limits -> intersect AST
-> parameterized SQL + keyset -> PG returns only allowed -> response
allowlist -> audit.

Runtime: Rocket 0.5 on Tokio vs sqlx async-std — AsyncStdDbExecutor
owns PgPool inside async-std, takes typed repo closures via bounded
channel, runs whole tx in one DB job, returns owned DTO; never moves
Transaction/connection/borrowed row; bounded queue/deadline/
parallelism + metrics.

Consistency: canonical write + outbox in one tx; worker drains outbox
FOR UPDATE SKIP LOCKED; projections/search eventual-consistent; API
returns commit_revision (min_revision supported).

Files: metadata/hash in PG, bytes in object storage; upload intent ->
limited/AV stream -> hash/MIME -> immutable blob -> authorized
short-lived download.

Network: browser sees Nuxt + /api/v1 only; internal/worker/metrics =
trusted network + mTLS; Nuxt never reaches DB/sources; connectors via
allowlisted egress; PostgreSQL unpublished. Scaling: stateless API;
worker queue leases; TenantLocator for dedicated shard.""",

"kb.docs.03": """File: docs/03-universal-domain-model.md.

All data: directed attributed multigraph; nodes = entities, edges +
properties = statements; types/fields/forms/rules/translations are
meta-level entities/statements. Unified logic, specialized storage.

Entity: EntityId bigint internal, PublicId UUIDv7; tenant_id,
entity_class (data|schema|policy|workflow|system), created/retired,
head_revision, canonical_entity_id after merge.

Statement: subject, predicate, object (Entity OR Scalar — exactly
one), validity (domain truth interval), recorded (system history),
provenance, confidence, status, revision. Revocation = new record/
status — the past is never erased.

PredicateDefinition: stable ID + ASCII alias, domain/range, directed/
symmetric/transitive/inverse, cardinality + uniqueness, required/
temporal mode, identity participation, normalization/validation,
index/search profile, confidentiality class, label/help translations,
non-semantic presentation hints. ИНН = scalar statement; "managed by"
= entity statement; typing = instance_of statement.

TypeDefinition: allowed/required predicates, composition/inheritance,
identity policies, validation rules, layout/search/report profiles,
lifecycle SM, published revision; an entity may carry several types.

Meta-kernel (closed): Entity, Statement, TypeDefinition,
PredicateDefinition, IDs, revisions, primitive scalar kinds, metaschema
rules — changing it needs a code/DB migration; app schema does not.

Scalar kinds: text + lang-tagged text, boolean, integer, decimal,
date/time/UTC instant/local datetime + IANA zone, UUID/opaque, URI/
e-mail/phone, money (decimal+ISO 4217), measurement, geo, schema-bound
JSON, immutable blob ref. f64 forbidden for money/shares/scores.

Provenance: connector/source record, fetch/effective times, raw
payload hash, legal basis/license, parser/mapping version, actor/job/
correlation, confidence, evidence. Sources never overwrite each other;
resolution policy forms effective view + keeps conflict.

ViewDefinition: root types, sections/fields, related collections,
columns/facets, computed values, actions, permission requirements,
translations — grants no access.

skr-core bundle: person, legal_entity, holding_view, address,
contact_point, document, license, request, check, risk_factor,
assessment, decision, task, contract, collateral; predicates like
has_tax_id/managed_by/founded_by/owns_share/affiliated_with/has_address/
checked_by; identity policies for ИНН/ОГРН/document/phone/source IDs;
bundle loaded via Admin API, not imported as a crate.

Holding: computed component over ownership/control/management/
affiliation at date+threshold; supports cycles/shares/history.

Invariants: exactly one object per statement; same-tenant subject/
predicate/object (except global schema refs); domain/range checked
against command's revision; active statement never changes in place;
canonical IDs acyclic; projection never sole store; canonical form
stored (+raw when needed).""",

"kb.docs.04": """File: docs/04-topology-unitarity-idempotency.md.

Unitarity: one constructor — statement "subject — predicate ->
object"; property vs relation differ only by object kind. Topology:
between published schemas a finite transformation path never
destroys canonical facts — inexpressible data kept as residual facts
(lineage reversible). Idempotency: f(f(x))=f(x) for commands, facts,
workers/imports, schema/projection migrations.

Transform primitives: AddType/RetireType/AliasType; AddPredicate/
RetirePredicate/RenameLabel; Add/RemoveMembership (temporal);
MapPredicate; SplitPredicate+residual; MergePredicates+lineage;
ConvertScalar (total parser or quarantine); ReifyEdge; Collapse-
RelationEntity+residual; MergeEntity/SplitEntity; MaterializeDerived/
DiscardProjection. Each: forward, optional reverse, pre/post-
conditions, mapping version, report.

Rename: aliases/labels change; type_id/predicate_id never do; display
name never a DB/API key.

IdentityPolicy: scope, key components, normalizer, quality, exact/
fuzzy mode, collision action, resurrection policy. Exact: ЮЛ =
country+regnum-type+ОГРН (alt country+verified ИНН); phone =
E.164+ext; document = country+type+series/number; external =
connector+namespace+source ID. Name/birthdate/address ->
same_as_candidate only.

Normalization: store raw_value + canonical_value +
normalizer_version; UTF-8 NFC + field trim (case kept); separate
casefold search key; ИНН/ОГРН digits+checksum; phone E.164/ext;
e-mail IDNA; decimal canonical scale; instant UTC + source zone;
symmetric edge = sorted endpoints. New normalizer -> migration +
collision report.

Fact fingerprint: tenant|canonical_subject|predicate|object_kind|
canonical_object|valid_from|valid_to|qualifier_hash|identity_scope.
Dedupe scopes: global_fact/per_source/per_observation. BLAKE2b-256
lookup; collisions decided on full canonical columns.

Idempotency-Key: unique tenant+principal+route+key; same payload ->
stored response; different -> 409; long call -> one job.

Concurrent create: ON CONFLICT gives one owner; second reads it —
same public ID, deduplicated=true. SELECT-before-INSERT without
unique constraint forbidden.

Merge: canonical picked, loser IDs -> aliases, statements resolved,
conflicts kept, keys transferred, EntityMerged mapping; repeat = no-
op. Split: plan distributes statements/keys + lineage anchor; repeat
plan = no-op. Worker inbox: unique consumer+event_id+handler_version.

Verifiable: normalize idempotent; repeat create = same ID; symmetric
edge endpoint order irrelevant; forward+reverse restores facts;
rebuild checksum stable; repeat merge silent; post-commit retry
returns stored; residual never unreachable.""",

"kb.docs.05": """File: docs/05-rust-workspace.md.

Layout: apps/{api,worker} + crates/{domain, application, contracts,
schema-registry, identity, query-engine, persistence-postgres,
db-runtime-async-std, security-domain, security-pdp, security-api,
connectors, risk-engine, workflow, reporting, files, observability,
test-support} + migrations/ + schema-packages/skr-core/ + web/ + docs/.
Dependencies point inward; domain never imports adapters.

Cargo: workspace-managed versions; Cargo.lock pins; default-features
off where they widen attack surface; cargo-deny; unsafe banned;
warnings denied in CI. Spec pins sqlx 0.5.13 async-std — DEVIATION:
repo uses sqlx 0.7.4 (see kb.conventions); rocket 0.5.1 json+secrets.

Required newtypes: TenantId, EntityId, PublicEntityId, TypeId,
PredicateId, SchemaRevisionId, StatementId, PolicyId, DecisionId,
PrincipalId, GroupId, SessionId, RequestId, CorrelationId,
IdempotencyKey, JobId, EventId, ProjectionRevision, LocaleTag,
TimeZoneId, ContentHash — never interchangeable String.

Ports: UnitOfWork/TransactionRunner; Entity/Statement/Schema/
IdentityKey repositories; QueryExecutor; SecurityDecisionPoint;
Outbox; AuditSink; Clock; IdGenerator; BlobStore; ConnectorInvoker;
JobQueue. Domain/application types only — no PgRow/rocket::Request/
serde_json::Value crossing.

DbExecutor: run(deadline, closure over PgPool handle); pool in
async-std; bounded queue + cancellation; closure never leaves
runtime; panic contained; graceful shutdown drains; readiness false
on DB-down/saturated queue.

RequestContext: tenant, principal, session, request/correlation IDs,
locale/timezone, auth strength, client metadata, deadline, policy
snapshot, purpose; guard-created, explicit, no globals.

Errors: DomainError, ApplicationError, SecurityError,
PersistenceError, ApiError (stable code+status+safe details). External
response never leaks SQL/stack/secrets/hidden existence.

Transactions: one boundary per use case; no external calls in tx;
lock order identity->entity->statement->outbox; retry only
serialization/deadlock + jitter/deadline; outbox for side effects;
READ COMMITTED (SERIALIZABLE only for proven race); optimistic
revision/If-Match.

Style: fmt+clippy -D warnings; no format!-SQL with user values;
secrecy/zeroize; Argon2id; UTC + explicit zone; decimal not float;
deny_unknown_fields on command DTOs.

Config: defaults -> file -> env -> secret provider; startup validates
DB URL/TLS/pool/keys/origins/storage/timeouts/connector refs/audit;
printed redacted. Feature flags: owner/expiry/default/audience/audit;
never a Cargo feature for tenant capability.""",

"kb.docs.06": """File: docs/06-api-contract.md.

Base: /api/v1 public; /internal/v1 + /internal/security/v1 internal.
HTTPS outside dev; JSON UTF-8; decimals as strings; instants RFC 3339
UTC; public IDs UUID (bigint never exposed); unknown command fields
rejected; X-Request-Id + X-Correlation-Id; OpenAPI generated from
contracts + snapshot-tested.

Envelope: {"data", "meta": {request_id, schema_revision,
commit_revision, policy_revision}, "links"}. Error: {"error": {code,
message_key, parameters, request_id}} — Nuxt localizes message_key;
code+parameters normative.

Statuses: 401 insufficient auth; 403 denied capability (hidden data
-> 404); 409 identity/version/idempotency; 412 bad If-Match; 422
validation; 429 quota+Retry-After; 503 dependency fail-closed.
DELETE = retire/revoke + tombstone; physical purge = separate
approval workflow.

Auth: OIDC AC+PKCE + HttpOnly SameSite cookie preferred; short bearer
for service accounts. POST /auth/session/exchange|refresh|logout;
GET /me, /me/context; POST /auth/step-up. Refresh rotation + reuse
detection; cookie mutations need CSRF/Origin.

Schema: GET /schemas/types{,/{id}}; POST /schemas/drafts; PUT
/schemas/drafts/{id}; POST .../validate|impact|publish; POST
/schemas/revisions/{id}/retire; GET .../migration-status. Publish =
If-Match + privilege + impact report + audit reason.

Entity: POST /entities; GET /entities/{public_id}; POST
/entities/query; PATCH; POST .../retire|restore|merge|split; GET
.../history|related; POST .../graph. Create: type_id, schema_revision,
identity_mode (return_existing), statements[](predicate_id + typed
{kind,value}) -> created/deduplicated + revision + warnings + actions.

Statements via entity command (cardinality+revision together); POST
/entities/{id}/statements; POST /statements/{id}/revoke|verify; GET
.../provenance. Never change predicate/subject/object in place.

Graph: as_of, max_depth, max_nodes, predicates, direction,
include_summaries; limits intersected with obligations; cycle =
one edge to visible node; signed opaque cursor. Query: POST
/entities/query; page 50 default/500 max UI; signed cursor; OFFSET
forbidden.

Mass ops: POST /jobs/imports|exports|bulk-mutations; GET/cancel/
errors/artifact — dry_run, snapshots, per-row results, retry, limits,
explicit partial success; browser never holds HTTP.

Checks/reports: POST /checks, /checks/{id}/run (202+job), POST/PATCH
/monitors, POST /reports, /reports/{id}/download.
Admin: /admin/users|service-accounts|capability-groups|data-groups|
policies|policy-simulations|connectors|translations|audit|jobs;
secrets write-only.

Concurrency: ETag "entity:uuid:revision"; If-Match or
expected_revision; 412 + safe summary; force update = capability +
reason. Caching: ETag binds revision+view+locale+policy scope;
private/no-store for secret data.

Events: /api/v1/events WS/SSE — permitted topics, field-filtered
payloads, policy change -> reauthorize/close, cursor reconnect,
bounded buffer+backpressure; events notify, client re-reads canon.

Limits: centralized LimitProfile — body size, statements/command,
filters, sort keys, page, graph depth/nodes, export rows/bytes,
upload bytes, concurrent jobs, deadline.""",

"kb.docs.07": """File: docs/07-query-engine.md.

AST: {from:{type_id,alias}, select:[{field,as}], where:{and:[...]},
sort:[{field|system,direction}], page:{size,after}, as_of} + exists/
related/aggregates/group_by/facets; joins only via relation path.

Operators per scalar kind: eq/ne/in/is_null; lt/lte/gt/gte/between;
contains/prefix/full_text; date overlap/valid_at; relation exists/
not_exists; geo within/distance. Regex/script/SQL fragment/
uncontrolled wildcard forbidden; contains on unindexed big text may
be cost-rejected.

Pipeline (fixed): parse+limits -> schema revision -> type-check ->
normalize literals -> capability -> PDP -> security AST rewrite ->
projection/index selection -> cost estimate -> parameterized SQL
compile -> deadline/cancel -> response allowlist -> audit. Never
compile user AST then string-append security WHERE.

PDP returns: allowed root types, select predicates, relation
predicates/directions, mandatory row predicates, masks, max page/
depth/export, forbidden aggregates/facets. Rewriter intersects
select, injects mandatory filters at root + every subquery, forbids
sort/facet on hidden fields, removes denied edges, caps summaries,
stamps plan policy revision. Fully denied root -> false/no-rows or
404.

SQL compiler: identifiers from trusted catalog; values always bind;
predicates grouped by storage family; current statements via partial
index/status; temporal adds valid range; every join carries tenant_id;
type membership before wide joins; EXISTS/LATERAL not random
DISTINCT; query tag = request/plan hash.

Keyset: sort ends with unique public_id/entity_id; cursor has query/
schema/policy fingerprints + last sort values + direction + tenant +
issued/expiry — signed; policy/schema change -> 409 cursor_stale.

Cost classes: interactive | extended | batch-only (->job) | rejected.
Estimated: predicates/paths, depth, selectivity, index fit, fan-out,
aggregate cardinality, page/export size.

Graph: recursive CTE only with max_depth/max_nodes + predicate
allowlist + tenant/temporal/security filters each step + visited-path
cycle detection + deterministic order + edge cursor + timeout;
frequent hierarchies -> closure projection.

Aggregates: separate permission (leak channel); filter/select/group
fields must be allowed; sensitive classes may need min group size/
rounding/denial.

Saved queries: AST + owner/scope/revision/migrations/presentation —
always under current policy. Plan cache: tenant + AST hash + schema
revision + policy shape + projection revision; invalidation on
publish.

Errors: query.syntax_invalid, unknown_type/predicate,
operator_not_supported, field_forbidden, too_complex, requires_batch,
cursor_invalid/stale, projection_lag, timeout.""",

"kb.docs.08": """File: docs/08-security-architecture.md.

Separation: security-domain/security-pdp/security-api crates; every
controller/use case/query/export/worker passes a PEP. Hot path:
LocalSecurityClient over the same JSON contract (no self-HTTP);
RemoteSecurityClient for split PDP.

Two right classes: system capabilities (login/use_ui, entity.create/
edit/retire/merge, query.advanced/aggregate, check.run/monitor.manage,
report.run/export, bulk.import/export/mutate, schema.draft/publish,
security.manage/simulate, connector.manage, audit.read,
admin.pages.access) — capability groups only. Data permissions
(tenant/namespace, type, entity, predicate field, relation
predicate/direction, statement/provenance, classification/tag/
ownership, row condition, historical period, purpose of use) — data
groups only.

Rule: effect, subject/group, action, resource selector, condition,
fields/relations, mask/obligation, priority, valid interval, policy
revision, reason/legal basis. Default deny; deny overrides allow;
break-glass = separate step-up flow, not a priority.

Evaluation: tenant/session/auth-strength -> snapshot memberships ->
capability -> applicable data rules -> drop false/expired ->
deny-overrides -> allowed set + obligations -> constrain -> decision
audit. Fail closed always — never "no rules -> allow".

Enforcement layers: router guard; application PEP; query PEP (type/
row/field/relation rewrite); repository (mandatory tenant+policy
predicates); PostgreSQL RLS; serializer allowlist; blob download
decision; worker/report/export (service identity + initiator
snapshot/recheck). UI visibility is UX, never a boundary.

Field policy: allow|omit|null-mask|partial-mask|tokenize|
aggregate-only; omit preferred. Edge permission checks predicate/
direction/endpoint types/context — denied edge can't be fetched,
traversed, or sorted-by.

RLS: separate DB roles (migration/API/worker); API role not owner,
no BYPASSRLS; tx-local tenant/principal/purpose settings; FORCE RLS;
connection reset before pool return. RLS never replaces PEP.

Authn: OIDC federation + optional emergency local admin; AC+PKCE;
MFA/step-up for publish/export/security/break-glass; Argon2id; short
access + rotating refresh + reuse detection; session/device/IP
signals; scoped service accounts + mTLS; rotating keys with kid.

Policy cache: key = tenant+principal+groups+policy revisions+auth
strength+purpose; compiled decision shape; membership/policy events
invalidate; short TTL; high-risk mutation may bypass.

Audit: authn, decision, denied request, policy change/simulation,
export, blob read, bulk, break-glass — actor/effective actor,
fingerprints, reason/policy IDs+revision, counts, tamper-evident
chain; no secrets, minimal PII.

Impersonation: capability + ticket/reason + timebox + banner +
original/effective principal + full audit; delegation = explicit
grant.

Break-glass: separate route, step-up MFA, reason/ticket, minimal
scope/TTL, owner notification, enhanced audit, tenant isolation/
export limits can't be disabled, auto-expiry + review.

Policy changes: draft -> validate -> simulate -> four-eyes ->
publish; rollback = new revision.

Fail modes: PDP down -> closed; audit down -> high-risk mutations/
exports closed (low-risk reads per policy); stale policy -> recheck
+ stale cursor; conservative clock.""",

"kb.docs.09": """File: docs/09-security-api.md.

Endpoints: POST /internal/security/v1/decisions/action|query|
mutation|blob; POST /internal/security/v1/simulations; GET
/internal/security/v1/policies/revision; POST
/internal/security/v1/cache/invalidate. Same contract for
LocalSecurityClient and mTLS internal HTTP; caller identity from
mTLS/token, delegated subject separate.

Context: request_id, tenant_id, principal {id, session_id,
auth_strength, groups_revision}, purpose, locale, time,
client_service. PDP never trusts client group lists — loads
membership or verifies signed snapshot.

Action decision -> {decision_id, effect, policy_revision,
obligations {max_rows, watermark, require_step_up, audit_level},
expires_at}.

Query decision request: action "entity.query" + query {
root_type_ids, selected_predicate_ids, relation_predicate_ids,
operations, requested_page_size, requested_graph_depth,
query_fingerprint} — normalized AST summary, never raw SQL.
Response: allow_with_obligations + {allowed_root_type_ids,
denied_type_ids, allowed_predicate_ids, denied_predicate_ids,
allowed_relations[{predicate_id, directions, target_type_ids}],
mandatory_filters{all:[{system_field, op, value|in_subject_scope}]},
masks, max_page_size, max_graph_depth, max_graph_nodes, aggregates}.
PEP must verify policy revision before cache/cursor.

Mutation decision: request = command + target type/entity + changed
predicates + relation targets + expected revision + batch size;
response = allow/deny + allowed predicates + required values/
approvals + forbidden targets + transforms + max batch + required
reason + audit level. Partially-forbidden -> denied wholesale;
partial mutation only via explicit endpoint with per-op results.

Row filters: typed security AST (distinct from user AST) referencing
only pre-registered system fields/scopes; PDP never returns SQL; one
trusted compiler. in_subject_scope via scope table/CTE; big ID lists
via scope handle/revision.

Masks: omit | null | fixed | partial(prefix/suffix) | hash/token |
bucket/range | aggregate_only — predicate + algorithm ID/version +
conditions; hidden fields forbidden in sort/filter (side channel).

Decision binding: tenant/principal/session/auth strength + action +
resource/query fingerprint + schema/policy/groups revisions +
purpose + expiry. decision_id is not a bearer permission.

Effects: allow | allow_with_obligations | deny | indeterminate
(->deny) | challenge (step-up). Codes: security.unauthenticated,
capability_denied, data_denied, step_up_required, policy_stale,
pdp_unavailable, context_invalid. Public API maps data_denied -> 404
where existence must hide.

Audit: PEP reports outcome (decision_id, rows/bytes, mutation
revision, status, latency, error class); outbox delivers later if
callback lost.

Hardening: mTLS + service allowlist, audience-bound short token,
network policy, body/depth/rate limits, replay protection, no CORS/
browser, safe logs, readiness on policy store, fuzz/contract tests.""",

"kb.docs.10": """File: docs/10-postgresql-schema.md.

Principles: PG = source of truth; sqlx only; DDL via migrations;
canonical normalized (JSONB only bounded metadata/raw); UUIDv7
public + bigint internal; tenant_id + RLS everywhere; timestamptz;
numeric for finance.

Schemas: core | meta | sec | ops | audit | integration | report.

core.tenant: id, alias, default_locale, default_time_zone,
data_region, status. meta: schema_revision (state draft/validating/
ready/published/retired/failed, parent, semantic_version,
content_hash, migration_plan; ONE current published via partial
unique), type_definition/type_revision/predicate_definition/
predicate_revision/type_predicate/localized_text.

core.entity: PK (tenant_id, entity_id); public_id unique; class;
created/retired; head_revision; canonical_entity_id; CHECK canonical
!= self. + entity_alias (merge losers) + entity_revision (append-only
per command).

Statement families (shared header): core.statement_edge|_text|
_number|_boolean|_date|_instant|_geo|_blob|_json. Header: tenant,
statement_id, public_id, subject_entity_id, predicate_id,
schema_revision_id, entity_revision, valid_from/to, recorded_at,
status (active/superseded/revoked/quarantined), provenance_set_id,
confidence, canonical_hash, qualifier_hash, supersedes/revoked_by.
Payload: edge = object_entity_id + canonical endpoints; text =
raw/nfc/search/language_tag; number = numeric + unit/currency; json =
value + schema hash + strict size. Active uniqueness fingerprint =
tenant+subject+predicate+canonical payload+validity+qualifier+scope.
NOTE: the active-fact unique index embeds the canonical object —
scalar values are bounded to ~2.5KB by the btree row limit (learned
live: index row >2704 fails 500).

core.entity_type_current: hot membership projection. Partial
status=active indexes on families; optional header projection —
no universal current copy.

core.identity_policy; core.identity_key (tenant+type+policy+key_hash
unique active; canonical_key maybe encrypted; state; quality);
core.match_candidate (pair+algorithm+score+status — never a merge).

integration.source_system / source_record / provenance_set/member.

ops.entity_summary (per locale, safe fields, revisions — rebuildable);
ops.entity_search (tokens + tsvector + optional trigram; sensitive
shards split); ops.typed_value_index (approved index profiles);
ops.projection_catalog/checkpoint (watermark, build generation,
checksum, atomic switch). Hot materialized read models allowed but
fully rebuildable.

sec.*: principal/user_identity/session/credential; capability_group/
data_group + membership tables (valid interval/revision); policy/
policy_revision/policy_rule; subject_scope_membership; decision_log/
outcome; break_glass_grant. Secrets ciphertext separate + key ID.

ops.idempotency_record (tenant+principal+route+key unique);
ops.outbox_event (versioned payload, lease, attempts);
ops.inbox_delivery (consumer+event+handler unique); ops.job (state,
progress, snapshots, lease, cursor, artifacts); ops.job_item.

audit.event: append-only, monthly RANGE; event/actor/action/decision/
fingerprints/hashes/checkpoint; UPDATE/DELETE forbidden.

core.blob + blob_reference; report.definition/revision/run/artifact.

Partitioning: statements HASH by tenant bucket (32/64); audit/outbox/
observations RANGE monthly; partition key in unique constraints.

Index catalog: entity tenant+public_id/canonical; entity_type
tenant+type+entity; statements tenant+subject+predicate active; edge
tenant+object+predicate; typed value; identity unique; GIN tsvector;
outbox/job/audit. Every index documents owner/selectivity/cost — no
catch-all JSONB index.

RLS: tx-start set_config(...,true) tenant/principal/purpose/policy;
rejects absence; reset after tx; pooled leak tested. Migrations:
sqlx migrate as separate job before readiness; expand/contract +
resumable backfill; app never runs DDL from a request.""",

"kb.docs.11": """File: docs/11-performance-and-scaling.md.

Profiles: dev 100K | integration 10M | scale 100M active+history |
extreme 500M+ statements — generator reproduces skew (hot types,
high-degree nodes, long texts, tenant imbalance, history).

SLO histograms: exact lookup, card GET, list first/next, full-text,
relation list, graph d1/2/3, create/update, policy decision/rewrite,
job enqueue, projection lag — P50/P95/P99 + throughput + errors +
rows examined/returned + DB/queue/pool wait.

Budgets: workers != connections; bounded DbExecutor queue; pool from
DB max/replicas/instances; acquire timeout < request deadline; batch
on own pool; no connection during external HTTP/render/user wait;
cancellation propagates; tx-local statement_timeout.

Query discipline: keyset only; selective type/tenant filter first;
EXISTS/LATERAL over exploding joins; project only requested
predicates; fan-out limits; precomputed summaries; bounded IN (big
scopes via temp tables); no SELECT *; no functions on indexed
columns; plan regression snapshots.

Point reads: identity_key unique -> summary + current statements;
card loads predicates batched per family (no N+1); related counts
aggregated/lazy.

Export job: pin query/schema/policy snapshot; keyset chunks; stream
file; per-chunk or bound-snapshot policy; byte/time limits; artifact;
no giant tx; PG exported snapshot only controlled+max-aged.

FTS: separate entity_search per locale/profile; exact IDs via
B-tree; trigram only for proven fuzzy; sensitive text never in
shared vector.

Graph: depth-1 adjacency B-tree; hierarchies via closure projection;
arbitrary = recursive CTE + budget; high-degree paged; cache key =
entity revision + predicate set + as_of + policy shape; deny enforced
each step.

Partitions: hash for parallelism/maintenance, never replaces tenant
filter; count by benchmark; partition key in unique constraints;
queries carry tenant; default partition monitored/forbidden; future
partitions precreated; detach/archive runbook; too-fine forbidden.

Projection lifecycle: draft -> building gen N -> verified -> active
-> retire old. Verify: bucket row/count/checksum + watermark +
sampled canonical compare + classification + plan benchmark. Read-
after-write waits watermark or canonical fallback.

Cache: L1 bounded per-process (immutable-by-revision); distributed
optional for summaries; key = tenant+revision+locale+policy shape;
short non-shared negative cache; event invalidation + TTL;
singleflight for stampedes.

Maintenance: per-table autovacuum, measured fillfactor, bloat/WAL/
replica monitoring, ANALYZE on skew, extended stats, REINDEX
CONCURRENTLY, pg_stat_statements, no idle-in-transaction.

Write amplification: every index/projection priced (insert latency/
WAL/storage); IndexProfile per predicate via online job; unused index
removed after review.

Sharding: partitions/replicas/projections/hardware first; then
TenantLocator; tenant never split initially; cross-tenant analytics
in separate warehouse.

Backpressure: per-tenant request/bulk limits; bounded queues;
connector quotas; 429 quota/503 saturation; adaptive disable;
circuit breakers.

Gates (release blockers): critical P95 regression >10% without ADR;
unapproved seq scan on big family; rows examined multiplication;
post-fetch policy filtering; projection lag over SLO; migration lock
over budget; duplicates or tenant leak in load test.""",

"kb.docs.12": """File: docs/12-schema-evolution.md.

States: draft -> validating -> impact_ready -> approved -> publishing
-> published -> retired/failed. Published immutable; fix = new
revision; one current per package/tenant.

Draft validation: aliases/IDs/translations, domain/range/cardinality,
inheritance cycles, inverse/symmetric coherence, identity
normalizers, migration totality, index/search profiles, security
classification, view/report/workflow refs, confusable aliases, cost.

Compatibility classes: presentation-only | additive | conditionally
compatible (required field w/ backfill) | semantic | breaking |
meta-kernel. Consumers get compatibility report + deadline.

Publish plan: freeze hash -> simulate on prod stats (no rows) ->
build indexes/projection concurrently -> deploy old+new reader ->
resumable-chunk backfill -> quarantine invalid -> verify counts/
checksums/invariants -> atomic activation -> SchemaPublished ->
retire old write path after observation window.

Expand/contract: never remove old predicate while requiring new —
add/dual-read or map, backfill, switch writes, verify, retire last.

MigrationPlan: source/target revisions, ordered primitive steps,
deterministic function/normalizer versions, pre/postconditions,
conflict/quarantine policy, chunk key/size, retry/idempotency key,
expected counts, reverse/compensation, index/projection deps, owner/
approval. No user code — versioned safe registry/DSL.

Required-field strategies: deterministic default from facts;
connector enrichment + review; manual task before activation;
required_for_new_only + legacy marker; quarantine/read-only. Fake
"unknown" forbidden — model absence or explicit semantic value.

Scalar conversion: converted|ambiguous|invalid|overflow; raw kept;
only converted published; rest -> migration issues w/ row context
without halting the resumable job.

Identity normalizer change: shadow-table keys -> collision/split
report -> classify auto-safe/manual/blocking -> review -> atomic
policy-version switch -> old keys kept as aliases; never silent
auto-merge.

Split/merge predicate: source kept, targets + mapping created;
conflicts -> effective view marks conflict, not last-write-wins.

Reify edge: A —owns-> B -> relation entity R (owner_role/owned_role +
share/period/source facts); R links original statement; reverse only
if qualifiers expressible (else residual).

Rollback: compensating revision + projection-generation switch;
facts never deleted; external side effects get compensation workflow.

Concurrent writes: validate against current revision + event; dual
projector applies to building generation; cutover when watermark +
catch-up equal.

API migration: DTO kept to deprecation; server maps aliases/old
predicate IDs via compatibility registry; stable IDs beat versioned
field names.

Acceptance: round-trip preserves canonical/residual facts; repeat
migration no dupes; resume at chunk boundaries; concurrent write
visible after cutover; invalid data reported not coerced; old API
works in promised window; policies mapped before activation.""",

"kb.docs.13": """File: docs/13-events-jobs-and-automation.md.

Domain events (facts about commits): Entity*/Statement*/Schema/
Policy/Check/RiskFactor/Assessment/Task/Monitor/Report/Projection
created/changed/etc. Envelope: event_id UUIDv7, type/version, tenant,
aggregate/public ID + revision, occurred/recorded, actor, request/
correlation/causation, classification, payload. Minimal PII.

Outbox: mutation + outbox in one tx; dispatcher SELECT FOR UPDATE
SKIP LOCKED, lease, publish, attempt/error, backoff+jitter, dead-
letter at threshold, published_at after ack. No exactly-once —
consumer effects idempotent.

Inbox: consumer+event+version registered first; completed replays
prior outcome; lease stealable after expiry; mark completed only
after local effect/outbox committed.

Job SM: queued -> leased -> running -> waiting_external/waiting_user
-> succeeded/partially_succeeded/failed/cancelled. Progress num/denom,
phase, heartbeat, attempts, deadline, lease, resumable cursor,
artifacts; cooperative cancellation + cannot_cancel_after.

Worker classes: projections, imports/exports, connectors/checks,
risk/workflow, reports, notifications, maintenance — own
concurrency/pool/priority so bulk can't starve critical projection.

Scheduler: IANA zone + calendar expression + misfire/concurrency
policies; DST nonexistent -> skip/run-next, repeated -> once/twice;
uniqueness schedule+occurrence; leader/lease single enqueue.

WorkflowDefinition: states/transitions, trigger (event/manual/
schedule), safe-DSL conditions, required capabilities/scope, actions,
timers/SLA, assignment, escalation, compensation, version. Instances
stay on pinned version or take explicit migration.

Check scenario: request -> identity resolve -> validation -> task ->
connector jobs -> provenance statements -> risk recompute -> risk
task -> decision entity -> notifications -> monitor schedule ->
report job. All in timeline + audit.

MonitorDefinition: target, sources, interval, detector, criteria,
quiet period, notification/workflow, legal basis. Run: pin snapshots
-> observations -> canonical normalized compare -> noise suppression
-> ChangeDetected -> risk/workflow -> next_run. Source error !=
absence of risk; staleness is a factor.

Reaction registry: create task/case, change category, start check/
report, send e-mail/webhook/message, notify external, request
approval, escalate — each with schema/permission/idempotency/timeout/
retry/compensation. Arbitrary shell forbidden; on-prem scripts only
via sandboxed connector runner.

Notifications: versioned template, locale, channel, recipient
resolution, classification, redaction; delivery ledger prevents
resend; no secrets in subject/push.

SLA: versioned business calendar per tenant/location; due instant
computed once + stored; calendar edits never rewrite old deadlines.

Failures: transient|rate_limited|auth|validation|permanent|
policy_denied|cancelled. Retry only transient/rate_limited; auth
blocks connector; service account never bypasses policy_denied.

Ops UI: queues/lag/leases/attempts/dead letters/source health/
watermark/job details; retry needs capability + keeps original id.""",

"kb.docs.14": """File: docs/14-connectors-and-risk-engine.md.

Connector SDK: descriptor/capabilities, credential validation,
health/quota, search/fetch/monitor ops, versioned schemas,
normalization mapping, rate limit/retry, legal/licensing metadata,
redacted diagnostics. Core never knows concrete API fields —
connector returns SourceObservation + mapping result.

Required sources (СКР): internal DB, АСУ, Евразия, Контур.Фокус, ФНС,
ФССП, ВАС/successor, ФМС/successor, bad-supplier registry, Вестник
госрегистрации, Росздравнадзор, open Internet, social. Each adapter
fixes owner/endpoint/contract/license/jurisdiction/retention/purposes
first — legacy name != API availability.

ConnectorInstance: type/version, enabled ops, environment/base URL/
proxy, secret refs, org account, quotas, timeout/retry, cache/
freshness, legal basis/purpose, permitted target types/fields,
retention/raw policy, trust rank, mapping version. Secrets
runner-only, never returned by Admin API.

Pipeline: PEP (check.run + field access) -> minimal input projection
-> normalized request + remote idempotency key -> quota reserve ->
call (timeout/circuit breaker) -> raw to encrypted blob/hash ->
parser observations -> mapper + provenance -> identity resolution
(exact/candidate) -> risk event -> audit (no secret payload).

File import: connector op file_import; per-row external ID/
fingerprint; preview/conflicts/target predicates; never overwrites
user values — new source-specific assertion + effective resolution.

Trust/resolution per predicate: manual_verified first; authoritative
rank; freshest-in-class; consensus; keep_all+conflict; derived
formula. Effective value = explained projection; assertions
immutable; high-impact auto-change may need approval.

CheckDefinition: target types, required inputs, connector ops,
deps, freshness/cache, criteria package, artifacts, timeout/SLA,
retry, assignment, purpose, report profile. CheckRun pins
definition/schema/policy/connector versions.

Risk DSL (no arbitrary code): compare/range/existence/count;
temporal age/duration; string exact/normalized set; relation exists/
path (bounded); source freshness/trust/conflict; decimal arithmetic;
boolean all/any/not; dictionary lookup; previous assessment change.
Criterion: ID/version, inputs, severity, weight, explanation key,
evidence selector, interval, owner, tests.

RiskFactor: criterion/version, target, detected/resolved, severity/
contribution, status, evidence refs, explanation, suppress/override
(actor/reason/expiry). Re-evaluation upserts by target+criterion+
evidence window.

Assessment: target/as_of, model/version, input watermark/fingerprint,
factor refs, component scores, final decimal score/category,
uncertainty/freshness, actors, status draft/approved/superseded,
explanation. Score != decision — Decision separate.

ЛТК/отсрочка/ОДЗ/ПДЗ = package financial predicates/entities
(currency, as_of, source); risk model references by ID.

Auto-reactions: threshold -> task/alert/report/recheck/notify;
high-impact change/block needs workflow approval; cooldown + dedup
key mandatory.

Internet/social class: allowlisted providers/queries, ToS/legal,
minimization, provenance URL+time, hash/snippet limits, human
verification, injection defense, mention != fact.

Adapter tests: contract fixtures, sandbox, drift, rate/timeout/
retry, malformed/oversized, Unicode, duplicates, legal flags,
redaction, reconciliation.""",

"kb.docs.15": """File: docs/15-import-export-and-reporting.md.

Heavy op = job; upload/download separate from run; input immutable +
hashed; mapping/definition/schema/policy versions pinned; results
reproducible + audited.

Formats: import XLSX/CSV/TSV/NDJSON/JSON package; export XLSX/CSV/
NDJSON/JSON/DOCX/PDF; legacy XLS via isolated converter only. Files
untrusted: size/zip/formula/macro/external-link/MIME + AV.

Import pipeline: upload+scan -> structure inspect -> mapping profile
-> preview+sample -> dry run stats -> approval for mass change ->
chunked per-row-idempotent execution -> reconciliation -> error
workbook/JSON -> audit + retention.

MappingProfile: sheet/range/headers, column->predicate/path,
type/locale/date/decimal parsing, normalizers, identity policy,
relation creation, provenance, empty/null semantics, default/lookup,
conflict action, formula handling, row filter, version/tests. Empty
cell != delete — explicit marker required.

ЮЛ Excel normative mapping: request fields -> check_request entity;
ОПФ/name/brand -> legal_entity statements; ИНН/ОГРН -> identity
statements/keys; ОКВЭД -> classifier relation + interval; capital ->
money + as_of; dates/registrar -> registration statements; addresses
-> address entity + typed relation; license -> license entity;
www/email/phone -> contact entities; manager -> person + management
relation + period/source; founder/share -> ownership relation entity;
formal/informal affiliation -> distinct predicates + evidence.

Dedupe: row key = file hash + mapping version + sheet + row + op;
semantic identity across files; repeated job -> prior row outcomes;
fuzzy -> review queue.

Formulas: computed value + formula/provenance kept; never executed;
external links/macros forbidden; normative formula workbooks via
controlled recalculation service.

ExportDefinition: AST, columns/paths, translations, formats, sorting,
template, limits, classification. Runtime: reauthorize -> snapshot ->
stream chunks -> masks before writer -> escape =, +, -, @ prefixes
-> watermark -> optional encrypt/sign -> artifact TTL.

ReportDefinition: dataset query/params, sections/tables/charts/
summaries, locale/timezone, template version, field formatting,
confidentiality, page/row limits, audiences/formats, snapshot,
schedule. Template: no SQL, no undeclared object paths.

Mandatory SKR reports: ФЛ/ЮЛ dossiers, holding composition, check
result+evidence, checks by day/month, employee/decision/unit stats,
graphical stats, rejection stats, payment discipline/limits/
deferments, collateral + recheck monitor, usage + task SLA.

Collateral statuses (rule version): >30d | overdue | <=30d |
unassigned | possibly-closed; color = presentation token.

Security: current authorization per chunk OR approved snapshot grant
(TTL/definition/fingerprint); schedule = own narrow grant + recipient
check; hidden fields can't reach template.

Delivery: short-lived download authz, Range optional, checksum,
no-cache; e-mail = link; attachment only encrypted/approved; ledger.

Retention: classification-based TTL/legal hold; recoverable/crypto
removal; metadata/audit kept.

Acceptance: round-trip IDs/Unicode/decimal/date; no formula
injection; no hidden field in any artifact; 100k/1M flat memory;
duplicate job -> no dupes; localized row errors; golden DOCX/XLSX/
PDF.""",

"kb.docs.16": """File: docs/16-nuxt-architecture.md.

Stack: Nuxt 4, Vue 3 Composition API, TS strict, Nuxt UI 4, i18n,
Pinia or useState (one choice), generated typed API client from
OpenAPI, Vitest+VTU+Playwright, axe-core; lockfile-pinned.

Rendering: authenticated = SPA (ssr:false — no SSR/cache leakage,
simpler WS/session); public help/login may SSR with own cache
policy. Nuxt server holds no business state, never touches PG; Nitro
proxy only manages cookies/headers/CSRF -> Rocket.

Structure: app/{pages,layouts,components/{entity,graph,query,
workflow,admin},composables,stores,middleware,plugins,assets,
error.vue} + layers/{core,skr} + generated/api + i18n/locales + tests.

API client (only wrapper): base URL+credentials, request-ID/locale/
timezone, CSRF header, timeout/AbortController, 401 refresh
singleflight, typed 403/404/409/412, ETag/If-Match, idempotency
keys, cursor, no mutation retry without key, redacted diagnostics.
Components never call $fetch directly.

Session: GET /me/context -> profile, locale/tz, menu capabilities,
workspaces, policy revision, presentation grants. Middleware checks
capability for UX — server re-decides. PolicyChanged: invalidate,
cancel queries, refetch, redirect. No tokens in browser storage;
sensitive drafts not in localStorage.

Metadata-driven UI: EntityFormRenderer (type/predicate revision +
ViewDefinition + values + field permissions + locale + validation);
field registry: text/textarea/rich(sanitized), boolean, integer/
decimal/money/measurement, date/time/datetime/timezone, enum/
dictionary, entity ref/autocomplete, multi-value/repeating, address/
contact/document, file, derived, relation editor — unknown kind ->
safe unsupported component, never raw JSON. EntityListRenderer:
columns/filters/facets/sort/actions/export from view; virtualization
+ cursor pagination; column prefs store IDs.

State: cache key = tenant+resource+revision+view+locale+policy;
draft separate; optimistic only reversible low-risk; 412 -> conflict
UI; jobs via events+fallback; graph keyed by ID + cursors + visited;
never park huge results in store.

Layout: header (workspace, search, jobs/notifications, locale,
profile), collapsible nav by capabilities+schema menu, breadcrumbs,
context panel, command palette, environment banner. Tenant brand
tokens configurable; legacy red theme not copied.

Search: exact-ID mode + full-text + filters + recent/saved +
keyboard nav + permission-filtered highlights + debounced
cancellable + no classified text in analytics. Advanced search
builds AST visually; batch-only/forbidden shown with reason.

Performance: lazy routes/components, split admin/report/graph,
cursor/virtual lists, summary-first + lazy tabs, progressive graph,
no huge client-side filtering, no N+1, CI budgets.

Validation: client = UX only; server authoritative; errors bound to
predicate IDs; draft survives 422/412; decimal string until submit.

A11y WCAG 2.2 AA: keyboard/focus, skip links, landmarks, label
associations, dialog focus, table headers, accessible graph tree,
color not sole carrier, reduced-motion, 200% zoom, RTL, SR
announcements.

Client security: CSP no unsafe-eval, sanitize source content, no
v-html, rel=noopener, API downloads, sandboxed preview, no secrets
in runtimeConfig.public, SCA, clickjacking protection, auto-lock,
reauth for high-risk.

Errors: error.vue distinguishes offline/auth/forbidden/stale/
conflict/rate/degraded + request ID; no stack/raw payload.""",

"kb.docs.17": """File: docs/17-ui-screen-specification.md.

Menu from schema packages + capabilities: Главная, Объекты, Заявки и
процессы, Проверки, Мониторинг, Риски и решения, Отчёты,
Документация, Администрирование, Интеграции, Аудит. Legacy menu items
become saved views/types, not code.

Dashboard: my tasks/overdue, new requests, checks, critical factors,
monitors due, reports/jobs, scoped stats, source health — all
aggregate-leak-safe, with as_of/freshness + drill-down.

Catalog /objects/[typeAlias]: title, saved views, simple/advanced
search, filters/facets, columns/sort, cursor table, bulk selection
via server-side token, actions by capability, status/freshness,
state handling.

Card /entities/[publicId]: header (localized name + type badges,
lifecycle/risk/classification, IDs by permission, freshness/conflict,
actions, revision/as_of). Tabs: Обзор | Связи | Граф/дерево |
Проверки | Риски | Процессы/задачи | Документы | История/provenance.
Hidden tabs leave no headers/counts.

Create/edit: schema-revision form, required/conditional, repeating
values/relations, exact ref search, duplicate suggestion pre-commit,
raw/canonical preview for identity fields, draft save, server
validation, create-and-open/add-related, unsaved guard, expected
revision. Exact dup -> open existing or explicit distinct; fuzzy ->
candidate list, never auto-merge; hidden entity -> generic conflict.

Related sections: per-predicate groups; items with qualifiers/
validity/source; add/link; safe inline edit; counts/cursors;
direction labels; effective/historical switch.

Graph: toolbar (as_of, depth, predicates, direction, layout, hide
resolved, export by permission); start depth 1; expand on demand;
Security-API budget; cycle = link-back; edge label/period/source;
high-degree paged; hidden node not inferable; accessible table/tree.

Global search: exact/text toggle, type chips, external = check job,
policy-bound recents, safe highlights, hidden types absent.

Check request: new/existing, initiator, category, target/create,
product/limit/deferment, purpose/urgency, documents, due, profile;
timeline = assignment/sources/factors/decisions/SLA.

Check screen: target summary, definition/version/status, source
states (error != "nothing found"), grouped results, conflicts/
provenance, criteria/factors, notes/evidence, run/retry, approve/
reject/request-info, report.

Risks/decision: factors (severity, criterion, age, evidence,
explanation); assessment (score/category/model/freshness); decision
form (outcome, reasons, conditions, limit/deferment, validity,
approvals); override = capability/reason/expiry.

Reports: categories (ФЛ/ЮЛ checks, time/employee/decision stats,
graphical, rejections, collateral); params scope/dates/employee/
product/branch/decision/optional/refresh/format/locale; async run
with estimate + progress.

Collateral monitor: filters (client/ИНН, product, contract, type/
status, responsible, ranges, overdue, closed); statuses >30d/overdue/
<=30d/unassigned/possibly-closed with icon/text/token.

Personas: security service (checks, evidence, sources, factors,
affiliations); manager (requests, limits, discipline, ОДЗ/ПДЗ; no
closed person fields); risk officer (ЛТК, deferment, assessment,
factors, approvals). Combined roles = intersected/unioned policies
+ deny-overrides.

Jobs drawer + deduped notifications + deep links. Responsive:
desktop-first; narrow -> drawer nav + selected columns; 35-col Excel
never just shrunk.

Acceptance: legacy scenarios reachable; type/view live after publish;
hidden data absent from DOM/network/counts/graph; 100k list
virtualized; cycle safe; a11y/RTL; status != color; dup -> existing
ID; policy change clears screen/cache.""",

"kb.docs.18": """File: docs/18-admin-console.md.

Principle: managed editor of versioned resources; every change =
draft -> validation -> impact -> approval -> publish -> audit;
high-risk sections need MFA/step-up.

Schema catalog: packages/types/predicates/revisions/deps/usage;
create draft from published, add/inherit type, add predicate,
cardinality/domain/range/temporal, identity/normalization,
validation/index/search, classification, translations, views,
validate/impact/publish/retire. Stable IDs immutable after publish;
confusable-alias check.

Form/View designer: sections/tabs/columns, field components, safe-DSL
conditional visibility, list filters/facets/sorts, relation groups,
actions/capabilities, responsive/persona/locale/RTL previews, a11y
warnings; never unhides a policy-hidden field.

Migration center: source/target, primitive steps, preview, affected
rows, invalid/ambiguous/collisions, index/storage/time estimate,
dry run, approval, progress/checkpoint, quarantine/issues,
verification/checksum, cutover/compensation. Publish blocked on
unmapped security rule or required migration issue.

Users: identity status, capability + data groups separately, valid
intervals, MFA/session/device, revoke sessions, service accounts,
access review, impersonation/delegation audit. Secrets never shown.

Capability groups: function/action x allow matrix, default deny,
composable acyclic, effective preview shows origin rules, no data
selectors.

Data groups/policies: target tenant/type/entity/predicate/relation/
classification; actions read/create/edit/retire/traverse/export/
history; row conditions, fields/masks, validity/purpose, allow/deny,
owner/legal basis. Simulation: principal/persona + example query ->
obligations/plan summary + before/after + side-channel warnings; no
raw SQL rules.

Four-eyes: author submits -> independent approver (diff/impact) ->
step-up -> publish -> notify/audit; author can't approve own;
emergency = break-glass + post-review.

Connectors: catalog/instance, ops, endpoint/proxy, secret presence/
version/rotate (write-only), license/purpose, quota, mapping
version, redacted health, test connection, enable/disable, retention,
affected monitors.

Editors: criteria builder (typed expression, inputs, severity/weight,
explanation, evidence, version/dates, tests, synthetic preview);
workflow (states/transitions, tasks, SLA/calendar, conditions,
actions, escalation, compensation) + linter (unreachable states, no
terminal, unguarded loops, missing permissions).

Reports: definitions/templates, params, dataset AST, formats,
classification, schedule, recipients, limits; synthetic/authorized
preview; allowlisted variables.

Translations: resource/key x locale matrix, completeness/fallback,
ICU plural preview, overflow, RTL preview, bundle import/export,
draft/review/publish; business data never auto-translated.

Jobs/projections: queues/lag/dead letters/leases, retry/cancel,
generations/checksums; rebuild = capability/reason + new generation;
watermark not hand-edited.

Audit explorer: actor/action/resource/time/decision filters; safe
diff, revisions, original/effective actor, reason, integrity;
export = separate right/watermark.

Settings: tenant locale/tz/retention, brand, limits, origins, OIDC
refs, calendars, feature flags, maintenance windows; DB URL/secrets
not editable.

Operator protection: environment banner, destructive confirm with
name/reason, impact/dry run, optimistic concurrency, undo via new
revision, estimate before bulk-all, time-limited grants, stable-ID
diffs, orphan-ref prevention, observable publish locks.

Acceptance: schema built+published via UI; rename keeps ID/data;
identity change surfaces collisions; person-deny simulation works;
both group classes; secret unreadable; RTL preview; second approver
required; audit links draft->approvals->publish.""",

"kb.docs.19": """File: docs/19-internationalization.md.

Basics: UTF-8 end-to-end; PG UTF8; locale = BCP 47; timezone = IANA
ID; API keys/aliases ASCII lower; labels = Unicode translations; raw
value kept, canonical/search separate; never change meaning via
global lower/trim/NFKC.

Normalization: default NFC; NFKC only per analyzed identity/search
field; normalizer version stored. Check control/zero-width/bidi
controls, confusable aliases, grapheme-safe truncation, combining
marks, emoji/non-BMP, invalid UTF-8 before serde, filename
normalization (name never a storage key).

Collation: identity equality on canonical bytes (not OS collation);
user sort = versioned ICU profile + locale + public-ID tie-breaker;
collation upgrade -> REINDEX/verification plan.

Translations: LocalizedText key = resource kind/id + field + locale
+ revision; fallback e.g. fr-CA -> fr -> tenant default -> en; API
returns IDs + localized display. Translated: labels/plurals/
descriptions/help/validation/error keys/menu/templates. Alias/ID
never translated.

User data: multiple lang-tagged variants; no auto machine
translation without provenance; legal name = official; display/
transliteration = separate statements.

Names: no assumed first/middle/last — optional components, full legal
name, native script, transliteration, per-locale order/display,
mononym, patronymic, honorifics; legacy ФИО keeps components, UI
shows full_name.

Address: country/areas/locality/postal/street/building + formatted
lines + raw; composition country-dependent; Russian structure never
required globally.

Phone: country-context parse, canonical E.164, extension separate.
E-mail: IDNA/UTS-46 domain, local part not lowercased without
policy. URLs: IDN allowed; canonical ASCII host + Unicode display +
anti-phishing.

Time: instant = UTC; user/tenant/source zone stored; local date
never forced to midnight UTC; dates stay date-only; schedule = local
rule + zone + DST policy; API explicit about offset/zone semantics.

Numbers/money: decimal-as-string w/ dot canonical; UI parses locale
+ Intl; money = amount + ISO currency; percents as ratio/percent;
units versioned; Excel uses typed cell value/locale.

Messages: ICU named params/plurals/select; no concatenation; server
sends message_key + typed params; HTML forbidden or sanitized.

RTL: dir by locale/content; logical CSS; semantic icon mirroring;
bidi isolation for numbers/IDs; tested tables/forms/graph; user bidi
controls visualized/sanitized.

Search: SearchProfile = analyzer + stemming + stop words +
transliteration; exact-ID separate; cross-script optional; grapheme-
safe security-filtered highlight.

Reports/files: localized fonts (CJK/Arabic/Cyrillic), RTL DOCX/PDF,
XLSX typed values + invariant formats, CSV UTF-8 (+BOM per profile),
safe filenames + Unicode titles, locale fallback on all templates.

Test matrix: ru-RU, en-US, fr-FR, ar (RTL), tr-TR, zh-CN, mixed/
emoji — sort, normalization, identity, forms, search, reports,
import/export, filenames, dates/DST, decimal/currency, a11y.""",

"kb.docs.20": """File: docs/20-threat-model.md.

Assets: PII/ID docs, ЮЛ data/contracts/limits/debt/collateral,
relation graph, risk criteria/decisions, accounts/tokens/secrets,
connector licenses/payloads, reports, schemas/policies, audit,
availability.

Boundaries: browser <-> Nuxt/proxy <-> Rocket API <-> PDP <-> PG/
storage <-> worker <-> external sources — each authenticates/
authorizes/validates/limits/audits.

STRIDE countermeasures:
- Spoofing: OIDC PKCE/state/nonce, MFA, secure cookie, refresh
  rotation+reuse detection, issuer/audience/time validation, mTLS,
  Argon2id, session revocation.
- Tampering: parameterized sqlx, stable IDs, four-eyes publish,
  signed bundles/hashes, immutable revisions, outbox/inbox,
  checksums, tamper-evident audit.
- Repudiation: original/effective actor, request/decision IDs,
  reason, auth strength, immutable events, trusted timestamps.
- Info disclosure: PEP rewrite before SQL, RLS, response allowlist,
  aggregate policy, tenant/policy cache keys, redaction, classified
  search, encrypted storage, short downloads.
- DoS: body/depth/complexity limits, cost guard, bounded queues,
  statement_timeout, keyset, async jobs, upload scanning, quotas/
  circuit breakers, backpressure.
- Elevation: default deny/deny-overrides, two permission classes,
  scoped service identity, step-up/four-eyes, break-glass,
  simulation, negative tests.

Specific attacks: SQLi (no interpolation; catalog identifiers only);
IDOR (UUID grants nothing; PDP decides; hidden -> 404); mass
assignment (deny unknown fields; predicate allowlist; system fields
never from body); XSS (escaping, no v-html, sanitizer, CSP, sandbox);
CSRF/CORS (token + SameSite + Origin allowlist); SSRF (endpoint
allowlist, DNS/IP validation, egress proxy, redirect revalidation,
metadata/private block, size/time limits); files (signature over
extension, zip limits, macro/external-link detection, AV, isolated
converter, no execution); formula injection (CSV/XLSX escaping,
template allowlist, formulas not executed); graph inference (denied
node/edge excluded each step; counts/facets/paths authorized);
cache leak (tenant/policy-shape/locale/revision keys; no shared
private caching); logging leak (allowlist logs; no secrets/PII);
supply chain (pinned lock, cargo-deny/audit, npm SCA, signatures,
protected CI, SBOM, reproducible container, secret scanning).

Encryption: TLS 1.2+; encrypted volumes; envelope encryption for
secrets/high-risk values; per-tenant/purpose key IDs; KMS/HSM
rotation; keyed hashes for identity keys; encrypted backups; keys
never in repo/logs. Encryption never replaces access control.

Lifecycle: classification drives retention/archive/legal hold/
export/deletion; GDPR via jurisdiction policy; erasure vs hold
conflicts documented; crypto-shredding after dependency/backup plan.

Gates: authn/authz matrix; cross-tenant property tests; hidden
field/filter/sort/facet/graph/report/export; deny conflicts; RLS
pool leak; CSRF/CORS/IDOR/mass assignment; SQL/AST fuzz; upload/zip/
formula; SSRF; dependency/container scan; secrets scan; external
pentest pre-production. Confirmed leak = release blocker.

Incident: detect -> revoke sessions/keys -> disable connector/
feature -> preserve evidence -> assess -> notify -> patch -> rotate
-> verify -> postmortem; clocks synced; emergency access preserves
evidence.""",

"kb.docs.21": """File: docs/21-observability-and-operations.md.

Logs (structured JSON): timestamp/level/service/version/env,
request/correlation/trace, hashed tenant, route/action/outcome/
status, duration/error ID, plan fingerprint, job/event/connector
IDs. Forbidden: tokens/cookies/passwords/secrets, raw req/res, SQL
with values, PII, connector payload; debug via redaction allowlist.

Metrics: RED for API + USE for DB/worker — DbExecutor queue/pool/
saturation, SQL fingerprints, outbox lag/dead-letters, job queue/
lease/heartbeat/failure, projection watermark/lag, connector
latency/quota/circuit, PDP latency/cache/deny, auth anomalies,
report rows/bytes, PG connections/WAL/replica/vacuum/bloat, storage
errors/scan backlog. Bounded label cardinality — never entity/user/
request IDs.

Traces: W3C trace context through Rocket -> PEP -> DbExecutor ->
worker/connector; no payloads; async jobs linked via causation.

Audit != logs: audit = business/security evidence w/ retention/
integrity + separate access; logs = diagnostics w/ short TTL.

Health: /health/live (process alive, no external calls);
/health/ready (config, DB, PDP/policy, storage/queue, migrations);
/health/startup. Detailed health internal; public generic. Connector
failure = degraded; authorization failure -> unready/fail closed.

SLO/budget: separate SLI per surface; maintenance excluded per
policy; budget steers rollout; availability never hides incorrect
responses.

Alerts (owner/severity/runbook/silence/test): RLS canary (critical),
auth spike/reuse, PDP indeterminate/unavailable, DB saturation/
replica/WAL, outbox/projection lag, stuck leases/jobs, connector
auth/quota, backup failure/restore age, audit chain failure, cert/
key expiry, capacity.

Dashboards: executive SLO, API/DB perf, security/PDP, jobs/
projections, connectors, imports/exports/reports, PG maintenance,
tenant fairness.

Backup: WAL + base backups, storage versioning/replication, bundles/
templates/config refs, KMS backup, encrypted + geo-compliant,
automated isolated restore validation, documented RPO/RTO. Backup
exists only after a restore drill.

DR: declare/freeze -> restore point -> restore DB/objects/keys ->
verify migrations/audit -> reconcile outbox/inbox/jobs -> rebuild
projections/search -> canaries -> reopen read then write -> document
gap. Initial: RPO <=15min, RTO <=4h.

Shutdown: drop readiness, reject new long work, drain HTTP, cancel/
wait DB jobs, flush telemetry; worker stops leasing, heartbeats,
completes safe chunk or saves cursor; outbox stays retryable.

Required runbooks: DB down/saturated, slow query/plan regression,
RLS/policy issue, leaked credential, connector outage/drift,
projection lag/rebuild, stuck/dead-letter job, failed migration,
disk/WAL growth, backup restore, policy/schema rollback, data leak,
key rotation.

Capacity: monthly forecast (entities/statements/history, index/
projection, WAL, backup, storage, DB CPU/IO/mem, backlog, quota);
thresholds -> planned scaling.

Ops access: JIT/time-limited, MFA, bastion, least privilege,
approval/audit; DBA != app user; raw query = ticket/purpose +
masked replica.""",

"kb.docs.22": """File: docs/22-testing-and-quality.md.

Pyramid: domain unit/property -> crate contract -> PG repository/
integration -> security negative/information-flow -> API/OpenAPI ->
connector fixtures -> Nuxt component/E2E/a11y -> migration/upgrade/
rollback -> perf/soak/failure -> pentest/DR drills.

Domain properties (generated Unicode/schemas/entities/intervals/ops):
normalization idempotent; canonical serialization deterministic;
statement fingerprint; symmetric edge order; cardinality under
concurrency; merge/split lineage acyclic; migration round-trip/
residual; decimal/time deterministic; same inputs -> same risk;
workflow invariants.

Persistence on real PG (never SQLite/mock): fresh + per-release
upgrade; constraints/FK; identity/fingerprint races; optimistic
locking; rollback/outbox atomicity; SKIP LOCKED leases; RLS context;
pooled-context reset; partition pruning; index/plan snapshots;
tz/Unicode/decimal; serialization retry.

Security matrix (personas x actions x resources x fields x relations
x purposes): default deny; deny-overrides; two group classes;
object/type/field/edge; GET/list/search/filter/sort/facet/count/
graph/history/blob; report/export/import error file; WS/event;
cache; worker/service account; impersonation/break-glass; policy
change mid-session. Negative tests check DB rows + DOM/network/
artifact/log/cache, not just response.

API contracts: OpenAPI snapshot/compat; strict DTO; statuses/codes;
ETag/If-Match; idempotency commit-before-response; cursor tamper/
stale; limits/malformed JSON; content-type/UTF-8; CSRF/CORS;
cancellation; WS reauthorization; consumer-driven tests.

Query: AST fuzz; operator typing; bind/no literals; security filter
in every subquery/step; cost guard; keyset completeness; null/
collation; cycles/high degree; aggregate inference; fingerprint
equivalence; injection strings.

Connectors: synthetic fixtures (no prod PII), sandbox smoke, drift,
pagination, rate/quota, retry/timeout, ambiguous, malformed/huge,
duplicates, mapping version, legal flags, redaction, secret rotation.

Import/report: both supplied XLSX; multi-row founders; date/decimal/
Unicode; dedupe/retry; formula/macro/external-link; zip/MIME; error
coordinates; 100k/1M streaming memory; DOCX/XLSX/PDF golden; hidden
fields absent everywhere; locale/RTL/fonts.

Nuxt: renderer per kind; permission removes DOM/actions; persona
scenarios; 412 conflict/draft; policy event clears state; virtual
list; graph cycle/hidden node; jobs/reconnect; keyboard/focus/zoom/
RTL/axe; CSP/XSS; visual regression.

Performance: scenario manifest (SHA, schema, dataset seed/size, PG
config/hardware, concurrency, warmup/duration, SLO); profiles:
exact lookup, hot/cold card, list, FTS, graph, writes, import/
export, policy cold/cache, projection catch-up. Soak 8-24h: leaks,
starvation, bloat/lag, queues, tail. Gates per kb.docs.11.

Chaos: kill API/worker per job phase; DB restart/failover; replica
lag; storage down; PDP/cache down; connector timeout; outbox
duplicate/reorder; clock skew; disk pressure; lost response after
commit. Invariants: no duplicate, no unauthorized fallback,
resumable, explicit status.

Static/supply: fmt/clippy; cargo test/nextest; deny/audit; vue-tsc/
eslint; npm audit/SCA; secret scan; SBOM/license; container scan/
sign; migration lint; OpenAPI breaking check.

Test data: synthetic; no prod dump without anonymization; versioned
classification-tagged fixtures; international formats, no real IDs.

DoD: coverage % != risk; critical code needs branch/property +
mutation where feasible; done = requirement link + tests + telemetry
+ migration/rollback + docs + security/perf review.""",

"kb.docs.23": """File: docs/23-deployment-and-configuration.md.

Topologies: on-prem single site (LB -> enginer-api x N -> PG
primary/replica -> object storage -> workers; local or internal
PDP); HA multi-zone; dedicated tenant shard (same release).

Containers: minimal non-root, pinned digest, read-only FS, bounded
tmp, no shell/package manager, seccomp/cap drop, health endpoints,
SBOM/signature, UTC + needed locale/fonts only, controlled CA trust.
API/worker as separate images/commands.

Environments: local/test/staging/production; prod data/secrets never
copied down; own OIDC/DB/storage/keys/domains/connector accounts per
env; environment banner outside prod.

Typed TOML config: [http] public_origin/request_timeout_ms/
max_json_bytes; [database] url_secret_ref/max_connections/
acquire_timeout_ms/statement_timeout_ms/db_executor_queue;
[security] policy_cache_ttl_seconds/fail_closed/internal_mtls_
identity; [storage] backend/bucket/encryption_key_ref; [jobs]
lease_seconds/max_attempts. No passwords in files; prefixed env
overrides; unknown prod setting = startup error.

Secrets (KMS): DB credential, OIDC secret, token signing/encryption
keys, connector credentials, storage keys, identity-hash keys,
SMTP/webhook creds; short-lived where possible; rotation overlap;
never logged/returned.

TLS/network: HTTPS/mTLS with trusted-proxy contract; forwarded
headers only from known proxies; DB TLS; policies: browser->public,
API->DB/PDP/storage, worker->DB/storage/egress; egress allowlist;
cert monitoring.

CI/CD: lint/unit -> integration PG/security -> API/UI contract ->
SCA/secret/SBOM/container -> migration fresh/upgrade -> E2E/a11y ->
perf smoke -> signed artifacts -> staging -> smoke/canary ->
approval -> prod rolling/canary -> post-deploy checks.

DB deploy: expand before new app; sqlx migrate as separate job w/
advisory lock + owner role; backfill = resumable worker; contract
after old drained; CONCURRENTLY/partition ops per runbook.

Rollout: compatible app first; flags off; canary; monitor SLO/DB/
outbox/PDP; gradual; rollback binary while expanded compatible;
irreversible side effect -> compensation.

Nuxt delivery: hashed immutable assets; index no-cache; same-origin
API; public config = origin/build/version only; private source maps;
CSP/SRI.

Readiness order: secrets/config -> DB TLS -> migration compat ->
policy/schema load -> storage/deps -> workers/routes -> ready.
Worker never runs unsupported schema/event version.

Scaling: HPA on CPU + request/queue metrics within global DB
connection budget; worker per queue class; no scale-to-zero for
latency-critical API/PDP; tenant fairness quotas.

Prod checklist: no default passwords; migrations verified; RLS
FORCE + grants; OIDC/MFA/CSRF/CORS/CSP; keys/certs/rotation; current
restore drill; audit chain; limits; alerts/runbooks; connector
legal/quotas; locale/fonts; load/pentest; rollback/compensation.""",

"kb.docs.24": """File: docs/24-current-code-migration.md.

Prototype (legacy-prototype/, excluded): Rocket 0.5-rc, Object/
ObjectType/Field, Link/LinkType, Dictionary, User/Group/Permission,
sqlx/PG, LFU cache — PoC, not production baseline.

Limitations: stale/mixed deps; plaintext DB password in config;
format-string SQL (injection); lazy-static pool + block_on; String
IDs; RawHtml/generic ERROR; 500 on client validation; allow-by-
absence permissions; post-materialization field security; ФЛ/ЮЛ not
universal-schema; definition/value mixed in Object; no statement
invariant; dynamic DDL; unproven cache keys; secrets in domain
types; RISE DDL; no Nuxt/jobs/outbox/policy/provenance/observability.

Rule: never rewrite in place — new workspace alongside, strangler +
migration importer; old sources stay reference.

Stage 0 repo security: rotate DB password (published = compromised);
remove tracked secrets -> ref/env; ban prototype for prod data;
secret/dependency scanning; read-only snapshot. Git history keeps
secrets — rotation mandatory.

Stage 1 skeleton: apps/crates per docs/05; Rocket 0.5.1 health,
typed config/error/context, DbExecutor, sqlx migrations; legacy
routes not auto-connected.

Stage 2 security baseline: tenant/principal/session, capability +
data groups, PDP/PEP, RLS, audit — before any data API; legacy
permissions -> draft policies + manual simulation.

Stage 3 meta-kernel/graph: meta/core tables, domain types, statement
families, identity/idempotency/outbox; schema publish + create/get/
query. Mapping: ObjectType->TypeDefinition; Field->PredicateDefinition;
Object.filled.fields->Entity+scalar statements; LinkType/Link->edge
predicate/statement or reified relation; Dictionary->classifier.

Stage 4 extraction: read-only parameterized extractor; immutable
source records; normalized IDs/timestamps/values; mapped users/
groups/objects/links/dictionaries; row-level quarantine; old-ID
aliases; count/relation/hash reconciliation. Importer idempotent by
source fingerprint + table + PK + mapping version.

Stage 5 SKR package: bundle ФЛ/ЮЛ/holding/contact/document/address/
request/check/risk/collateral + views/identity/workflow/translations/
Excel mapping; no Person/LegalEntity structs in core.

Stage 6 Nuxt: login/layout, dynamic list/card/form/relations/tree,
checks/reports/jobs, admin; legacy UI read-only or redirected.

Stage 7 dual-run: new writes to new canon; legacy one-way
compatibility projection only; UAT; freeze legacy writes; delta
import; switch; archive; decommission.

Reconciliation per table/type: source rows, imported/quarantined/
skipped, merges/candidates, statements by predicate, orphan links,
tz precision, null/empty, dup keys, checksum buckets, sampled
comparison; cutover blocked on unexplained delta.

API compat: legacy consumers via separate adapter (old paths ->
new commands + security decision + deprecation deadline); RawHtml/
generic ERROR never carried.

Reusable: domain knowledge/scenarios, Rocket stage organization,
type/field/link/dictionary concepts, auth reference, fixtures + SQL
model, Cargo.lock. Never reused: SQL string builders, secret config,
allow-by-absence, dynamic DDL.

Acceptance: all rows accounted; old ID resolves; links/provenance/
history kept; no dup identity; permissions not widened; secrets
rotated; scenario equivalence; new SLOs; rollback rehearsed;
decommission approved.""",

"kb.docs.25": """File: docs/25-delivery-plan-and-acceptance.md.

Phases + exits:
- P0 Foundation: workspace, CI, config/secrets, Rocket health/
  error/context, DbExecutor, migrations, observability, dev stack.
  Exit: safe app connects; migration/backup smoke; no secrets; SCA.
- P1 Security/meta-kernel: tenant/identity/session, two group
  classes, PDP/PEP/RLS/audit, schema draft/publish. Exit: persona
  matrix default-deny; schema publishes; leak tests pass.
- P2 Entity graph: entities/statements, identity/idempotency, CRUD/
  history, AST/keyset, relations/graph, projections/search. Exit:
  arbitrary type without code; dedupe works; 100M SLO met.
- P3 Nuxt core/admin: auth/layout, dynamic UI, jobs, schema/security
  admin, i18n/a11y. Exit: new type after publish without deploy;
  role UI/E2E/RTL/keyboard pass.
- P4 SKR package: ФЛ/ЮЛ/holding, request/check/risk/workflow, Excel
  import, views. Exit: DOCX/XLSX scenarios; no hardcoded objects.
- P5 Connectors/monitoring: SDK, sources, provenance, schedules,
  criteria/reactions. Exit: sandbox checks, stale/error semantics,
  idempotent monitoring, legal/secret/quota controls.
- P6 Reporting/production: reports/exports, perf, HA/backup/DR,
  pentest, cutover. Exit: prod checklist, restore/rollback drill,
  signed release, UAT.

Connector order: internal DB -> supplied imports -> authoritative
tax/company -> enforcement/court/procurement/registration/health ->
Internet/social last.

DoD: requirement link; designs; security classification/policy;
migrations/rollback; tests incl. negative; telemetry; i18n/a11y;
perf/limits; runbook; docs; clean deps; review.

E2E scenarios: A universal warehouse package published live; B 100
concurrent same-phone creates -> one entity + retries one ID +
distinct extension; C both ЮЛ workbooks preview/dry-run, graph
mapping, repeat import no dupes, relations w/ provenance; D type
split/reify/rename round-trip + residual + quarantine + concurrent
writes in projection; E holdings-allowed/person-denied sees nothing
of persons anywhere (incl. counts/graph/report/WS/cache + DB
instrumentation); F field security — same ЮЛ, one sees passport/
phone, other nothing incl. sort/facet/export + mass-assign denied;
G check run — source timeout != clean, provenance, idempotent
factor/assessment/task/decision/report/monitor; H monitor — DST run
once, repeat result no dup, real change -> evidence/task; I 100M/
500M SLOs, no OFFSET/seq scan; J i18n names/IDN/decimal/case/DST/
RTL/emoji round-trip; K audit/DR — tamper-evident links, RPO/RTO
restore, projection rebuild, canaries.

Perf acceptance: P95 targets from vision; zero unauthorized rows;
bounded graph/export; projection-lag target; no queue collapse;
soak stable.

Security acceptance: threat model reviewed; independent pentest;
critical/high fixed; RLS/PEP matrix; MFA/four-eyes/break-glass;
rotation; scans; incident/restore drill; connector legal basis.

Docs acceptance: links exist; terms consistent; examples match
contracts; traced requirements; ADR updated; architecture change
updates docs+tests.

NOT readiness: small test only, UI-hidden button, SELECT-before-
INSERT, generic JSONB, post-filter permissions, manual SQL, untested
backup, field-policy-free report, plaintext connector secret,
missing perf dataset.""",

"kb.docs.26": """File: docs/26-decisions-and-tradeoffs.md.

ADR-001 Модульный монолит — api/worker + strict crates; no
microservice-per-context.
ADR-002 Statement graph + typed physical families — rejected
table-per-type, one-JSONB, pure RDF.
ADR-003 Stable IDs, labels mutable — rename never migrates facts/
policies/views; UUID in API.
ADR-004 Canonical facts semantically immutable — change = revision/
supersede/revoke; projections rebuildable.
ADR-005 Residual facts for lossless transform.
ADR-006 PostgreSQL source of truth; no dynamic DDL in request path.
ADR-007 SQLx async-std bridge — Rocket Tokio unblocked; PgPool owned
by AsyncStdDbExecutor (deviation: sqlx 0.7.4 in repo, spec pinned
0.5.13 — see kb.conventions).
ADR-008 Logical Security API, local hot-path transport —
LocalSecurityClient calls PDP in-process; mTLS endpoint for others.
ADR-009 Permissions before query — PDP obligations rewrite AST
before SQL; post-fetch filtering rejected.
ADR-010 Two group classes — capability vs data never mix.
ADR-011 Default deny / deny-overrides — allow-by-absence forbidden;
break-glass separate flow.
ADR-012 Idempotency ledger + semantic keys — transport idempotency
!= identity/fact uniqueness.
ADR-013 Keyset pagination — OFFSET rejected; signed cursor binds
query/schema/policy.
ADR-014 Background jobs/outbox — external calls/bulk/monitoring
never in request tx; exactly-once via inbox/ledger.
ADR-015 Nuxt authenticated SPA — ssr:false for app/admin; public
pages may SSR.
ADR-016 Metadata-driven UI, security not UI-driven.
ADR-017 Fuzzy never auto-merges — exact keys may return existing;
probabilistic -> candidate.
ADR-018 Holding as graph view — temporal ownership/control/
affiliation; no nested duplication.
ADR-019 International defaults — UTF-8/NFC/BCP47/IANA/decimal;
locale assumptions banned.
ADR-020 No ordinary physical delete — retire/revoke/tombstone;
regulated erasure via separate workflow.
ADR-021 Service account isolation — job binds initiator snapshot/
current auth; own narrow grant.
ADR-022 Report/export security = dataset security — templates can't
request undeclared/forbidden fields.
ADR-023 PostgreSQL 17+ baseline; newer-major features only via ADR.

ADR changes need context + alternatives + consequences + migration
+ security/perf impact + approval + dependent doc/test updates;
old ADR superseded, never rewritten.""",

"kb.docs.27": """File: docs/27-glossary.md.

Canonical terms:
- EngineR: universal platform core. СКР САНАР: applied risk-control
  package. Аннуитетный риск: permanently-possible risk of continuous
  activity.
- Entity: stable graph-node identity anchor. Statement: atom
  subject-predicate-entity/scalar + time/provenance (prefer
  Statement over "fact" in code).
- Predicate: meaning of a property/relation. TypeDefinition:
  versioned constraints/behavior. Meta-kernel: closed primitive set.
  Schema package: signed declarative types/views/policy refs/
  workflows/reports. Schema revision: immutable published schema.
- Residual fact: inexpressible source fact kept with lineage.
  Lineage: origin/transformation trace. Provenance: source +
  evidence. Effective view: multi-source view by trust policy.
- IdentityPolicy: exact/fuzzy identification rule. Identity key:
  canonical unique key -> owner. Match candidate: hypothesis, not
  merge. Fingerprint: deterministic canonical hash. Idempotency
  ledger: stored outcomes of repeated commands.
- Merge/Split: governed identity union/division + lineage.
  Projection: rebuildable read model. Watermark: canonical revision
  a projection reached.
- Query AST: typed JSON query tree. PEP: enforcement point. PDP:
  decision point (decision + obligations). Obligation: mandatory
  limit/mask/filter.
- Capability: system-function right. Data permission: right to
  type/entity/field/relation/row. Capability/data groups: only that
  class of rights. RLS: PG row security barrier. Purpose of use:
  declared processing purpose. Break-glass: emergency access +
  step-up/audit.
- Outbox: events atomic with mutation. Inbox: processed-events
  ledger. Job: resumable background operation.
- Connector: source adapter. Observation: source's version of facts.
  Check: sources+criteria orchestration. Risk criterion: versioned
  typed rule. Risk factor: detected condition + evidence.
  Assessment: computed score on versioned inputs. Decision:
  separate human/workflow decision. Monitor: scheduled recheck/
  detector.
- ViewDefinition: list/card/form/report metadata. Public ID:
  UUIDv7 for API. Internal ID: compact bigint. Keyset cursor: opaque
  signed continuation by sort keys.
- ЛТК: trade-credit limit (package term). ОДЗ/ПДЗ: receivables
  indicators — semantics set by the customer's package. ЮЛ/ФЛ:
  legal/natural person — package types, not core structs. Холдинг:
  temporal ownership/control/affiliation graph view.""",
}


def main():
    src = open(TARGET, encoding="utf-8").read()
    for kb_id, content in NEW.items():
        # find the article dict for kb_id and replace its content="""...""" block
        start = src.index(f'kb_id="{kb_id}"')
        cstart = src.index('content="""', start)
        cend = src.index('"""', cstart + len('content="""'))
        src = src[:cstart] + 'content="""' + content + src[cend:]
    open(TARGET, "w", encoding="utf-8").write(src)
    # verify all sizes
    sys.path.insert(0, HERE)
    import importlib
    import kb_docs_provision
    importlib.reload(kb_docs_provision)
    bad = [(a["kb_id"], len(a["content"].encode("utf-8")))
           for a in kb_docs_provision.ARTICLES
           if len(a["content"].encode("utf-8")) > CAP]
    print("over cap:", bad or "none")


if __name__ == "__main__":
    main()
