"""Second trim pass — cut remaining over-limit kb_content blocks to
<=2250 bytes so every statement fits the fact-unique btree limit.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TARGET = os.path.join(HERE, "kb_docs_provision.py")
CAP = 2250

NEW = {
"kb.docs.03": """File: docs/03-universal-domain-model.md.

All data = directed attributed multigraph: nodes are entities; edges
+ properties are statements; types/fields/forms/rules/translations are
meta-level entities/statements. Logic unified; storage specialized.

Entity: EntityId bigint internal + PublicId UUIDv7; tenant_id,
entity_class (data|schema|policy|workflow|system), created/retired,
head_revision, canonical_entity_id (after merge).

Statement: subject, predicate, object (Entity OR Scalar — exactly
one), validity (domain truth), recorded (system history), provenance,
confidence, status, revision. Revocation = new record — past never
erased.

PredicateDefinition: stable ID + ASCII alias, domain/range, directed/
symmetric/transitive/inverse, cardinality + uniqueness, required/
temporal, identity participation, normalization/validation, index/
search profile, confidentiality class, translations, non-semantic
presentation hints. ИНН = scalar statement; "managed by" = entity
statement; typing = instance_of.

TypeDefinition: allowed/required predicates, composition/inheritance,
identity policies, validation, layout/search/report profiles,
lifecycle SM, published revision; multiple types per entity allowed.

Meta-kernel (closed): Entity, Statement, TypeDefinition,
PredicateDefinition, IDs, revisions, primitive scalars, metaschema
rules — change needs code/DB migration; app schema doesn't.

Scalar kinds: text (+lang-tagged), boolean, integer, decimal,
date/time/instant/local+IANA, UUID/opaque, URI/email/phone, money
(decimal+ISO 4217), measurement, geo, schema-bound JSON, blob ref.
f64 forbidden for money/shares/scores.

Provenance: source record, fetch/effective times, payload hash,
legal basis/license, parser/mapping version, actor/job, confidence,
evidence. Sources never overwrite; resolution policy -> effective
view + keeps conflict.

ViewDefinition: root types, sections/fields, collections, columns/
facets, computed values, actions, permission requirements —
grants no access.

skr-core bundle: person, legal_entity, holding_view, address,
contact_point, document, license, request, check, risk_factor,
assessment, decision, task, contract, collateral; predicates e.g.
has_tax_id/managed_by/founded_by/owns_share/affiliated_with/
has_address/checked_by; identity policies for ИНН/ОГРН/document/
phone/source IDs; loaded via Admin API.

Holding = computed view over ownership/control/management/
affiliation (date+threshold); cycles/shares/history OK.

Invariants: one object per statement; same tenant (except global
schema refs); domain/range vs command revision; active statement
immutable in place; canonical IDs acyclic; projection never sole
store; canonical stored (+raw if needed).""",

"kb.docs.04": """File: docs/04-topology-unitarity-idempotency.md.

Unitarity: one constructor — statement "subject — predicate ->
object"; property vs relation differ only by object kind. Topology:
finite transformation path between published schemas never destroys
canonical facts — inexpressible data kept as residual facts.
Idempotency: f(f(x))=f(x) for commands, facts, workers/imports,
migrations.

Primitives: Add/Retire/AliasType; Add/Retire/RenamePredicate|Label;
Add/RemoveMembership; MapPredicate; SplitPredicate+residual;
MergePredicates+lineage; ConvertScalar (total or quarantine);
ReifyEdge; CollapseRelationEntity; MergeEntity/SplitEntity;
MaterializeDerived/DiscardProjection — each w/ forward, optional
reverse, conditions, mapping version, report.

Rename: aliases/labels change; type_id/predicate_id never; display
name never a DB/API key.

IdentityPolicy: scope, key components, normalizer, quality, exact/
fuzzy, collision action, resurrection. Exact keys: ЮЛ = country+
regnum+ОГРН (alt verified ИНН); phone = E.164+ext; document =
country+type+series/number; external = connector+namespace+source
ID. Name/birthdate/address -> same_as_candidate only.

Normalization: store raw + canonical + normalizer_version; UTF-8
NFC + trim (case kept); casefold search key; ИНН/ОГРН checksum;
E.164/ext; IDNA email; decimal canonical scale; instant UTC + zone;
symmetric edge = sorted endpoints. New normalizer -> migration +
collision report.

Fact fingerprint: tenant|canonical_subject|predicate|object_kind|
canonical_object|validity|qualifier_hash|identity_scope. Scopes:
global_fact/per_source/per_observation. BLAKE2b-256 lookup;
collisions on full canonical columns.

Idempotency-Key: unique tenant+principal+route+key; same payload ->
stored response; different -> 409; long call -> one job.

Concurrent create: ON CONFLICT one owner; second reads it — same
public ID, deduplicated=true. SELECT-before-INSERT forbidden.

Merge: canonical picked, losers -> aliases, statements resolved,
conflicts kept, keys moved, EntityMerged; repeat no-op. Split: plan
distributes + anchor; repeat no-op. Inbox: unique consumer+event+
handler_version.

Verifiable: normalize idempotent; repeat create = same ID; symmetric
edge order irrelevant; forward+reverse restores; rebuild checksum
stable; residual never unreachable.""",

"kb.docs.05": """File: docs/05-rust-workspace.md.

Layout: apps/{api,worker} + crates/{domain, application, contracts,
schema-registry, identity, query-engine, persistence-postgres,
db-runtime-async-std, security-domain, security-pdp, security-api,
connectors, risk-engine, workflow, reporting, files, observability,
test-support} + migrations/ + schema-packages/skr-core/ + web/ +
docs/. Deps point inward; domain never imports adapters.

Cargo: workspace versions; lock-pinned; default-features off;
cargo-deny; unsafe banned; warnings denied in CI. Spec pins sqlx
0.5.13 async-std — DEVIATION: repo uses 0.7.4 (kb.conventions);
rocket 0.5.1 json+secrets.

Newtypes required: TenantId, EntityId, PublicEntityId, TypeId,
PredicateId, SchemaRevisionId, StatementId, PolicyId, DecisionId,
PrincipalId, GroupId, SessionId, RequestId, CorrelationId,
IdempotencyKey, JobId, EventId, ProjectionRevision, LocaleTag,
TimeZoneId, ContentHash — never plain String.

Ports: UnitOfWork/TransactionRunner; Entity/Statement/Schema/
IdentityKey repositories; QueryExecutor; SecurityDecisionPoint;
Outbox; AuditSink; Clock; IdGenerator; BlobStore; ConnectorInvoker;
JobQueue. No PgRow/rocket::Request/serde_json::Value crossing.

DbExecutor: run(deadline, closure over PgPool); pool in async-std;
bounded queue + cancel; closure never leaves runtime; panic
contained; graceful drain; readiness false on DB-down/saturated.

RequestContext: tenant/principal/session/request+correlation IDs/
locale+tz/auth strength/client/deadline/policy snapshot/purpose —
guard-created, explicit, no globals.

Errors: Domain/Application/Security/Persistence/Api — stable code +
safe details; never leak SQL/stack/secrets/hidden existence.

Transactions: one boundary per use case; no external calls in tx;
lock order identity->entity->statement->outbox; retry only
serialization/deadlock + jitter; outbox for side effects; READ
COMMITTED; optimistic revision/If-Match.

Style: fmt+clippy -D warnings; no format!-SQL w/ user values;
secrecy/zeroize; Argon2id; UTC+explicit zone; decimal not float;
deny_unknown_fields on commands.

Config: defaults -> file -> env -> secret provider; startup validates
DB TLS/pool/keys/origins/storage/timeouts/connector refs/audit;
printed redacted. Feature flags: owner/expiry/default/audience/
audit; never Cargo features for tenant capability.""",

"kb.docs.06": """File: docs/06-api-contract.md.

Base: /api/v1 public; /internal/v1 + /internal/security/v1 internal.
HTTPS outside dev; JSON UTF-8; decimals as strings; instants RFC
3339 UTC; public IDs UUID; unknown command fields rejected;
X-Request-Id + X-Correlation-Id; OpenAPI from contracts crate.

Envelope: {data, meta:{request_id, schema_revision, commit_revision,
policy_revision}, links}. Error: {error:{code, message_key,
parameters, request_id}}; Nuxt localizes message_key.

Statuses: 401 auth; 403 denied (hidden -> 404); 409 identity/
version/idempotency; 412 If-Match; 422 validation; 429 +Retry-After;
503 fail-closed. DELETE = retire/revoke + tombstone; purge = separate
approval.

Auth: OIDC AC+PKCE + HttpOnly SameSite cookie; short bearer for
service accounts. POST /auth/session/exchange|refresh|logout; GET
/me, /me/context; POST /auth/step-up. Refresh rotation + reuse
detection; CSRF/Origin for cookie mutations.

Schema: GET /schemas/types{,/{id}}; POST /schemas/drafts; PUT/
schemas/drafts/{id}; POST .../validate|impact|publish; POST
/revisions/{id}/retire; GET .../migration-status. Publish = If-Match
+ privilege + impact + reason.

Entity: POST /entities; GET /entities/{public_id}; POST /entities/
query; PATCH; POST retire|restore|merge|split; GET history|related;
POST graph. Create: type_id, schema_revision, identity_mode,
statements[](predicate_id + typed {kind,value}) -> created/
deduplicated + revision + warnings + actions.

Statements via entity command; POST /entities/{id}/statements; POST
/statements/{id}/revoke|verify; GET provenance. Never change
predicate/subject/object in place.

Graph: as_of, max_depth, max_nodes, predicates, direction,
include_summaries; limits intersected w/ obligations; cycle = edge
to visible node; signed cursor. Query: page 50/500 max; signed
cursor; OFFSET forbidden.

Jobs: POST /jobs/imports|exports|bulk-mutations; GET/cancel/errors/
artifact — dry_run, snapshots, per-row results, limits, explicit
partial success.

Checks/reports: POST /checks, /checks/{id}/run (202+job), POST/PATCH
/monitors, POST /reports, /reports/{id}/download. Admin:
/admin/users|service-accounts|capability-groups|data-groups|
policies|policy-simulations|connectors|translations|audit|jobs;
secrets write-only.

Concurrency: ETag "entity:uuid:revision"; If-Match or
expected_revision; 412 + safe summary; force update = capability +
reason. Caching: ETag binds revision+view+locale+policy; secret ->
private/no-store.

Events /api/v1/events: permitted topics, field-filtered payloads,
policy change -> reauthorize/close, cursor reconnect, bounded
buffer; events notify, client re-reads canon.

Limits: centralized LimitProfile — body, statements/command,
filters, sort keys, page, graph depth/nodes, export rows/bytes,
upload bytes, concurrent jobs, deadline.""",

"kb.docs.07": """File: docs/07-query-engine.md.

AST: {from:{type_id,alias}, select, where:{and/or/not,...}, sort,
page:{size,after}, as_of} + exists/related/aggregates/group_by/
facets; joins only via relation path.

Operators per kind: eq/ne/in/is_null; lt/lte/gt/gte/between;
contains/prefix/full_text; date overlap/valid_at; relation exists/
not_exists; geo within/distance. Regex/script/SQL/uncontrolled
wildcard forbidden; unindexed contains may be cost-rejected.

Pipeline (fixed): parse+limits -> schema revision -> type-check ->
normalize literals -> capability -> PDP -> security AST rewrite ->
index selection -> cost estimate -> parameterized SQL -> deadline/
cancel -> response allowlist -> audit. Never string-append security
WHERE.

PDP returns: allowed root types, select + relation predicates/
directions, mandatory row predicates, masks, max page/depth/export,
forbidden aggregates/facets. Rewriter: intersects select, injects
mandatory filters at root + every subquery, forbids sort/facet on
hidden fields, removes denied edges, caps summaries, stamps policy
revision. Denied root -> false/no-rows or 404.

SQL compiler: catalog identifiers; bind params; family-grouped
predicates; partial index/status for current; temporal range; every
join has tenant_id; membership before wide joins; EXISTS/LATERAL not
DISTINCT; query tag = plan hash.

Keyset: sort ends unique public_id; cursor = query/schema/policy
fingerprints + last sort values + direction + tenant + expiry —
signed; schema/policy change -> 409 cursor_stale.

Cost classes: interactive | extended | batch-only (->job) |
rejected. Estimates: paths, depth, selectivity, index fit, fan-out,
aggregate cardinality, sizes.

Graph: recursive CTE only w/ max_depth/max_nodes + allowlist +
tenant/temporal/security per step + cycle detection + deterministic
order + cursor + timeout; hierarchies -> closure projection.

Aggregates: separate permission; fields must be allowed; sensitive
may need min group size/rounding/denial.

Saved queries: AST + owner/scope/revision — under current policy.
Plan cache: tenant + AST + schema + policy shape + projection
revisions; publish invalidates.

Errors: query.syntax_invalid, unknown_type/predicate,
operator_not_supported, field_forbidden, too_complex, requires_batch,
cursor_invalid/stale, projection_lag, timeout.""",

"kb.docs.08": """File: docs/08-security-architecture.md.

Separation: security-domain/-pdp/-api crates; every controller/use
case/query/export/worker passes a PEP. Hot path: LocalSecurityClient
(same JSON contract, no self-HTTP); RemoteSecurityClient optional.

Two right classes: capabilities (login/use_ui, entity.create/edit/
retire/merge, query.advanced/aggregate, check.run/monitor.manage,
report.run/export, bulk.*, schema.draft/publish, security.manage/
simulate, connector.manage, audit.read, admin.pages.access) in
capability groups; data permissions (tenant/namespace, type, entity,
field, relation/direction, statement/provenance, classification/tag/
ownership, row condition, period, purpose) in data groups.

Rule: effect, subject/group, action, selector, condition, fields/
relations, mask/obligation, priority, interval, revision, reason.
Default deny; deny-overrides; break-glass = separate step-up flow.

Evaluation: tenant/session/auth-strength -> snapshot memberships ->
capability -> data rules -> drop false/expired -> deny-overrides ->
allowed set + obligations -> constrain -> decision audit. Fail
closed always.

Enforcement: router guard; application PEP; query PEP rewrite;
repository (mandatory tenant+policy predicates); RLS; serializer
allowlist; blob decision; worker (service identity + initiator
snapshot/recheck). UI visibility never a boundary.

Field policy: allow|omit|null-mask|partial-mask|tokenize|
aggregate-only (omit preferred). Edge permission: predicate/
direction/endpoint types/context — denied edge can't be fetched/
traversed/sorted-by.

RLS: separate roles; API not owner, no BYPASSRLS; tx-local
tenant/principal/purpose; FORCE RLS; connection reset. RLS never
replaces PEP.

Authn: OIDC + optional local admin; AC+PKCE; MFA/step-up for
publish/export/security/break-glass; Argon2id; rotating refresh +
reuse detection; scoped service accounts + mTLS; kid'd keys.

Policy cache: key = tenant+principal+revisions+strength+purpose;
decision shape; events invalidate; short TTL; high-risk bypass.

Audit: authn, decision, denied, policy change/simulation, export,
blob read, bulk, break-glass — actors, fingerprints, reasons, policy
revisions, counts, tamper-evident chain; no secrets.

Impersonation: capability + ticket + timebox + banner + original/
effective + audit; delegation = explicit grant. Break-glass: route,
MFA, reason, minimal scope/TTL, notify, auto-expiry + review.

Policy changes: draft -> validate -> simulate -> four-eyes ->
publish; rollback = new revision.

Fail: PDP down -> closed; audit down -> high-risk closed; stale
policy -> recheck + stale cursor.""",

"kb.docs.09": """File: docs/09-security-api.md.

Endpoints: POST /internal/security/v1/decisions/action|query|
mutation|blob; POST /internal/security/v1/simulations; GET
/policies/revision; POST /cache/invalidate. Same contract for
LocalSecurityClient + mTLS HTTP; caller from mTLS/token; delegated
subject separate.

Context: request_id, tenant_id, principal {id, session_id,
auth_strength, groups_revision}, purpose, locale, time,
client_service. PDP loads membership or verifies signed snapshot —
never trusts client group lists.

Action decision -> decision_id, effect, policy_revision, obligations
{max_rows, watermark, require_step_up, audit_level}, expires_at.

Query decision: action entity.query + query {root_type_ids,
selected_predicate_ids, relation_predicate_ids, operations,
requested_page_size, requested_graph_depth, query_fingerprint}.
Response: allow_with_obligations + {allowed_root_type_ids,
denied_type_ids, allowed/denied_predicate_ids, allowed_relations
[{predicate_id, directions, target_type_ids}], mandatory_filters{all:
[{system_field, op, value|in_subject_scope}]}, masks, max_page_size,
max_graph_depth, max_graph_nodes, aggregates}. PEP checks policy
revision before cache/cursor.

Mutation: request = command + target + changed predicates +
relation targets + expected revision + batch. Response: allow/deny
+ allowed predicates + required approvals/values + forbidden targets
+ transforms + max batch + required reason + audit level.
Partially-forbidden -> denied wholesale; partial only via explicit
endpoint + per-op results.

Row filters: typed security AST (distinct from user AST) over
pre-registered system fields/scopes; PDP never returns SQL; same
trusted compiler. in_subject_scope via scope table/CTE; huge ID
lists via scope handle/revision.

Masks: omit|null|fixed|partial(prefix/suffix)|hash/token|bucket/
range|aggregate_only — predicate + algorithm ID/version +
conditions; hidden fields forbidden in sort/filter.

Decision binding: subject/session/strength + action + fingerprint +
revisions + purpose + expiry; decision_id not a bearer permission.

Effects: allow | allow_with_obligations | deny | indeterminate
(->deny) | challenge. Codes: security.unauthenticated,
capability_denied, data_denied, step_up_required, policy_stale,
pdp_unavailable, context_invalid. Public maps data_denied -> 404.

Audit: PEP reports outcome (decision_id, rows/bytes, revision,
status, latency, error class); outbox delivers later.

Hardening: mTLS + service allowlist, audience-bound token, network
policy, limits, replay protection, no CORS/browser, safe logs,
readiness on policy store, fuzz tests.""",

"kb.docs.10": """File: docs/10-postgresql-schema.md.

Principles: PG = source of truth; sqlx; DDL via migrations; JSONB
only bounded metadata/raw; UUIDv7 public + bigint internal;
tenant_id + RLS; timestamptz; numeric finance.

Schemas: core | meta | sec | ops | audit | integration | report.

core.tenant: id, alias, default_locale/tz, data_region, status.
meta.schema_revision: state (draft/validating/ready/published/
retired/failed), parent, semantic_version, content_hash,
migration_plan; ONE current published (partial unique). +
type/predicate definition + revision tables + type_predicate +
localized_text.

core.entity: PK (tenant_id, entity_id); public_id unique; class;
created/retired; head_revision; canonical_entity_id; CHECK != self.
+ entity_alias + entity_revision.

Statement families: core.statement_edge|_text|_number|_boolean|
_date|_instant|_geo|_blob|_json — shared header (tenant,
statement_id, public_id, subject, predicate_id, schema_revision,
entity_revision, valid_from/to, recorded_at, status active/
superseded/revoked/quarantined, provenance_set_id, confidence,
canonical_hash, qualifier_hash, supersedes/revoked_by) + typed
payload (edge = object_entity_id; text = raw/nfc/search/lang;
number = value+unit/currency; json = value+schema hash). Active
uniqueness fingerprint = tenant+subject+predicate+canonical payload
+validity+qualifier+scope. NOTE: the active-fact unique index embeds
the canonical object — scalar values bounded ~2.5KB by btree row
limit (>2704 -> 500).

core.entity_type_current: hot membership projection; partial
status=active indexes on families; optional header projection.

core.identity_policy + core.identity_key (unique tenant+type+policy+
key_hash; canonical_key maybe encrypted; state; quality) +
core.match_candidate (never a merge).

integration: source_system, source_record (source+namespace+
external_id unique), provenance_set/member.

ops.entity_summary (locale, safe fields, revisions); ops.entity_
search (tokens+tsvector+trigram; sensitive shards split); ops.
typed_value_index; ops.projection_catalog/checkpoint (watermark,
generation, checksum, atomic switch); managed hot read models OK
but rebuildable.

sec.*: principal/user_identity/session/credential; capability_group,
data_group + memberships (interval/revision); policy+revision+rule;
subject_scope_membership; decision_log/outcome; break_glass_grant.
Secret ciphertext separate + key ID.

ops: idempotency_record (tenant+principal+route+key unique);
outbox_event; inbox_delivery (consumer+event+handler unique); job;
job_item.

audit.event: append-only, monthly RANGE; event/actor/action/finger-
prints/hashes/checkpoint; UPDATE/DELETE forbidden.

core.blob/blob_reference; report.definition/revision/run/artifact.

Partitioning: statements HASH by tenant bucket (32/64); audit/outbox/
observations RANGE monthly; partition key in unique constraints.

Indexes: entity tenant+public_id/canonical; type membership; per-
family subject+predicate active; edge object+predicate; typed value;
identity unique; GIN tsvector; outbox/job/audit. Each index has
owner/selectivity/cost — no catch-all JSONB.

RLS: tx set_config(tenant/principal/purpose/policy); rejects absence;
reset after tx; pooled leak tested. Migrations via separate job;
expand/contract + resumable backfill; no DDL from user request.""",

"kb.docs.11": """File: docs/11-performance-and-scaling.md.

Profiles: dev 100K | integration 10M | scale 100M+history |
extreme 500M+ statements; skew reproduced.

SLO histograms: exact lookup, card, list pages, full-text, relation
list, graph d1/2/3, writes, policy decision/rewrite, job enqueue,
projection lag — P50/P95/P99 + throughput + errors + rows examined/
returned + waits.

Budgets: workers != connections; bounded DbExecutor queue; pool from
DB max/replicas/instances; acquire < request deadline; batch own
pool; never hold connection during external HTTP/render; cancellation
propagates; tx-local statement_timeout.

Query discipline: keyset; selective type/tenant first; EXISTS/
LATERAL; project requested predicates; fan-out limits; summaries;
bounded IN (big scopes via temp tables); no SELECT *; no functions
on indexed columns; plan snapshots.

Reads: identity_key unique -> summary + current; batched family
predicates (no N+1); aggregated/lazy counts.

Export job: pin snapshots; keyset chunks; stream; per-chunk or
bound-snapshot policy; byte/time limits; artifact; no giant tx.

FTS: entity_search per locale/profile; exact via B-tree; trigram
for proven fuzzy; sensitive text never in shared vector.

Graph: adjacency B-tree; hierarchies via closure projection;
arbitrary = CTE + budget; high-degree paged; cache key = revision +
predicates + as_of + policy shape; deny per step.

Partitions: hash helps maintenance, never replaces tenant filter;
benchmarked count; partition key in unique constraints; tenant in
queries; default partition forbidden; precreated; detach runbook.

Projection lifecycle: draft -> building -> verified -> active ->
retire. Verify: bucket checksums, watermark, sampled compare,
classification, plan benchmark. Read-after-write waits watermark or
canonical fallback.

Cache: L1 bounded (immutable-by-revision); distributed optional;
key = tenant+revision+locale+policy shape; event invalidation +
TTL; singleflight.

Maintenance: per-table autovacuum, fillfactor, bloat/WAL/replica
monitoring, ANALYZE on skew, extended stats, REINDEX CONCURRENTLY,
pg_stat_statements, no idle-in-tx.

Write amplification: each index/projection priced (insert/WAL/
storage); IndexProfile per predicate via online job; unused index
removed.

Sharding: partitions/replicas/projections first; TenantLocator;
tenant unsplit; analytics in separate warehouse.

Backpressure: per-tenant limits; bounded queues; connector quotas;
429/503; adaptive disable; circuit breakers.

Gates: critical P95 regression >10% w/o ADR; unapproved seq scan;
rows-examined multiplication; post-fetch policy filtering; lag over
SLO; migration lock over budget; dupes or tenant leak.""",

"kb.docs.12": """File: docs/12-schema-evolution.md.

States: draft -> validating -> impact_ready -> approved ->
publishing -> published -> retired/failed. Published immutable; fix
= new revision; one current per package/tenant.

Draft validation: aliases/IDs/translations, domain/range/cardinality,
inheritance cycles, inverse/symmetric coherence, identity
normalizers, migration totality, index/search profiles,
classification, view/report/workflow refs, confusables, cost.

Compatibility: presentation-only | additive | conditional (required
field w/ backfill) | semantic | breaking | meta-kernel.

Publish plan: freeze hash -> simulate on stats -> concurrent index/
projection build -> deploy old+new reader -> resumable backfill ->
quarantine invalid -> verify counts/checksums -> atomic activation
-> SchemaPublished -> retire old write path after window.

Expand/contract: never remove old predicate while requiring new —
add/map, backfill, switch writes, verify, retire last.

MigrationPlan: source/target, ordered primitives, deterministic
function/normalizer versions, conditions, conflict/quarantine
policy, chunk key/size, retry key, expected counts, reverse/
compensation, dependencies, owner/approval. No user code — safe
versioned DSL.

Required-field strategies: deterministic default; connector
enrichment + review; manual task; required_for_new_only + marker;
quarantine/read-only. Fake "unknown" forbidden — model absence or
explicit semantic value.

Scalar conversion: converted|ambiguous|invalid|overflow; raw kept;
converted published; rest -> issues w/ row context.

Normalizer change: shadow keys -> collisions/splits -> classify ->
review -> atomic switch -> old keys as aliases; never silent merge.

Split/merge predicate: source kept, targets + mapping; conflicts ->
marked, not last-write-wins.

Reify edge: A —owns-> B -> entity R (owner/owned roles + share/
period/source); reverse only if qualifiers expressible (else
residual).

Rollback: compensating revision + projection switch; facts never
deleted; side effects get compensation workflow.

Concurrent writes: current-revision validation + event; dual
projector into building generation; cutover when watermark +
catch-up equal.

API migration: DTO kept to deprecation; compatibility registry maps
aliases/old IDs; stable IDs preferred.

Acceptance: round-trip preserves facts; repeat migration no dupes;
resume at boundaries; concurrent write visible; invalid reported not
coerced; old API in window; policies mapped before activation.""",

"kb.docs.13": """File: docs/13-events-jobs-and-automation.md.

Domain events (commit facts): Entity*/Statement*/Schema/Policy/
Check/RiskFactor/Assessment/Task/Monitor/Report/Projection variants.
Envelope: event_id UUIDv7, type/version, tenant, aggregate/public ID
+ revision, occurred/recorded, actor, correlation/causation,
classification, payload. Minimal PII.

Outbox: mutation + outbox same tx; dispatcher FOR UPDATE SKIP
LOCKED, lease, publish, attempt/error, backoff+jitter, dead-letter,
published_at after ack. Consumers must be idempotent.

Inbox: consumer+event+version registered; completed replays
outcome; lease stealable after expiry; mark after local effect
committed.

Job SM: queued -> leased -> running -> waiting_external/waiting_user
-> succeeded/partially_succeeded/failed/cancelled. Progress num/
denom, phase, heartbeat, attempts, deadline, lease, cursor,
artifacts; cooperative cancel + cannot_cancel_after.

Worker classes: projections, imports/exports, connectors/checks,
risk/workflow, reports, notifications, maintenance — own
concurrency/priority so bulk can't starve critical work.

Scheduler: IANA zone + calendar expr + misfire/concurrency; DST
nonexistent -> skip/run-next, repeated -> once/twice; uniqueness
schedule+occurrence; leader/lease single enqueue.

WorkflowDefinition: states/transitions, triggers (event/manual/
schedule), safe-DSL conditions, required capabilities/scope,
actions, timers/SLA, assignment, escalation, compensation, version.
Instances pinned or migrated explicitly.

Check scenario: request -> identity resolve -> validation -> task
-> connector jobs -> provenance statements -> risk recompute ->
risk task -> decision entity -> notifications -> monitor schedule
-> report job; all in timeline + audit.

MonitorDefinition: target, sources, interval, detector, criteria,
quiet period, notify/workflow, legal basis. Run: pin snapshots ->
observations -> canonical compare -> noise suppress -> ChangeDetected
-> risk/workflow -> next_run. Source error != absence of risk.

Reactions registry: create task/case, change category, start check/
report, e-mail/webhook/message, notify external, request approval,
escalate — each with schema/permission/idempotency/timeout/retry/
compensation. Arbitrary shell forbidden; on-prem scripts via
sandboxed connector runner only.

Notifications: versioned template, locale, channel, recipient,
classification, redaction; delivery ledger prevents resend; no
secrets in subject/push.

SLA: versioned business calendar; due instant computed once +
stored; edits never rewrite old deadlines.

Failures: transient|rate_limited|auth|validation|permanent|
policy_denied|cancelled; retry only transient/rate_limited.

Ops UI: queues/lag/leases/dead letters/source health/watermark/
job details; retry needs capability.""",

"kb.docs.14": """File: docs/14-connectors-and-risk-engine.md.

Connector SDK: descriptor/capabilities, credential validation,
health/quota, search/fetch/monitor, versioned schemas, normalization
mapping, rate limit/retry, legal/licensing metadata, redacted
diagnostics. Core never knows concrete API fields — connector
returns SourceObservation + mapping.

Required sources: internal DB, АСУ, Евразия, Контур.Фокус, ФНС,
ФССП, ВАС/successor, ФМС/successor, bad-supplier registry, Вестник
госрегистрации, Росздравнадзор, Internet, social. Each fixes owner/
endpoint/contract/license/jurisdiction/retention/purposes first.

Instance config: type/version, ops, environment/URL/proxy, secret
refs, account, quotas, timeout/retry, cache, legal basis/purpose,
permitted types/fields, retention/raw policy, trust rank, mapping
version. Secrets runner-only.

Pipeline: PEP (check.run + field access) -> minimal input -> remote
idempotency key -> quota -> call (timeout/circuit) -> raw to
encrypted blob/hash -> parser observations -> mapper + provenance ->
identity resolution (exact/candidate) -> risk event -> audit.

File import: op file_import; per-row external ID/fingerprint;
preview/conflicts; never overwrites user values — new source-specific
assertion + resolution.

Trust/resolution per predicate: manual_verified first; authoritative
rank; freshest-in-class; consensus; keep_all+conflict; derived
formula. Effective = explained projection; high-impact auto-change
may need approval.

CheckDefinition: targets, inputs, connector ops, deps, freshness/
cache, criteria package, artifacts, timeout/SLA, retry, assignment,
purpose, report profile. CheckRun pins versions.

Risk DSL (no arbitrary code): compare/range/existence/count;
temporal; string exact/normalized set; relation exists/path
(bounded); source freshness/trust/conflict; decimal arithmetic;
all/any/not; dictionary lookup; previous assessment change.
Criterion: ID/version, inputs, severity, weight, explanation key,
evidence selector, interval, owner, tests.

RiskFactor: criterion/version, target, detected/resolved, severity/
contribution, status, evidence, explanation, suppress/override.
Upsert by target+criterion+evidence window.

Assessment: target/as_of, model/version, watermark/fingerprint,
factor refs, component + final scores/category, uncertainty, actors,
status draft/approved/superseded, explanation. Score != decision.

ЛТК/отсрочка/ОДЗ/ПДЗ = package financial predicates; risk model
references by ID.

Auto-reactions: threshold -> task/alert/report/recheck/notify;
high-impact -> workflow approval; cooldown + dedup key mandatory.

Internet/social class: allowlisted providers/queries, ToS/legal,
minimization, provenance URL+time, hash/snippet limits, human
verification, injection defense, mention != fact.

Adapter tests: fixtures, sandbox, drift, rate/timeout/retry,
malformed, Unicode, duplicates, legal flags, redaction,
reconciliation.""",

"kb.docs.15": """File: docs/15-import-export-and-reporting.md.

Heavy op = job; upload/download separate; input immutable + hashed;
versions pinned; reproducible + audited.

Formats: import XLSX/CSV/TSV/NDJSON/JSON; export +DOCX/PDF; legacy
XLS via isolated converter. Untrusted: size/zip/formula/macro/
external-link/MIME + AV.

Import: upload+scan -> structure inspect -> mapping profile ->
preview+sample -> dry run -> approval for mass change -> chunked
per-row-idempotent exec -> reconciliation -> error file -> audit +
retention.

MappingProfile: sheet/range/headers, column->predicate/path, typed
parsing (type/locale/date/decimal), normalizers, identity policy,
relation creation, provenance, empty/null semantics, defaults,
conflict action, formula handling, row filter, version/tests. Empty
cell != delete — explicit marker required.

ЮЛ Excel mapping: request fields -> check_request; ОПФ/name/brand ->
legal_entity statements; ИНН/ОГРН -> identity keys; ОКВЭД ->
classifier relation + interval; capital -> money + as_of; registrar
-> org relation; addresses -> address entity + typed relation;
license -> license entity; contacts -> contact entities; manager ->
person + management relation; founder/share -> ownership relation
entity; affiliations -> distinct predicates + evidence.

Dedupe: row key = file hash + mapping version + sheet + row + op;
semantic identity across files; repeat job -> prior outcomes; fuzzy
-> review queue.

Formulas: computed value + formula kept; never executed; external
links/macros forbidden; recalculation via controlled service.

ExportDefinition: AST, columns/paths, translations, formats,
sorting, template, limits, classification. Runtime: reauthorize ->
snapshot -> chunks -> masks before writer -> escape =,+, -,@ ->
watermark -> optional encrypt/sign -> artifact TTL.

ReportDefinition: dataset query, sections/tables/charts/summaries,
locale/tz, template version, formatting, confidentiality, limits,
audiences/formats, snapshot, schedule. No SQL in template; declared
dataset only.

Mandatory SKR reports: ФЛ/ЮЛ dossiers, holding composition, check
result+evidence, checks by period, employee/decision/unit stats,
graphical stats, rejection stats, payment discipline/limits/
deferments, collateral + recheck monitor, usage + SLA.

Collateral statuses: >30d | overdue | <=30d | unassigned |
possibly-closed; color = presentation token.

Security: per-chunk current authz OR approved snapshot grant
(TTL/definition/fingerprint); schedule = own narrow grant + recipient
check; hidden fields can't reach template.

Delivery: short-lived download authz, checksum, no-cache; e-mail =
link; attachments only encrypted/approved; ledger.

Retention: classification TTL/legal hold; recoverable removal;
metadata/audit kept.

Acceptance: round-trip IDs/Unicode/decimal/date; no formula
injection; no hidden field anywhere; 100k/1M flat memory; dup job
-> no dupes; localized errors; golden DOCX/XLSX/PDF.""",

"kb.docs.16": """File: docs/16-nuxt-architecture.md.

Stack: Nuxt 4, Vue 3 Composition API, TS strict, Nuxt UI 4, i18n,
Pinia/useState, generated typed client from OpenAPI, Vitest+VTU+
Playwright, axe-core; lockfile-pinned.

Rendering: authenticated = SPA (ssr:false — no SSR/cache leak,
simple WS/session); public pages may SSR w/ own cache policy. Nuxt
server holds no business state, never touches PG; Nitro proxy only
manages cookies/headers/CSRF -> Rocket.

Structure: app/{pages,layouts,components/{entity,graph,query,
workflow,admin},composables,stores,middleware,plugins,assets,
error.vue} + layers/{core,skr} + generated/api + i18n/locales +
tests.

API client (only wrapper): base URL, request-ID/locale/tz, CSRF,
timeout/abort, 401 refresh singleflight, typed errors, ETag/If-Match,
idempotency keys, cursor, no retry w/o key, redacted. Components
never call $fetch.

Session: GET /me/context -> profile, locale/tz, capabilities,
workspaces, policy revision, grants. Middleware = UX-only check;
server re-decides. PolicyChanged -> invalidate/cancel/refetch/
redirect. No tokens in storage; no sensitive drafts in localStorage.

Metadata-driven UI: EntityFormRenderer (revision + ViewDefinition +
values + field permissions + locale + validation). Field registry:
text/textarea/rich(sanitized), boolean, integer/decimal/money/
measurement, date/time/datetime/tz, enum/dictionary, entity ref/
autocomplete, multi-value/repeating, address/contact/document, file,
derived, relation editor — unknown -> safe unsupported, never raw
JSON. EntityListRenderer: columns/filters/facets/sort/actions/export
from view; virtualization + cursor; prefs by ID.

State: key = tenant+resource+revision+view+locale+policy; draft
separate; optimistic only reversible low-risk; 412 -> conflict UI;
jobs via events+fallback; graph keyed + cursors + visited.

Layout: header (workspace/search/jobs/locale/profile), collapsible
nav by capabilities + schema menu, breadcrumbs, context panel,
command palette, environment banner. Brand tokens configurable.

Search: exact-ID + full-text + filters + recent/saved + keyboard +
filtered highlights + debounce + no classified text in analytics;
AST builder; batch/forbidden shown w/ reason.

Performance: lazy routes, split admin/report/graph, cursor/virtual,
summary-first + lazy tabs, progressive graph, no client-side big
filtering, no N+1, CI budgets.

Validation: server authoritative; errors bound to predicate IDs;
draft survives 422/412; decimal string until submit.

A11y WCAG 2.2 AA: keyboard/focus, skip links, landmarks, label
associations, table headers, accessible graph tree, color not sole
carrier, reduced-motion, 200% zoom, RTL, SR announcements.

Client security: CSP no unsafe-eval, sanitize source content, no
v-html, rel=noopener, API downloads, sandboxed preview, no secrets
in runtimeConfig.public, SCA, clickjacking, auto-lock, step-up.

Errors: error.vue per class + request ID; no stack/payload.""",

"kb.docs.17": """File: docs/17-ui-screen-specification.md.

Menu from packages + capabilities: Главная, Объекты, Заявки и
процессы, Проверки, Мониторинг, Риски и решения, Отчёты,
Документация, Администрирование, Интеграции, Аудит. Legacy items =
saved views/types, not code.

Dashboard: my tasks/overdue, requests, checks, critical factors,
monitors due, reports/jobs, scoped stats, source health — aggregate-
leak-safe, as_of + drill-down.

Catalog /objects/[typeAlias]: saved views, simple/advanced search,
filters/facets, columns/sort, cursor table, bulk via server-side
token, capability-gated actions, status/freshness.

Card /entities/[publicId]: header (name + badges, lifecycle/risk/
classification, IDs by permission, freshness/conflict, actions,
revision/as_of). Tabs: Обзор | Связи | Граф/дерево | Проверки |
Риски | Процессы/задачи | Документы | История. Hidden tabs leave no
headers/counts.

Create/edit: revision form, required/conditional, repeating values/
relations, exact ref search, duplicate suggestion pre-commit, raw/
canonical preview for identity fields, draft, server validation,
create-and-open/add-related, unsaved guard, expected revision.
Exact dup -> open existing or explicit distinct; fuzzy -> candidates,
never auto-merge; hidden entity -> generic conflict.

Related: per-predicate groups; qualifiers/validity/source; add/link;
safe inline edit; counts; direction labels; effective/historical.

Graph: toolbar (as_of, depth, predicates, direction, layout, hide
resolved, export); start depth 1; on-demand expand; Security budget;
cycle = link-back; edge label/period/source; high-degree paged;
hidden not inferable; accessible tree alternative.

Search: exact/text toggle, type chips, external = check job (never
sync browser call), policy recents, safe highlights, hidden types
absent.

Check request: new/existing, initiator, category, target/create,
product/limit/deferment, purpose/urgency, docs, due, profile;
timeline = assignment/sources/factors/decisions/SLA.

Check screen: summary, definition/version/status, source states
(error != "nothing found"), grouped results, conflicts/provenance,
criteria/factors, notes/evidence, run/retry, approve/reject/
request-info, report.

Risks/decision: factors (severity, criterion, age, evidence,
explanation); assessment (score/category/model/freshness); decision
form (outcome, reasons, conditions, limit/deferment, validity,
approvals); override = capability/reason/expiry.

Reports: categories (ФЛ/ЮЛ checks, time/employee/decision stats,
graphical, rejections, collateral); params scope/dates/employee/
product/branch/decision/format/locale; async + estimate + progress.

Collateral monitor: filters (client/ИНН, product, contract, type/
status, responsible, ranges, overdue, closed); statuses >30d/
overdue/<=30d/unassigned/possibly-closed + icon/text/token.

Personas: security service (checks/evidence/sources/factors/
affiliations); manager (requests, limits, discipline, ОДЗ/ПДЗ);
risk officer (ЛТК, deferment, assessment, factors, approvals).
Combined roles = policy intersection/union + deny-overrides.

Jobs drawer + deduped notifications. Responsive: narrow -> drawer +
selected columns; 35-col Excel never just shrunk.

Acceptance: legacy scenarios reachable; publish -> live UI; hidden
data absent (DOM/network/counts/graph); 100k virtualized; cycle
safe; a11y/RTL; status != color; dup -> existing ID; policy change
clears cache.""",

"kb.docs.18": """File: docs/18-admin-console.md.

Principle: managed versioned-resource editor; every change = draft
-> validation -> impact -> approval -> publish -> audit; high-risk
-> MFA/step-up.

Schema catalog: packages/types/predicates/revisions/deps/usage;
draft from published, add/inherit type, add predicate, cardinality/
domain/range/temporal, identity/normalization, validation/index/
search, classification, translations, views, validate/impact/
publish/retire. Stable IDs immutable; confusable-alias check.

Form/View designer: sections/tabs/columns, field components, safe-
DSL visibility, list filters/sorts, relation groups, capabilities,
responsive/persona/locale/RTL previews, a11y warnings; never unhides
policy-hidden field.

Migration center: source/target, steps, preview, affected rows,
invalid/ambiguous/collisions, estimates, dry run, approval,
progress/checkpoint, quarantine, verification, cutover/compensation.
Blocked on unmapped security rule or required issue.

Users: identity status, both group classes separately, intervals,
MFA/session/device, revoke, service accounts, review, impersonation/
delegation audit. Secrets never shown.

Capability groups: function/action matrix, default deny, acyclic
composition, effective preview w/ origin rules, no data selectors.

Data groups/policies: target tenant/type/entity/predicate/relation/
classification; actions read/create/edit/retire/traverse/export/
history; conditions, fields/masks, validity/purpose, allow/deny,
owner/legal basis. Simulation: persona + example -> obligations/
plan + before/after + side-channel warnings; no raw SQL.

Four-eyes: author submits -> independent approver (diff/impact) ->
step-up -> publish -> audit; no self-approval; emergency =
break-glass + post-review.

Connectors: catalog/instance, ops, endpoint/proxy, secret presence/
rotate (write-only), license/purpose, quota, mapping version,
redacted health, test, enable/disable, retention, monitors.

Editors: criteria builder (typed expression, inputs, severity/
weight, explanation, evidence, version/dates, tests, synthetic
preview); workflow (states/transitions, tasks, SLA/calendar,
conditions, actions, escalation, compensation) + linter (unreachable,
no terminal, unguarded loops, missing perms).

Reports: definitions/templates, params, dataset AST, formats,
classification, schedule, recipients, limits; synthetic/authorized
preview; allowlisted variables.

Translations: key x locale matrix, completeness/fallback, ICU
plural preview, overflow, RTL preview, bundle import/export,
draft/review/publish; no auto-translation of business data.

Jobs/projections: queues/lag/dead letters, retry/cancel,
generations/checksums; rebuild = capability/reason + new generation;
watermark not hand-edited.

Audit explorer: actor/action/resource/time/decision filters; safe
diff, revisions, original/effective actor, reason, integrity;
export = separate right.

Settings: tenant locale/tz/retention, brand, limits, origins, OIDC
refs, calendars, feature flags, maintenance windows; DB URL/secrets
not editable.

Protection: env banner, destructive confirm w/ name/reason, impact/
dry run, optimistic concurrency, undo via new revision, estimate
before bulk, time-limited grants, stable-ID diffs, orphan-ref
prevention, observable publish locks.

Acceptance: schema built+published via UI; rename keeps ID/data;
collisions surfaced; person-deny simulation; both group classes;
secret unreadable; RTL preview; second approver; audit links
draft->approvals->publish.""",

"kb.docs.19": """File: docs/19-internationalization.md.

Basics: UTF-8 end-to-end; PG UTF8; locale = BCP 47; tz = IANA;
API keys/aliases ASCII lower; labels = Unicode; raw value kept,
canonical/search separate; never change meaning via global lower/
trim/NFKC.

Normalization: default NFC; NFKC only per analyzed identity/search
field; normalizer version stored. Check bidi/zero-width controls,
confusables, grapheme-safe truncation, combining marks, emoji/non-
BMP, invalid UTF-8 before serde, filename normalization (name never
a storage key).

Collation: identity equality on canonical bytes (not OS collation);
user sort = versioned ICU + locale + public-ID tie-breaker; upgrade
-> REINDEX plan.

Translations: LocalizedText = resource kind/id + field + locale +
revision; fallback e.g. fr-CA -> fr -> tenant default -> en; API =
IDs + localized display. Translated: labels/plurals/descriptions/
help/error keys/menu/templates. IDs never translated.

User data: lang-tagged variants; no auto MT without provenance;
legal name official; display/transliteration = separate statements.

Names: no assumed first/middle/last — optional components, full
legal name, native script, transliteration, per-locale order,
mononym, patronymic, honorifics; legacy ФИО keeps components.

Address: country/areas/locality/postal/street/unit + formatted
lines + raw; composition country-dependent.

Phone: country-context parse, E.164, ext separate. E-mail: IDNA/
UTS-46 domain, local part kept per policy. URLs: IDN + canonical
ASCII host + Unicode display + anti-phishing.

Time: instant = UTC; user/tenant/source zone stored; local date
never forced UTC; date-only stays date; schedule = rule + zone +
DST policy; API explicit on semantics.

Numbers/money: decimal-as-string w/ dot; UI locale parse + Intl;
money = amount + ISO currency; percents ratio/percent; units
versioned; Excel = typed cell value.

Messages: ICU params/plurals/select; no concatenation; message_key
+ typed params; HTML forbidden/sanitized.

RTL: dir by locale/content; logical CSS; semantic icon mirroring;
bidi isolation; user bidi controls visualized/sanitized.

Search: SearchProfile = analyzer + stemming + stop words +
transliteration; exact-ID separate; grapheme-safe filtered
highlight.

Reports/files: localized fonts (CJK/Arabic/Cyrillic), RTL DOCX/PDF,
XLSX typed values, CSV UTF-8, safe filenames + Unicode titles,
locale fallback.

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

STRIDE: Spoofing -> OIDC PKCE/state/nonce, MFA, secure cookie,
refresh rotation+reuse detection, mTLS, Argon2id, revocation.
Tampering -> parameterized sqlx, stable IDs, four-eyes, signed
bundles/hashes, immutable revisions, outbox/inbox, checksums,
tamper-evident audit. Repudiation -> original/effective actor,
decision IDs, reason, immutable events, trusted timestamps.
Disclosure -> PEP rewrite before SQL, RLS, allowlist, aggregate
policy, tenant/policy cache keys, redaction, classified search,
encryption, short downloads. DoS -> limits, cost guard, bounded
queues, statement_timeout, keyset, async jobs, scanning, quotas/
breakers, backpressure. Elevation -> default deny/deny-overrides,
two classes, scoped service identity, step-up/four-eyes, break-glass,
simulation, negative tests.

Specific: SQLi (no interpolation; catalog identifiers); IDOR (UUID
grants nothing; hidden -> 404); mass assignment (deny unknown
fields; system fields never from body); XSS (escaping, sanitizer,
CSP, sandbox); CSRF/CORS (token + SameSite + Origin allowlist);
SSRF (endpoint allowlist, DNS/IP validation, egress proxy, redirect
revalidation, blocked private ranges); files (signature over
extension, zip limits, macro detection, AV, isolated converter, no
execution); formula injection (escaping, allowlist, no execution);
graph inference (denied excluded each step; counts/facets/paths
authorized); cache leak (tenant/shape/locale/revision keys; no
shared private caching); log leak (allowlist; no secrets/PII);
supply chain (lockfile, audits, signatures, protected CI, SBOM,
reproducible, secret scanning).

Encryption: TLS 1.2+; encrypted volumes; envelope for secrets;
per-tenant/purpose key IDs; KMS rotation; keyed identity hashes;
encrypted backups; never in repo/logs. Never replaces access
control.

Lifecycle: classification -> retention/archive/legal hold/export/
deletion; GDPR via jurisdiction; erasure vs hold documented;
crypto-shredding after dependency plan.

Gates: authz matrix; cross-tenant properties; hidden field/filter/
sort/facet/graph/report/export; deny conflicts; RLS pool leak;
CSRF/CORS/IDOR; SQL/AST fuzz; upload/zip/formula; SSRF; dependency/
container scan; secrets scan; pentest pre-prod. Confirmed leak =
release blocker.

Incident: detect -> revoke -> disable -> preserve evidence ->
assess -> notify -> patch -> rotate -> verify -> postmortem.""",

"kb.docs.21": """File: docs/21-observability-and-operations.md.

Logs (structured JSON): timestamp/level/service/version/env, request/
correlation/trace, hashed tenant, route/action/outcome/status,
duration/error ID, plan fingerprint, job/event/connector IDs.
Forbidden: tokens/secrets, raw req/res, SQL values, PII, connector
payload; debug via redaction allowlist.

Metrics: RED (API) + USE (DB/worker) — DbExecutor queue/pool/
saturation, SQL fingerprints, outbox lag/dead-letters, job queue/
leases/heartbeat, projection watermark/lag, connector latency/
quota/circuit, PDP latency/cache/deny, auth anomalies, report rows/
bytes, PG connections/WAL/replica/vacuum/bloat, storage errors.
Bounded label cardinality.

Traces: W3C context Rocket -> PEP -> DbExecutor -> worker/connector;
no payloads; jobs via causation links.

Audit != logs: audit = evidence w/ retention/integrity + separate
access; logs = diagnostics w/ short TTL.

Health: /health/live (no external calls); /health/ready (config, DB,
PDP/policy, storage/queue, migrations); /health/startup; detailed
internal; public generic. Connector failure = degraded; authz
failure -> unready/fail closed.

SLO/budget: separate SLI per surface; maintenance per policy; budget
steers rollout; availability never hides incorrect responses.

Alerts (owner/severity/runbook/test): RLS canary (critical), auth
spike/reuse, PDP down/indeterminate, DB saturation/replica/WAL,
outbox/projection lag, stuck leases, connector auth/quota, backup
failure/age, audit chain failure, cert expiry, capacity.

Dashboards: executive SLO, API/DB perf, security/PDP, jobs/
projections, connectors, reports, PG maintenance, tenant fairness.

Backup: WAL + base backups, storage versioning/replication, bundles/
templates/config refs, KMS backup, encrypted + geo-compliant,
automated restore validation, documented RPO/RTO — exists only
after a restore drill.

DR: declare/freeze -> restore -> verify migrations/audit ->
reconcile outbox/inbox/jobs -> rebuild projections -> canaries ->
reopen read then write -> document gap. Initial RPO <=15min,
RTO <=4h.

Shutdown: drop readiness, reject long work, drain HTTP, cancel/wait
DB jobs, flush telemetry; worker releases leases, saves cursor;
outbox retryable.

Runbooks: DB down/saturated, plan regression, RLS/policy issue,
leaked credential, connector outage/drift, projection rebuild,
stuck/dead-letter job, failed migration, disk/WAL growth, backup
restore, rollback, data leak, key rotation.

Capacity: monthly forecast + planned-scaling thresholds.

Ops access: JIT/time-limited, MFA, bastion, least privilege,
approval/audit; DBA != app user; raw query = ticket + masked
replica.""",

"kb.docs.22": """File: docs/22-testing-and-quality.md.

Pyramid: domain unit/property -> crate contract -> PG repository/
integration -> security negative/info-flow -> API/OpenAPI ->
connector fixtures -> Nuxt component/E2E/a11y -> migration/upgrade/
rollback -> perf/soak/failure -> pentest/DR.

Domain properties (generated Unicode/schemas/entities/ops):
normalization idempotent; canonical deterministic; fingerprint;
symmetric edge order; cardinality under concurrency; merge/split
acyclic; migration round-trip/residual; decimal/time deterministic;
same inputs -> same risk; workflow invariants.

Persistence (real PG only): fresh + per-release upgrade;
constraints; identity/fingerprint races; optimistic locking;
rollback/outbox atomicity; SKIP LOCKED; RLS context; pooled reset;
partition pruning; plan snapshots; tz/Unicode/decimal;
serialization retry.

Security matrix (personas x actions x resources x fields x
relations x purposes): default deny; deny-overrides; two classes;
object/type/field/edge; GET/list/search/filter/sort/facet/count/
graph/history/blob; report/export/import error file; WS/event;
cache; worker/service account; impersonation/break-glass; policy
change mid-session. Negative tests check rows + DOM/network/
artifact/log/cache.

API contracts: OpenAPI snapshot/compat; strict DTO; statuses/codes;
ETag/If-Match; idempotency commit-before-response; cursor tamper;
limits/malformed; CSRF/CORS; cancellation; WS reauthorization;
consumer-driven tests.

Query: AST fuzz; operator typing; bind/no literals; security filter
in every subquery/step; cost guard; keyset completeness; null/
collation; cycles; aggregate inference; fingerprint equivalence;
injection strings.

Connectors: synthetic fixtures, sandbox smoke, drift, pagination,
rate/quota, retry/timeout, ambiguous, malformed, duplicates, mapping
version, legal flags, redaction, secret rotation.

Import/report: both supplied XLSX; multi-row founders; date/decimal/
Unicode; dedupe/retry; formula/macro/external-link; zip/MIME; error
coordinates; 100k/1M streaming memory; DOCX/XLSX/PDF golden; hidden
fields absent; locale/RTL/fonts.

Nuxt: renderer per kind; permission removes DOM/actions; personas;
412 conflict/draft; policy event clears state; virtual list; graph
cycle/hidden node; jobs/reconnect; keyboard/focus/zoom/RTL/axe;
CSP/XSS; visual regression.

Performance: scenario manifest (SHA, schema, seed/size, config/
hardware, concurrency, warmup/duration, SLO); profiles incl. lookup,
card, list, FTS, graph, writes, import/export, policy cold/cache,
catch-up. Soak 8-24h. Gates per kb.docs.11.

Chaos: kill per job phase; DB restart/failover; replica lag; storage
down; PDP/cache down; connector timeout; outbox dup/reorder; clock
skew; disk pressure; lost post-commit response. Invariants: no
duplicate, no unauthorized fallback, resumable, explicit status.

Static/supply: fmt/clippy; test/nextest; deny/audit; vue-tsc/eslint;
npm audit/SCA; secret scan; SBOM; container scan/sign; migration
lint; OpenAPI breaking check.

Test data: synthetic; no prod dump w/o anonymization; versioned
classified fixtures; international formats, no real IDs.

DoD: coverage % != risk; critical code needs branch/property +
mutation where feasible; done = link + tests + telemetry +
migration/rollback + docs + security/perf review.""",

"kb.docs.23": """File: docs/23-deployment-and-configuration.md.

Topologies: on-prem single site (LB -> api x N -> PG primary/
replica -> storage -> workers); HA multi-zone; dedicated tenant
shard (same release).

Containers: minimal non-root, pinned digest, read-only FS, bounded
tmp, no shell/pkg mgr, seccomp/cap drop, health endpoints, SBOM/
signature, UTC + needed locale/fonts, controlled CA trust.

Environments: local/test/staging/production; prod data/secrets
never copied down; own OIDC/DB/storage/keys/domains/connectors;
env banner outside prod.

Typed TOML: [http] public_origin/request_timeout_ms/max_json_bytes;
[database] url_secret_ref/max_connections/acquire_timeout_ms/
statement_timeout_ms/db_executor_queue; [security]
policy_cache_ttl_seconds/fail_closed/internal_mtls_identity;
[storage] backend/bucket/encryption_key_ref; [jobs] lease_seconds/
max_attempts. No passwords in files; prefixed env overrides;
unknown prod setting = startup error.

Secrets (KMS): DB cred, OIDC secret, token signing/encryption keys,
connector creds, storage keys, identity-hash keys, SMTP/webhook;
short-lived where possible; rotation overlap; never logged.

TLS/network: HTTPS/mTLS trusted-proxy contract; forwarded headers
only known proxies; DB TLS; policies browser->public, API->DB/PDP/
storage, worker->DB/storage/egress; egress allowlist; cert monitor.

CI/CD: lint/unit -> integration PG/security -> API/UI contract ->
SCA/secret/SBOM/container -> migration fresh/upgrade -> E2E/a11y ->
perf smoke -> signed artifacts -> staging -> canary -> approval ->
prod rolling/canary -> post-deploy checks.

DB deploy: expand first; sqlx migrate as separate job + advisory
lock + owner role; resumable backfill; contract after drain;
CONCURRENTLY/partition per runbook.

Rollout: compatible app first; flags off; canary; monitor; gradual;
rollback binary while compatible; irreversible -> compensation.

Nuxt: hashed immutable assets; index no-cache; same-origin API;
public config = origin/build/version; private source maps; CSP/SRI.

Readiness: secrets/config -> DB TLS -> migration compat -> policy/
schema -> storage/deps -> workers/routes -> ready. Worker never
runs unsupported schema/event version.

Scaling: HPA on CPU + queue metrics within global connection
budget; worker per queue class; no scale-to-zero for latency-
critical API/PDP; tenant fairness quotas.

Prod checklist: no default passwords; migrations verified; RLS
FORCE + grants; OIDC/MFA/CSRF/CORS/CSP; keys/certs/rotation;
restore drill; audit chain; limits; alerts/runbooks; connector
legal/quotas; locale/fonts; load/pentest; rollback/compensation.""",

"kb.docs.24": """File: docs/24-current-code-migration.md.

Prototype (legacy-prototype/, excluded): Rocket 0.5-rc, Object/
ObjectType/Field, Link/LinkType, Dictionary, User/Group/Permission,
sqlx/PG, LFU cache — PoC, not baseline.

Limitations: stale deps; plaintext DB password; format-string SQL;
lazy-static pool + block_on; String IDs; RawHtml/generic ERROR; 500
on client validation; allow-by-absence permissions; post-
materialization field security; non-universal ФЛ/ЮЛ; definition/
value mixed; no statement invariant; dynamic DDL; unproven cache;
secrets in domain types; RISE DDL; no Nuxt/jobs/outbox/policy/
provenance/observability.

Rule: never rewrite in place — new workspace alongside, strangler +
importer; old sources stay reference.

Stage 0 repo security: rotate DB password immediately; remove
tracked secrets -> ref/env; ban prototype for prod data; scanning;
read-only snapshot. History keeps secrets — rotation mandatory.

Stage 1 skeleton: apps/crates per docs/05; Rocket health, typed
config/error/context, DbExecutor, sqlx migrations; legacy routes
not auto-connected.

Stage 2 security baseline: tenant/principal/session, capability +
data groups, PDP/PEP, RLS, audit — before any data API; legacy
permissions -> draft policies + simulation.

Stage 3 meta-kernel/graph: meta/core tables, domain types,
statement families, identity/idempotency/outbox; schema publish +
basic CRUD. Mapping: ObjectType->TypeDefinition; Field->
PredicateDefinition; Object.filled.fields->Entity+scalar
statements; Link->edge/reified relation; Dictionary->classifier.

Stage 4 extraction: read-only parameterized extractor; immutable
source records; normalized IDs/values; mapped domain; row-level
quarantine; old-ID aliases; count/relation/hash reconciliation.
Idempotent by source fingerprint + table + PK + mapping version.

Stage 5 SKR package: bundle ФЛ/ЮЛ/holding/contact/document/address/
request/check/risk/collateral + views/identity/workflow/
translations/Excel mapping; no structs in core.

Stage 6 Nuxt: login/layout, dynamic list/card/form/relations/tree,
checks/reports/jobs, admin; legacy read-only meanwhile.

Stage 7 dual-run: new writes to new canon; legacy one-way
projection only; UAT; freeze legacy; delta import; switch; archive;
decommission.

Reconciliation per table/type: source/imported/quarantined/skipped,
merges/candidates, statements by predicate, orphan links, tz
precision, null/empty, dup keys, checksum buckets, sampled compare;
cutover blocked on delta.

API compat: separate adapter (old paths -> new commands + security
decision + deprecation deadline); RawHtml/generic ERROR never
carried.

Reusable: domain knowledge/scenarios, Rocket organization, type/
field/link/dictionary concepts, auth reference, fixtures + SQL
model, Cargo.lock. Never reused: SQL string builders, secret config,
allow-by-absence, dynamic DDL.

Acceptance: rows accounted; old ID resolves; links/provenance/
history kept; no dup identity; permissions not widened; secrets
rotated; scenario equivalence; SLOs; rollback rehearsed;
decommission approved.""",

"kb.docs.25": """File: docs/25-delivery-plan-and-acceptance.md.

Phases + exits:
- P0 Foundation: workspace, CI, config/secrets, Rocket health/
  error/context, DbExecutor, migrations, observability, dev stack.
  Exit: safe app connects; backup smoke; no secrets; SCA.
- P1 Security/meta-kernel: tenant/identity/session, two group
  classes, PDP/PEP/RLS/audit, draft/publish. Exit: default-deny
  matrix; schema publishes; leak tests pass.
- P2 Entity graph: statements, identity/idempotency, CRUD/history,
  AST/keyset, relations/graph, projections/search. Exit: arbitrary
  type without code; dedupe; 100M SLO.
- P3 Nuxt core/admin: auth/layout, dynamic UI, jobs, admin, i18n/
  a11y. Exit: new type live without deploy; role UI/E2E/RTL pass.
- P4 SKR package: ФЛ/ЮЛ/holding, request/check/risk/workflow, Excel
  import, views. Exit: DOCX/XLSX scenarios; no hardcoded objects.
- P5 Connectors/monitoring: SDK, sources, provenance, schedules,
  criteria/reactions. Exit: sandbox checks, idempotent monitoring,
  legal/quota controls.
- P6 Reporting/production: reports/exports, perf, HA/DR, pentest,
  cutover. Exit: checklist, restore/rollback drill, signed release,
  UAT.

Connector order: internal DB -> supplied imports -> authoritative
tax/company -> enforcement/court/procurement/registration/health ->
Internet/social last.

DoD: requirement link; designs; security classification/policy;
migrations/rollback; tests incl. negative; telemetry; i18n/a11y;
perf/limits; runbook; docs; clean deps; review.

E2E scenarios:
A universal "Склад" package published live, CRUD/search/report work.
B 100 concurrent same-phone creates -> one entity; retries one ID;
  distinct extension per policy.
C both ЮЛ workbooks preview/dry-run; graph mapping; repeat import
  no dupes; relations w/ provenance.
D type split/reify/rename round-trip + residual + quarantine +
  concurrent writes in target projection.
E holdings-allowed/person-denied sees zero person data anywhere
  incl. counts/graph/report/WS/cache; DB instrumentation confirms.
F field security — same ЮЛ, one sees passport/phone, other nothing
  incl. sort/facet/export; mass-assign denied.
G check: source timeout != clean; provenance; idempotent factor/
  assessment/task/decision/report/monitor.
H monitor: DST run once; repeat result no dup; real change ->
  evidence/task.
I 100M/500M SLOs; bounded memory; no OFFSET/seq scan.
J i18n names/IDN/decimal/case/DST/RTL/emoji round-trip.
K audit/DR: tamper-evident links; RPO/RTO restore; rebuild;
  canaries.

Perf acceptance: P95 targets; zero unauthorized rows; bounded
graph/export; lag target; no queue collapse; soak stable.

Security acceptance: threat model reviewed; independent pentest;
critical/high fixed; RLS/PEP matrix; MFA/four-eyes/break-glass;
rotation; scans; drills; connector legal basis.

Docs acceptance: links exist; terms consistent; examples match
contracts; traced; ADR updated; change updates docs+tests.

NOT readiness: small test only, UI-hidden button, SELECT-before-
INSERT, generic JSONB, post-filter permissions, manual SQL,
untested backup, field-policy-free report, plaintext secret,
missing perf dataset.""",

"kb.docs.26": """File: docs/26-decisions-and-tradeoffs.md.

ADR-001 modular monolith — api/worker + strict crates; no
microservice-per-context.
ADR-002 statement graph + typed families — rejected table-per-type,
one-JSONB, pure RDF.
ADR-003 stable IDs, labels mutable — UUID in API.
ADR-004 canonical facts immutable — change = revision/supersede/
revoke.
ADR-005 residual facts for lossless transform.
ADR-006 PG source of truth; no dynamic DDL in request.
ADR-007 sqlx async-std bridge — AsyncStdDbExecutor owns PgPool
(repo uses sqlx 0.7.4 vs spec 0.5.13 — kb.conventions).
ADR-008 logical Security API, local transport — in-process PDP;
mTLS for others.
ADR-009 permissions before query — obligations rewrite AST before
SQL; post-fetch rejected.
ADR-010 two group classes — capability vs data never mix.
ADR-011 default deny / deny-overrides; break-glass separate.
ADR-012 idempotency ledger + semantic keys — transport !=
uniqueness.
ADR-013 keyset — OFFSET rejected; signed cursor binds revisions.
ADR-014 jobs/outbox — external calls/bulk/monitoring never in
request tx; exactly-once via inbox/ledger.
ADR-015 Nuxt SPA — ssr:false for app/admin.
ADR-016 metadata-driven UI, security not UI-driven.
ADR-017 fuzzy never auto-merges — exact -> existing; probabilistic
-> candidate.
ADR-018 holding as graph view — temporal ownership/control/
affiliation.
ADR-019 international defaults — UTF-8/NFC/BCP47/IANA/decimal.
ADR-020 no ordinary physical delete — retire/revoke/tombstone +
regulated erasure.
ADR-021 service account isolation — bound snapshot/current auth;
own narrow grant.
ADR-022 report/export security = dataset security — no undeclared/
forbidden fields.
ADR-023 PostgreSQL 17+ baseline; newer features via ADR.

ADR change needs context + alternatives + consequences + migration
+ impact + approval + dependent updates; old ADR superseded.""",

"kb.docs.27": """File: docs/27-glossary.md.

- EngineR: universal platform core. СКР САНАР: applied risk-control
  package. Аннуитетный риск: permanently-possible risk of
  continuous activity.
- Entity: stable graph-node identity anchor. Statement: atom
  subject-predicate-entity/scalar + time/provenance (prefer
  Statement over "fact" in code).
- Predicate: meaning of a property/relation. TypeDefinition:
  versioned constraints/behavior. Meta-kernel: closed primitive
  set. Schema package: signed declarative types/views/policy refs/
  workflows/reports. Schema revision: immutable published schema.
- Residual fact: inexpressible source fact kept with lineage.
  Lineage: origin/transformation trace. Provenance: source +
  evidence. Effective view: multi-source view by trust policy.
- IdentityPolicy: exact/fuzzy identification rule. Identity key:
  canonical unique key -> owner. Match candidate: hypothesis, not
  merge. Fingerprint: deterministic canonical hash. Idempotency
  ledger: stored outcomes of repeated commands.
- Merge/Split: governed identity union/division + lineage.
  Projection: rebuildable read model. Watermark: canonical
  revision a projection reached.
- Query AST: typed JSON query tree. PEP: enforcement point. PDP:
  decision point (decision + obligations). Obligation: mandatory
  limit/mask/filter.
- Capability: system-function right. Data permission: right to
  type/entity/field/relation/row. Capability/data groups: only
  that class of rights. RLS: PG row security barrier. Purpose of
  use: declared processing purpose. Break-glass: emergency access
  + step-up/audit.
- Outbox: events atomic with mutation. Inbox: processed-events
  ledger. Job: resumable background operation.
- Connector: source adapter. Observation: source's version of
  facts. Check: sources+criteria orchestration. Risk criterion:
  versioned typed rule. Risk factor: detected condition +
  evidence. Assessment: computed score on versioned inputs.
  Decision: separate human/workflow decision. Monitor: scheduled
  recheck/detector.
- ViewDefinition: list/card/form/report metadata. Public ID:
  UUIDv7. Internal ID: compact bigint. Keyset cursor: opaque
  signed continuation.
- ЛТК: trade-credit limit (package term). ОДЗ/ПДЗ: receivables
  indicators — semantics set by the customer's package. ЮЛ/ФЛ:
  legal/natural person — package types, not core structs. Холдинг:
  temporal ownership/control/affiliation graph view.""",
}


def main():
    src = open(TARGET, encoding="utf-8").read()
    for kb_id, content in NEW.items():
        start = src.index(f'kb_id="{kb_id}"')
        cstart = src.index('content="""', start)
        cend = src.index('"""', cstart + len('content="""'))
        src = src[:cstart] + 'content="""' + content + src[cend:]
    open(TARGET, "w", encoding="utf-8").write(src)
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
