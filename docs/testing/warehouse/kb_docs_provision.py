"""Extend the project KB: kb.self (how to write KB articles) + one
kb_article per normative spec in docs/. Everything goes through MCP
tools/call — no raw SQL, no raw HTTP API.

Run:  python docs/testing/warehouse/kb_docs_provision.py
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kb_provision import rpc, call, failed, show  # noqa: E402

ARTICLES = [
    dict(
        kb_id="kb.self", sort=0, category="meta",
        title="How to write to this knowledge base",
        summary="Write recipe for agents: kb_article field names/kinds, "
                "entities.create signature, kb_id dedup semantics, "
                "kb_related edges, kb.<domain>.<topic> naming, and the "
                "schema draft lifecycle for catalog changes.",
        tags="kb,mcp,howto,write,schema",
        content="""This KB is data, not code: every article is a `kb_article`
entity; links use the `kb_related` edge predicate (kb_article ->
kb_article). ALL writes go through MCP tools/call at
http://localhost:8020/mcp (see kb.mcp.contract for the handshake;
see kb.onboarding for read/query recipes).

## kb_article fields (published schema revision 1)

  kb_id        text     identity + required — the stable key
  kb_title     text     required — human title
  kb_category  text     required — e.g. meta|spec|mcp|ops|data|schema|...
  kb_summary   text     2-4 sentence digest
  kb_content   text     full condensed reference
  kb_sort      integer  ordering hint (spec articles use the doc number)
  kb_tags      text     comma-separated topic tags

## Creating an article

  entities.create {
    "type_id": "kb_article",
    "fields": {"kb_id": "kb.<domain>.<topic>", "kb_title": "...",
               "kb_category": "...", "kb_summary": "...",
               "kb_content": "...", "kb_sort": <int>, "kb_tags": "..."}
  }
`identity_mode` defaults to deduplicate; `reject`/`create_candidate`
also exist. `kb_id` IS the identity key: re-creating with the same
kb_id returns outcome `deduplicated` with the SAME public_id — it does
NOT replace content. To amend an article use:

  entities.update {"id": "<public_id>", "fields": {"kb_content": "..."}}

Find the public_id first via entities.query
{"query": {"from": {"type_id": "kb_article"},
           "where": {"field": "kb_id", "op": "eq", "value": "<kb_id>"}}}
or entities.list {"type": "kb_article"}.

## Linking articles

  relations.create {"id": "<subject public_id>",
                    "predicate": "kb_related",
                    "target_entity_id": "<object public_id>"}
Duplicate active relation -> 409 (idempotent, not a bug).

## Naming convention

kb_id follows `kb.<domain>.<topic>`: e.g. kb.mcp.contract,
kb.docs.06, kb.schema.lifecycle, kb.self. Keep dots hierarchic:
<area>[.<subtopic>].

## Changing the schema itself

New fields/types/predicates are a SCHEMA change, not a data write:
draft_create -> draft_update(if_match) -> draft_validate ->
draft_impact -> draft_get (fresh etag) -> draft_publish(reason,
if_match). See kb.schema.lifecycle. Never raw SQL, never raw API."""),

    dict(
        kb_id="kb.docs.00", sort=0, category="spec",
        title="Видение, назначение и границы",
        summary="Vision and product boundaries: EngineR is a universal "
                "platform where new object types/fields/relations/checks/"
                "reports/processes are added without core code changes; "
                "the first domain package is СКР САНАР. Lists the 10 "
                "design principles, functional contours, and "
                "non-functional targets.",
        tags="vision,scope,principles,slo",
        content="""File: docs/00-vision-and-scope.md (normative; Russian).

EngineR: universal platform for interconnected information systems; new
domains are described declaratively (types, fields, relations, identity,
views, rights, workflows, reports) and become available to API/Nuxt
after publish — no Rust/Nuxt recompilation. First package: СКР САНАР
(early detection of annuity risks in continuous financial activity).

10 principles: flexibility without perf loss (typed indexes/projections);
meaning over form (stable IDs, mutable labels); nothing lost silently
(residual facts + provenance); double idempotency; security before
selection (forbidden rows/types/fields excluded IN SQL); no domain
hardcode (person/legal_entity are a config package); audit by design;
international by default (UTF-8, IANA tz, Unicode norm, RTL); modular
monolith before distribution; stable API (versioned DTOs, not tables).

Functional contours: universal data (types/predicates/CRUD/graph/
search/import/export), risk control (requests, cards, connectors,
criteria/factors/assessments/decisions/evidence, monitoring, tasks/
SLA, reports), administration (schema editor, two permission classes,
users/service accounts, connectors/secrets/quotas, translations,
workflows, jobs, audit).

NFR targets: hundreds of millions of active statements + billion-row
history; P95 card read 200ms, first search page 500ms, single write
500ms; no OFFSET on large sets; no unbounded graph traversal or export
in request thread; horizontal API/worker scaling; >=99.9% availability.

Out of scope: legal interpretation of national sources, concrete
criteria/weights/limits/customer brand, browser DB/connector access,
arbitrary user SQL/JS/Rust, synchronous heavy reports, unmanaged fuzzy
auto-merge."""),

    dict(
        kb_id="kb.docs.01", sort=1, category="spec",
        title="Дополнительные материалы и трассировка требований",
        summary="Digest of the original DOCX/XLSX source materials "
                "(Тезисы, ТЗ для СКР, Экраны СКР, Для СКР по ЮЛ) and the "
                "requirements→implementation→verification traceability "
                "table. Fixes architectural clarifications: holding is a "
                "computed graph view, affiliation predicates are "
                "temporal, fuzzy match produces a candidate not a merge.",
        tags="requirements,traceability,sources,excel",
        content="""File: docs/01-source-materials-and-requirements.md.

Source corpus (unchanged): Тезисы.docx, ТЗ для СКР.docx, Экраны СКР.docx,
Для СКР по ЮЛ.XLSX + variant (35-column ЮЛ grid A1:AI17).

Тезисы: annuity-risk concept, web UI for objects/search/tree, checks +
monitoring, reports, integrations with closed/licensed/personal/
commercial/open data, Word/Excel report files. PHP/MySQL stack belongs
to the old product — functionality moves to Rust/PostgreSQL/Nuxt.

ТЗ objects: person (ФЛ), legal entity (ЮЛ), holding. Named sources are
connector-package requirements, NOT built-in tables: internal base/АСУ/
Евразия, Контур.Фокус, ФНС, ФССП, ВАС, ФМС, bad-supplier registry,
Вестник госрегистрации, Росздравнадзор, Internet/social. Workplaces:
security service, managers (ЛТК, payment discipline, ОДЗ/ПДЗ), risk
officers — roles expressed via policies and views, not code branches.

Excel ЮЛ normative mapping: request fields -> request entity;
organization (ОПФ/name/brand/ИНН/ОГРН/ОКВЭД/capital/registrar) ->
statements; manager/founder -> person/legal_entity entities + relations;
addresses/license/contacts -> own entities/statements. An Excel row is a
VIEW of the graph, not a core table.

Экраны СКР: 9 screenshots -> carried patterns: persistent navigation,
contextual title, search, section-grouped related links, tree, report
filters/status legend.

Traceability: arbitrary objects -> 03; lossless transform -> 04; no
dupes -> identity keys/fingerprints/ledger; two right classes -> 08;
exclude-before-answer -> 09/07; 100M+ -> 10/11; sources -> 14;
monitoring -> 13; Word/Excel -> 15; card/tree -> 17; i18n -> 19.

Clarifications: holding = computed ownership/control/affiliation view;
formal vs informal affiliation = distinct temporal predicates with
source+evidence; external result never overwrites a user fact without
trust policy; fuzzy match -> candidate only; tree screen = bounded
projection, cycles allowed and marked."""),

    dict(
        kb_id="kb.docs.02", sort=2, category="spec",
        title="Архитектура системы",
        summary="System architecture: a modular monolith with two Rust "
                "deployables (enginer-api, enginer-worker) sharing "
                "domain/application crates. Defines the bounded "
                "contexts, layered architecture, canonical CreateEntity "
                "and query flows, the AsyncStdDbExecutor runtime "
                "bridge, outbox consistency, and network boundaries.",
        tags="architecture,contexts,layers,monolith,outbox",
        content="""File: docs/02-system-architecture.md.

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
worker queue leases; TenantLocator for dedicated shard."""),

    dict(
        kb_id="kb.docs.03", sort=3, category="spec",
        title="Универсальная доменная модель",
        summary="The universal domain model: all data is a directed "
                "attributed multigraph of entities and statements. "
                "Defines Entity, Statement, PredicateDefinition, "
                "TypeDefinition, the closed meta-kernel, scalar kinds, "
                "provenance, ViewDefinition, the skr-core package, "
                "holdings, and the seven invariants.",
        tags="domain-model,entity,statement,predicate,metamodel",
        content="""File: docs/03-universal-domain-model.md.

All data = directed attributed multigraph: entities + statements;
types/fields/forms/rules/translations are meta-level entities/
statements. Logic unified; storage specialized.

Entity: EntityId bigint internal + PublicId UUIDv7; tenant_id,
entity_class (data|schema|policy|workflow|system), created/retired,
head_revision, canonical_entity_id (after merge).

Statement: subject, predicate, object (Entity OR Scalar — exactly
one), validity (domain truth), recorded (system history),
provenance, confidence, status, revision. Revocation = new record —
past never erased.

PredicateDefinition: stable ID + ASCII alias, domain/range,
directed/symmetric/transitive/inverse, cardinality + uniqueness,
required/temporal, identity participation, normalization/
validation, index/search profile, confidentiality class,
translations, non-semantic hints. ИНН = scalar; "managed by" =
entity statement; typing = instance_of.

TypeDefinition: allowed/required predicates, inheritance, identity
policies, validation, layout/search/report profiles, lifecycle SM;
multiple types per entity allowed.

Meta-kernel (closed): Entity, Statement, Type/PredicateDefinition,
IDs, revisions, primitive scalars, metaschema rules — change needs
code/DB migration; app schema doesn't.

Scalar kinds: text(+lang), boolean, integer, decimal, date/time/
instant/local+IANA, UUID, URI/email/phone, money(decimal+ISO 4217),
measurement, geo, schema-bound JSON, blob ref. f64 forbidden for
money/shares/scores.

Provenance: source record, times, payload hash, legal basis/license,
parser/mapping version, actor/job, confidence, evidence. Sources
never overwrite; resolution policy -> effective view + conflict.

ViewDefinition: root types, sections, collections, columns/facets,
computed values, actions, permission requirements — grants no
access.

skr-core bundle: person, legal_entity, holding_view, address,
contact_point, document, license, request, check, risk_factor,
assessment, decision, task, contract, collateral; predicates e.g.
has_tax_id/managed_by/founded_by/owns_share/affiliated_with;
identity policies for ИНН/ОГРН/document/phone/source IDs; loaded via
Admin API.

Holding = computed view over ownership/control/affiliation (date+
threshold); cycles OK.

Invariants: one object per statement; same tenant (global schema
excepted); domain/range vs command revision; active statement
immutable; canonical IDs acyclic; projection never sole store."""),

    dict(
        kb_id="kb.docs.04", sort=4, category="spec",
        title="Топологичность, унитарность и идемпотентность",
        summary="Formal invariants: one statement constructor "
                "(unitarity), lossless transformation between any "
                "published schemas via primitives + residual facts "
                "(topology), and f(f(x))=f(x) for commands, facts, "
                "workers and migrations (idempotency). Specifies "
                "IdentityPolicy, normalization, fact fingerprint, "
                "Idempotency-Key semantics, and merge/split.",
        tags="invariants,idempotency,identity,dedup,merge",
        content="""File: docs/04-topology-unitarity-idempotency.md.

Unitarity: one constructor — statement "subject—predicate->object";
property vs relation differ by object kind. Topology: finite
transform path between schemas never destroys canonical facts —
inexpressible data = residual facts. Idempotency: f(f(x))=f(x) for
commands/facts/workers/migrations.

Primitives: Add/Retire/AliasType; Add/Retire/RenamePredicate|Label;
Add/RemoveMembership; MapPredicate; SplitPredicate+residual;
MergePredicates; ConvertScalar (total or quarantine); ReifyEdge;
CollapseRelationEntity; Merge/SplitEntity; MaterializeDerived/
DiscardProjection — w/ forward, reverse, conditions, report.

Rename: aliases/labels change; type_id/predicate_id never; display
name never a key.

IdentityPolicy: scope, key components, normalizer, quality,
exact/fuzzy, collision action, resurrection. Exact: ЮЛ =
country+regnum+ОГРН; phone = E.164+ext; document = country+type+
series/number; external = connector+namespace+source ID. Name/
birthdate/address -> same_as_candidate only.

Normalization: raw + canonical + normalizer_version; UTF-8 NFC +
trim; casefold search key; ИНН/ОГРН checksum; E.164; IDNA email;
decimal scale; UTC+zone; symmetric edge = sorted endpoints. New
normalizer -> migration + collision report.

Fingerprint: tenant|canonical_subject|predicate|object_kind|
canonical_object|validity|qualifier_hash|identity_scope. Scopes:
global_fact/per_source/per_observation. BLAKE2b-256 lookup;
collisions on canonical columns.

Idempotency-Key: unique tenant+principal+route+key; same payload
-> stored response; different -> 409.

Concurrent create: ON CONFLICT one owner; second reads it —
deduplicated=true, same ID. SELECT-before-INSERT forbidden.

Merge: canonical picked, losers -> aliases, keys moved,
EntityMerged; repeat no-op. Split: plan distributes + anchor. Inbox:
unique consumer+event+handler_version.

Verifiable: normalize idempotent; repeat create = same ID; edge
order irrelevant; forward+reverse restores; rebuild checksum
stable; residual never unreachable."""),

    dict(
        kb_id="kb.docs.05", sort=5, category="spec",
        title="Структура Rust workspace и правила реализации",
        summary="Rust workspace layout and implementation rules: "
                "crate inventory and responsibilities, Cargo "
                "configuration (resolver/workspace deps/cargo-deny/no "
                "unsafe), required newtypes, application ports, the "
                "DbExecutor trait, RequestContext, layered error "
                "enums, transaction discipline, style rules, typed "
                "configuration, and feature flags.",
        tags="rust,workspace,crates,ports,conventions",
        content="""File: docs/05-rust-workspace.md.

Layout: apps/{api,worker} + crates/{domain, application, contracts,
schema-registry, identity, query-engine, persistence-postgres,
db-runtime-async-std, security-domain, security-pdp, security-api,
connectors, risk-engine, workflow, reporting, files, observability,
test-support} + migrations/ + schema-packages/skr-core/ + web/.
Deps point inward; domain never imports adapters.

Cargo: workspace versions; lock-pinned; cargo-deny; unsafe banned;
warnings denied. Spec pins sqlx 0.5.13 async-std — DEVIATION: repo
uses 0.7.4 (kb.conventions); rocket 0.5.1 json+secrets.

Newtypes: TenantId, EntityId, PublicEntityId, TypeId, PredicateId,
SchemaRevisionId, StatementId, PolicyId, DecisionId, PrincipalId,
GroupId, SessionId, RequestId, CorrelationId, IdempotencyKey, JobId,
EventId, ProjectionRevision, LocaleTag, TimeZoneId, ContentHash —
never String.

Ports: UnitOfWork/TransactionRunner; Entity/Statement/Schema/
IdentityKey repos; QueryExecutor; SecurityDecisionPoint; Outbox;
AuditSink; Clock; IdGenerator; BlobStore; ConnectorInvoker; JobQueue.
No PgRow/Request/Value crossing.

DbExecutor: run(deadline, closure over PgPool); pool in async-std;
bounded queue + cancel; panic contained; graceful drain; readiness
false on DB-down/saturation.

RequestContext: tenant/principal/session/IDs/locale+tz/auth
strength/client/deadline/policy snapshot/purpose — explicit, no
globals.

Errors: Domain/Application/Security/Persistence/Api — stable code
+ safe details; never leak SQL/stack/secrets/hidden existence.

Transactions: one boundary per use case; no external calls in tx;
lock order identity->entity->statement->outbox; retry only
serialization/deadlock; outbox for side effects; READ COMMITTED;
If-Match optimistic concurrency.

Style: fmt+clippy; no format!-SQL; secrecy/zeroize; Argon2id; UTC+
explicit zone; decimal not float; deny_unknown_fields.

Config: defaults -> file -> env -> secret provider; startup
validates DB/pool/keys/origins/storage/timeouts/refs/audit.
Feature flags: owner/expiry/default/audience/audit."""),

    dict(
        kb_id="kb.docs.06", sort=6, category="spec",
        title="Контракт Rocket JSON API",
        summary="The /api/v1 REST contract: response envelope with "
                "meta revisions, error shape with stable codes, HTTP "
                "semantics (401/403/409/412/422/429/503), auth/session "
                "endpoints, schema/entity/statement/graph/query/jobs/"
                "checks/admin surfaces, optimistic concurrency via "
                "ETag+If-Match, caching rules, WebSocket/SSE, and the "
                "centralized LimitProfile.",
        tags="api,rest,envelope,errors,endpoints",
        content="""File: docs/06-api-contract.md.

Base: /api/v1 public; /internal/v1 + /internal/security/v1 internal.
HTTPS outside dev; JSON UTF-8; decimals as strings; RFC 3339 UTC;
public IDs UUID; unknown command fields rejected; X-Request-Id +
X-Correlation-Id; OpenAPI from contracts.

Envelope: {data, meta:{request_id, schema_revision, commit_revision,
policy_revision}, links}. Error: {error:{code, message_key,
parameters, request_id}}; Nuxt localizes message_key.

Statuses: 401 auth; 403 denied (hidden -> 404); 409 identity/
version/idempotency; 412 If-Match; 422 validation; 429 +Retry-After;
503 fail-closed. DELETE = retire/tombstone; purge = separate
approval.

Auth: OIDC AC+PKCE + HttpOnly SameSite cookie; bearer for service
accounts. POST /auth/session/exchange|refresh|logout; GET /me,
/me/context; POST /auth/step-up. Refresh rotation + reuse
detection; CSRF/Origin for cookie mutations.

Schema: GET /schemas/types{,/{id}}; POST/PUT /schemas/drafts{,/{id}};
POST .../validate|impact|publish; POST /revisions/{id}/retire; GET
.../migration-status. Publish = If-Match + privilege + impact +
reason.

Entity: POST /entities; GET /entities/{public_id}; POST /entities/
query; PATCH; POST retire|restore|merge|split; GET history|related;
POST graph. Create: type_id, schema_revision, identity_mode,
statements[](predicate_id + {kind,value}) -> created/deduplicated +
revision + warnings + actions.

Statements via entity command; POST /entities/{id}/statements;
/statements/{id}/revoke|verify; GET provenance. Never change
predicate/subject/object in place.

Graph: as_of, max_depth, max_nodes, predicates, direction; limits
intersected w/ obligations; cycle = edge to visible node; signed
cursor. Query: page 50/500; OFFSET forbidden.

Jobs: POST /jobs/imports|exports|bulk-mutations; GET/cancel/errors/
artifact — dry_run, snapshots, per-row results, explicit partial
success.

Checks/reports: POST /checks, /checks/{id}/run (202+job), POST/PATCH
/monitors, POST /reports, /reports/{id}/download. Admin: /admin/
users|service-accounts|capability-groups|data-groups|policies|
policy-simulations|connectors|translations|audit|jobs; secrets
write-only.

Concurrency: ETag "entity:uuid:revision"; If-Match or
expected_revision; 412 + safe summary; force = capability + reason.
Caching: ETag binds revision+view+locale+policy; secret -> private.

Events /api/v1/events: permitted topics, field-filtered, policy
change -> reauthorize/close, cursor reconnect; events notify, client
re-reads canon.

Limits: centralized LimitProfile — body, statements/command, sort
keys, page, graph depth/nodes, export, upload, jobs, deadline."""),

    dict(
        kb_id="kb.docs.07", sort=7, category="spec",
        title="Язык запросов и безопасная компиляция",
        summary="The JSON AST query language and its safe compilation "
                "pipeline: operator set per scalar kind, the fixed "
                "parse->PDP->rewrite->compile pipeline, security AST "
                "rewrite rules, parameterized SQL emission, signed "
                "keyset cursors, cost classes, bounded graph "
                "traversal, aggregate leak controls, plan cache, and "
                "stable error codes.",
        tags="query,ast,sql-compiler,security-rewrite,keyset",
        content="""File: docs/07-query-engine.md.

AST: {from:{type_id,alias}, select, where, sort, page:{size,after},
as_of} + exists/related/aggregates/group_by/facets; joins only via
relation path.

Operators per kind: eq/ne/in/is_null; lt/lte/gt/gte/between;
contains/prefix/full_text; date overlap/valid_at; relation exists/
not_exists; geo within/distance. Regex/script/SQL/wildcard
forbidden.

Pipeline (fixed): parse+limits -> schema revision -> type-check ->
normalize -> capability -> PDP -> security AST rewrite -> index
selection -> cost estimate -> parameterized SQL -> deadline ->
response allowlist -> audit. Never string-append security WHERE.

PDP returns: allowed root types, select + relation predicates/
directions, mandatory row predicates, masks, max page/depth/export,
forbidden aggregates/facets. Rewriter intersects select, injects
filters at root + every subquery, forbids sort/facet on hidden
fields, removes denied edges, stamps policy revision. Denied root
-> false/no-rows or 404.

SQL compiler: catalog identifiers; bind params; family-grouped
predicates; partial index/status; temporal range; every join has
tenant_id; membership before wide joins; EXISTS/LATERAL not
DISTINCT.

Keyset: sort ends unique public_id; cursor = fingerprints + last
sort values + direction + tenant + expiry — signed; schema/policy
change -> 409 cursor_stale.

Cost classes: interactive | extended | batch-only (->job) |
rejected.

Graph: recursive CTE only w/ depth/nodes + allowlist + tenant/
temporal/security per step + cycle detection + deterministic order
+ cursor + timeout; hierarchies -> closure projection.

Aggregates: separate permission; fields allowed; sensitive may
need min group size/rounding/denial.

Saved queries evaluated under current policy. Plan cache: tenant +
AST + schema/policy/projection revisions.

Errors: query.syntax_invalid, unknown_type/predicate,
operator_not_supported, field_forbidden, too_complex,
requires_batch, cursor_invalid/stale, projection_lag, timeout."""),

    dict(
        kb_id="kb.docs.08", sort=8, category="spec",
        title="Архитектура безопасности",
        summary="Security architecture: separate security crates, two "
                "permission classes (system capabilities vs data "
                "permissions), the deny-by-default policy model, the "
                "evaluation sequence, the seven enforcement layers, "
                "field/edge policy semantics, PostgreSQL RLS "
                "defense-in-depth, authentication, policy cache, "
                "audit, impersonation/delegation, break-glass, policy "
                "change workflow, and fail modes.",
        tags="security,pdp,pep,permissions,rls,break-glass",
        content="""File: docs/08-security-architecture.md.

Separation: security-domain/-pdp/-api crates; every controller/use
case/query/export/worker passes a PEP. Hot path: LocalSecurityClient
(same JSON contract, no self-HTTP).

Two classes: capabilities (login/use_ui, entity.create/edit/retire/
merge, query.advanced/aggregate, check.run/monitor.manage,
report.run/export, bulk.*, schema.draft/publish, security.manage/
simulate, connector.manage, audit.read, admin.pages.access) in
capability groups; data permissions (tenant, type, entity, field,
relation/direction, provenance, classification/tag/ownership, row
condition, period, purpose) in data groups.

Rule: effect, subject/group, action, selector, condition, fields/
relations, mask/obligation, priority, interval, revision, reason.
Default deny; deny-overrides; break-glass = separate step-up flow.

Evaluation: tenant/session/strength -> memberships -> capability ->
data rules -> drop false/expired -> deny-overrides -> allowed set +
obligations -> constrain -> audit. Fail closed.

Enforcement: router guard; application PEP; query PEP rewrite;
repository (mandatory predicates); RLS; serializer allowlist; blob
decision; worker (service identity + initiator snapshot/recheck).
UI visibility never a boundary.

Field policy: allow|omit|null-mask|partial-mask|tokenize|
aggregate-only (omit preferred). Edge permission: predicate/
direction/endpoints/context — denied edge can't be fetched/
traversed/sorted-by.

RLS: separate roles; API not owner, no BYPASSRLS; tx-local
settings; FORCE RLS; connection reset. RLS never replaces PEP.

Authn: OIDC + optional local admin; AC+PKCE; MFA/step-up for
publish/export/security/break-glass; Argon2id; rotating refresh +
reuse detection; scoped service accounts + mTLS; kid'd keys.

Policy cache: key = tenant+principal+revisions+strength+purpose;
events invalidate; short TTL; high-risk bypass.

Audit: authn/decision/denied/policy change/export/blob read/bulk/
break-glass — actors, fingerprints, reasons, revisions, counts,
tamper-evident chain; no secrets.

Impersonation: capability + ticket + timebox + banner + audit;
delegation = explicit grant. Break-glass: route, MFA, reason,
minimal scope/TTL, notify, auto-expiry + review.

Policy changes: draft -> validate -> simulate -> four-eyes ->
publish; rollback = new revision.

Fail: PDP down -> closed; audit down -> high-risk closed; stale
policy -> recheck + stale cursor."""),

    dict(
        kb_id="kb.docs.09", sort=9, category="spec",
        title="Security API: контракты PDP/PEP",
        summary="The PDP/PEP contract used identically for in-process "
                "LocalSecurityClient and mTLS internal HTTP: endpoint "
                "list, shared request context, action/query/mutation/"
                "blob decision shapes with obligations, mandatory "
                "row-filter AST, mask kinds, decision binding, effect/"
                "error codes, outcome audit protocol, and internal "
                "API hardening.",
        tags="security-api,pdp,decisions,obligations,masks",
        content="""File: docs/09-security-api.md.

Endpoints: POST /internal/security/v1/decisions/action|query|
mutation|blob; POST /internal/security/v1/simulations; GET
/policies/revision; POST /cache/invalidate. Same contract for
LocalSecurityClient + mTLS; caller from mTLS/token; delegated
subject separate.

Context: request_id, tenant_id, principal {id, session_id,
auth_strength, groups_revision}, purpose, locale, time,
client_service. PDP loads membership or verifies signed snapshot —
never trusts client lists.

Action -> decision_id, effect, policy_revision, obligations
{max_rows, watermark, require_step_up, audit_level}, expires_at.

Query decision: action entity.query + query {root_type_ids,
selected_predicate_ids, relation_predicate_ids, operations,
page_size, graph_depth, query_fingerprint} — AST summary, never
raw SQL. Response: allow_with_obligations + {allowed/denied type +
predicate IDs, allowed_relations[{predicate_id, directions,
target_type_ids}], mandatory_filters (system_field ops incl.
in_subject_scope), masks, max_page_size/graph_depth/graph_nodes,
aggregates}. PEP checks policy revision before cache/cursor.

Mutation: command + target + changed predicates + relation targets
+ expected revision + batch -> allow/deny + allowed predicates +
required approvals/values + forbidden targets + transforms + max
batch + required reason + audit level. Partially-forbidden ->
denied wholesale; partial only via explicit endpoint.

Row filters: typed security AST over pre-registered system fields/
scopes; PDP never returns SQL; one trusted compiler; in_subject_
scope via scope table/CTE; huge ID lists via scope handle.

Masks: omit|null|fixed|partial|hash/token|bucket/range|
aggregate_only — predicate + algorithm + conditions; hidden fields
forbidden in sort/filter.

Decision binding: subject/session/strength + action + fingerprint +
revisions + purpose + expiry; decision_id not a bearer permission.

Effects: allow | allow_with_obligations | deny | indeterminate
(->deny) | challenge. Codes: security.unauthenticated,
capability_denied, data_denied, step_up_required, policy_stale,
pdp_unavailable, context_invalid. Public maps data_denied -> 404.

Audit: outcome (decision_id, rows/bytes, revision, status, latency,
error class) via PEP; outbox delivers later.

Hardening: mTLS + allowlist, audience-bound token, limits, replay
protection, no CORS/browser, fuzz tests."""),

    dict(
        kb_id="kb.docs.10", sort=10, category="spec",
        title="PostgreSQL: нормативная схема",
        summary="Normative PostgreSQL schema: the core/meta/sec/ops/"
                "audit/integration/report schemas, tenant and meta "
                "registry tables, entity + typed statement families, "
                "membership/current projections, identity/dedupe "
                "tables, provenance, projections, security and ops "
                "tables, audit, files/reports, partitioning, index "
                "catalog, RLS context protocol, and migration rules.",
        tags="postgresql,schema,tables,partitions,indexes,rls",
        content="""File: docs/10-postgresql-schema.md.

Principles: PG = source of truth; sqlx; DDL via migrations; JSONB
only bounded metadata/raw; UUIDv7 + bigint; tenant_id + RLS;
timestamptz; numeric finance.

Schemas: core | meta | sec | ops | audit | integration | report.

core.tenant: id, alias, locale/tz, data_region, status.
meta.schema_revision: state (draft/validating/ready/published/
retired/failed), parent, semantic_version, content_hash,
migration_plan; ONE current published. + type/predicate def +
revision + type_predicate + localized_text.

core.entity: PK (tenant_id, entity_id); public_id unique; class;
retired; head_revision; canonical_entity_id; CHECK != self. +
entity_alias + entity_revision.

Statement families: core.statement_edge|_text|_number|_boolean|
_date|_instant|_geo|_blob|_json — shared header (tenant, ids,
subject, predicate_id, schema_revision, entity_revision, valid_from/
to, recorded_at, status active/superseded/revoked/quarantined,
provenance_set_id, confidence, canonical_hash, qualifier_hash) +
typed payload (edge = object_entity_id; text = raw/nfc/search/lang;
number = value+unit/currency; json = value+schema hash). Active
uniqueness fingerprint = tenant+subject+predicate+canonical payload
+validity+qualifier+scope. NOTE: the fact-unique index embeds the
canonical object — scalar values bounded ~2.5KB by the btree limit.

core.entity_type_current: hot membership projection; partial
status=active indexes; optional header projection.

core.identity_policy + identity_key (unique tenant+type+policy+
key_hash; canonical_key maybe encrypted) + match_candidate.

integration: source_system, source_record, provenance_set/member.

ops.entity_summary (per locale, rebuildable); ops.entity_search
(tokens+tsvector; sensitive shards split); ops.typed_value_index;
ops.projection_catalog/checkpoint (watermark, generation, atomic
switch).

sec.*: principal/identity/session/credential; capability_group,
data_group + memberships (interval/revision); policy+revision+
rule; subject_scope_membership; decision_log/outcome;
break_glass_grant. Secrets separate + key ID.

ops: idempotency_record (unique tenant+principal+route+key);
outbox_event; inbox_delivery (consumer+event+handler unique);
job; job_item.

audit.event: append-only, monthly RANGE; event/actor/action/finger-
prints/hashes/checkpoint; UPDATE/DELETE forbidden.

core.blob/blob_reference; report.definition/revision/run/artifact.

Partitioning: statements HASH by tenant bucket (32/64); audit/
outbox/observations RANGE monthly; partition key in uniques.

Indexes: entity public_id/canonical; type membership; per-family
subject+predicate active; edge object+predicate; typed value;
identity unique; GIN tsvector; outbox/job/audit. Each index has
owner/selectivity/cost — no catch-all JSONB.

RLS: tx set_config(tenant/principal/purpose/policy); rejects
absence; reset after tx. Migrations via separate job; expand/
contract + resumable backfill."""),

    dict(
        kb_id="kb.docs.11", sort=11, category="spec",
        title="Производительность и масштабирование",
        summary="Performance and scaling discipline: dataset load "
                "profiles (dev/integration/scale/extreme), required "
                "SLO histograms, connection/runtime budgets, query "
                "discipline rules, export job protocol, full-text "
                "search, graph handling, partition/projection/cache "
                "rules, PostgreSQL maintenance, write amplification, "
                "sharding via TenantLocator, backpressure, and "
                "release-blocking benchmark gates.",
        tags="performance,scaling,slo,partitions,projections,cache",
        content="""File: docs/11-performance-and-scaling.md.

Profiles: dev 100K | integration 10M | scale 100M+history |
extreme 500M+; skew reproduced.

SLO histograms: exact lookup, card, list pages, FTS, relation
list, graph d1/2/3, writes, policy decision/rewrite, job enqueue,
projection lag — P50/P95/P99 + rows examined/returned + waits.

Budgets: workers != connections; bounded DbExecutor queue; pool
from DB max/instances; acquire < request deadline; batch own pool;
no connection during external HTTP/render; cancellation; tx-local
statement_timeout.

Query discipline: keyset; selective type/tenant first; EXISTS/
LATERAL; project requested predicates; fan-out limits; summaries;
bounded IN (big scopes via temp tables); no SELECT *; plan
snapshots.

Reads: identity_key unique -> summary + current; batched family
predicates; aggregated/lazy counts.

Export job: pin snapshots; keyset chunks; stream; per-chunk or
bound-snapshot policy; byte/time limits; no giant tx.

FTS: entity_search per locale; exact via B-tree; trigram for
proven fuzzy; sensitive text separate.

Graph: adjacency B-tree; hierarchies via closure projection;
arbitrary = CTE + budget; high-degree paged; cache key = revision +
predicates + as_of + policy shape; deny per step.

Partitions: hash for maintenance, never replaces tenant filter;
benchmarked count; partition key in uniques; tenant in queries;
default partition forbidden; precreated; detach runbook.

Projection lifecycle: draft -> building -> verified -> active ->
retire. Verify: bucket checksums, watermark, sampled compare,
classification, plan benchmark. Read-after-write waits watermark
or canonical fallback.

Cache: L1 bounded (immutable-by-revision); key = tenant+revision+
locale+policy shape; event invalidation + TTL; singleflight.

Maintenance: autovacuum, fillfactor, bloat/WAL/replica monitoring,
ANALYZE on skew, extended stats, REINDEX CONCURRENTLY,
pg_stat_statements, no idle-in-tx.

Write amplification: each index/projection priced; IndexProfile
per predicate; unused removed.

Sharding: partitions/replicas/projections first; TenantLocator;
tenant unsplit; separate analytics warehouse.

Backpressure: per-tenant limits; bounded queues; connector quotas;
429/503; adaptive disable; circuit breakers.

Gates: P95 regression >10% w/o ADR; unapproved seq scan; rows-
examined multiplication; post-fetch filtering; lag over SLO;
migration lock over budget; dupes or tenant leak."""),

    dict(
        kb_id="kb.docs.12", sort=12, category="spec",
        title="Эволюция схем и преобразование объектов",
        summary="Schema evolution: revision states, draft validation "
                "coverage, compatibility classes, the 10-step publish "
                "plan, expand/contract discipline, MigrationPlan "
                "contents, required-field strategies, scalar "
                "conversion, identity normalizer changes, predicate "
                "split/merge and edge reification, rollback semantics, "
                "concurrent-write handling during backfill, API "
                "migration, and acceptance checks.",
        tags="schema-evolution,migration,publish,compatibility",
        content="""File: docs/12-schema-evolution.md.

States: draft -> validating -> impact_ready -> approved ->
publishing -> published -> retired/failed. Published immutable; one
current per package/tenant.

Draft validation: aliases/IDs/translations, domain/range/
cardinality, cycles, inverse/symmetric coherence, identity
normalizers, migration totality, index/search profiles,
classification, refs, confusables, cost.

Compatibility: presentation-only | additive | conditional |
semantic | breaking | meta-kernel.

Publish plan: freeze hash -> simulate on stats -> concurrent index/
projection build -> deploy old+new reader -> resumable backfill ->
quarantine invalid -> verify counts/checksums -> atomic activation
-> SchemaPublished -> retire old write path after window.

Expand/contract: never remove old predicate while requiring new —
add/map, backfill, switch writes, verify, retire last.

MigrationPlan: source/target, ordered primitives, deterministic
function/normalizer versions, conditions, conflict/quarantine
policy, chunk key/size, retry key, expected counts, reverse/
compensation, deps, owner/approval. No user code — safe versioned
DSL.

Required-field: deterministic default; connector enrichment +
review; manual task; required_for_new_only + marker; quarantine.
Fake "unknown" forbidden — model absence/explicit value.

Scalar conversion: converted|ambiguous|invalid|overflow; raw kept;
only converted published; rest -> issues w/ row context.

Normalizer change: shadow keys -> collisions -> classify -> review
-> atomic switch -> old keys as aliases; never silent merge.

Split/merge predicate: source kept + mapping; conflicts marked, not
last-write-wins.

Reify edge: A —owns-> B -> relation entity R (+ share/period/
source); reverse only if qualifiers expressible (else residual).

Rollback: compensating revision + projection switch; facts never
deleted; side effects -> compensation workflow.

Concurrent writes: current-revision validation + event; dual
projector into building generation; cutover when watermark +
catch-up equal.

API migration: DTO kept to deprecation; compatibility registry;
stable IDs preferred.

Acceptance: round-trip preserves facts; no dupes on repeat; resume
at boundaries; concurrent write visible; invalid reported not
coerced; old API in window; policies mapped before activation."""),

    dict(
        kb_id="kb.docs.13", sort=13, category="spec",
        title="События, задания и автоматизация",
        summary="Events, jobs and automation: domain event envelope, "
                "transactional outbox with SKIP LOCKED dispatch, "
                "inbox dedup, the job state machine, worker workload "
                "classes, scheduler DST rules, WorkflowDefinition, "
                "the counterparty-check scenario, monitor runs, the "
                "reaction action registry, notifications, SLA "
                "calendars, failure classes, and the operations UI.",
        tags="events,jobs,outbox,workflow,monitoring,scheduler",
        content="""File: docs/13-events-jobs-and-automation.md.

Domain events (commit facts): Entity/Statement/Schema/Policy/
Check/RiskFactor/Assessment/Task/Monitor/Report/Projection
variants. Envelope: event_id UUIDv7, type/version, tenant,
aggregate ID + revision, occurred/recorded, actor, correlation/
causation, classification, payload. Minimal PII.

Outbox: mutation + outbox same tx; dispatcher FOR UPDATE SKIP
LOCKED, lease, publish, attempt/error, backoff+jitter, dead-letter,
published_at after ack. Consumers idempotent.

Inbox: consumer+event+version registered; completed replays; lease
stealable; mark after local effect committed.

Job SM: queued -> leased -> running -> waiting_external/
waiting_user -> succeeded/partially_succeeded/failed/cancelled.
Progress, phase, heartbeat, attempts, deadline, lease, cursor,
artifacts; cooperative cancel + cannot_cancel_after.

Worker classes: projections, imports/exports, connectors/checks,
risk/workflow, reports, notifications, maintenance — own
concurrency/priority.

Scheduler: IANA zone + calendar + misfire/concurrency; DST
nonexistent -> skip/run-next, repeated -> once/twice; uniqueness
schedule+occurrence; leader/lease single enqueue.

WorkflowDefinition: states/transitions, triggers (event/manual/
schedule), safe-DSL conditions, capabilities/scope, actions,
timers/SLA, assignment, escalation, compensation, version.
Instances pinned or migrated explicitly.

Check scenario: request -> identity resolve -> validation -> task
-> connector jobs -> provenance statements -> risk recompute ->
risk task -> decision entity -> notifications -> monitor schedule
-> report job.

MonitorDefinition: target, sources, interval, detector, criteria,
quiet period, notify/workflow, legal basis. Run: pin snapshots ->
observations -> canonical compare -> noise suppress ->
ChangeDetected -> risk/workflow -> next_run. Source error !=
absence of risk.

Reactions registry: create task/case, change category, start check/
report, e-mail/webhook/message, notify external, request approval,
escalate — each w/ schema/permission/idempotency/timeout/retry/
compensation. Arbitrary shell forbidden; on-prem scripts via
sandboxed runner only.

Notifications: versioned template, locale, channel, redaction;
delivery ledger prevents resend; no secrets in subject.

SLA: versioned business calendar; due computed once + stored;
edits never rewrite old deadlines.

Failures: transient|rate_limited|auth|validation|permanent|
policy_denied|cancelled; retry only transient/rate_limited.

Ops UI: queues/lag/leases/dead letters/source health/watermark/
details; retry needs capability."""),

    dict(
        kb_id="kb.docs.14", sort=14, category="spec",
        title="Коннекторы, проверки и движок риска",
        summary="Connectors, checks and the risk engine: Connector SDK "
                "contract, required source registry, instance config, "
                "the 11-step execution pipeline, file import as a "
                "connector operation, trust/resolution policies, "
                "CheckDefinition, the typed risk-criterion DSL, "
                "RiskFactor and Assessment shapes, limit/deferment "
                "domain fields, reaction rules, the high-risk "
                "Internet connector class, and adapter test coverage.",
        tags="connectors,checks,risk-engine,criteria-dsl,provenance",
        content="""File: docs/14-connectors-and-risk-engine.md.

Connector SDK: descriptor/capabilities, credential validation,
health/quota, search/fetch/monitor, versioned schemas, mapping,
rate limit/retry, legal/licensing metadata, redacted diagnostics.
Core never knows concrete API fields — connector returns
SourceObservation + mapping.

Required sources: internal DB, АСУ, Евразия, Контур.Фокус, ФНС,
ФССП, ВАС/successor, ФМС/successor, bad-supplier registry, Вестник
госрегистрации, Росздравнадзор, Internet, social. Each fixes owner/
endpoint/contract/license/jurisdiction/retention/purposes first.

Instance config: type/version, ops, environment/URL/proxy, secret
refs, quotas, timeout/retry, cache, legal basis/purpose, permitted
types/fields, retention/raw policy, trust rank, mapping version.
Secrets runner-only.

Pipeline: PEP (check.run + fields) -> minimal input -> remote
idempotency key -> quota -> call (timeout/circuit) -> raw to
encrypted blob -> parser observations -> mapper + provenance ->
identity resolution -> risk event -> audit.

File import: op file_import; per-row external ID/fingerprint;
never overwrites user values — new assertion + resolution.

Trust/resolution per predicate: manual_verified first; authoritative
rank; freshest-in-class; consensus; keep_all+conflict; derived
formula. High-impact auto-change may need approval.

CheckDefinition: targets, inputs, ops, deps, freshness/cache,
criteria package, artifacts, timeout/SLA, retry, assignment,
purpose, report profile. CheckRun pins versions.

Risk DSL (no arbitrary code): compare/range/existence/count;
temporal; string exact/set; relation exists/path (bounded); source
freshness/trust/conflict; decimal arithmetic; all/any/not;
dictionary lookup; previous change. Criterion: ID/version, inputs,
severity, weight, explanation key, evidence selector, interval,
owner, tests.

RiskFactor: criterion/version, target, detected/resolved,
severity/contribution, status, evidence, suppress/override. Upsert
by target+criterion+evidence window.

Assessment: target/as_of, model/version, watermark/fingerprint,
factor refs, scores/category, uncertainty, actors, status,
explanation. Score != decision.

ЛТК/отсрочка/ОДЗ/ПДЗ = package financial predicates; risk model by
ID.

Auto-reactions: threshold -> task/alert/report/recheck/notify;
high-impact -> approval; cooldown + dedup key mandatory.

Internet/social: allowlisted providers/queries, ToS/legal,
minimization, provenance, hash/snippet limits, human verification,
injection defense, mention != fact.

Adapter tests: fixtures, sandbox, drift, rate/timeout/retry,
malformed, Unicode, duplicates, legal flags, redaction,
reconciliation."""),

    dict(
        kb_id="kb.docs.15", sort=15, category="spec",
        title="Импорт, экспорт и отчётность",
        summary="Import, export and reporting: heavy operations are "
                "jobs with immutable hashed inputs and pinned "
                "versions; supported formats, the 10-step import "
                "pipeline, MappingProfile fields, the normative ЮЛ "
                "Excel mapping, per-row dedupe, formula rules, "
                "ExportDefinition runtime with masking/formula-injection "
                "escaping, ReportDefinition, mandatory SKR reports, "
                "collateral monitoring statuses, security modes, "
                "delivery and retention.",
        tags="import,export,reporting,excel,mapping,formula-injection",
        content="""File: docs/15-import-export-and-reporting.md.

Heavy op = job; upload/download separate; input immutable + hashed;
versions pinned; reproducible + audited.

Formats: import XLSX/CSV/TSV/NDJSON/JSON; export +DOCX/PDF; legacy
XLS via isolated converter. Untrusted: size/zip/formula/macro/
external-link/MIME + AV.

Import: upload+scan -> inspect -> mapping profile -> preview ->
dry run -> approval for mass change -> chunked per-row-idempotent
exec -> reconciliation -> error file -> audit + retention.

MappingProfile: sheet/range/headers, column->predicate/path, typed
parsing, normalizers, identity policy, relation creation,
provenance, empty/null semantics, defaults, conflict action,
formula handling, row filter, version/tests. Empty cell != delete —
explicit marker required.

ЮЛ Excel mapping: request fields -> check_request; ОПФ/name/brand
-> legal_entity statements; ИНН/ОГРН -> identity keys; ОКВЭД ->
classifier relation + interval; capital -> money + as_of;
addresses -> address entity + relation; license -> license entity;
contacts -> contact entities; manager -> person + management
relation; founder/share -> ownership relation entity; affiliations
-> distinct predicates + evidence.

Dedupe: row key = file hash + mapping version + sheet + row + op;
repeat job -> prior outcomes; fuzzy -> review queue.

Formulas: computed value kept; never executed; external links/
macros forbidden; recalculation via controlled service.

ExportDefinition: AST, columns/paths, translations, formats,
sorting, template, limits, classification. Runtime: reauthorize ->
snapshot -> chunks -> masks before writer -> escape =,+,-,@ ->
watermark -> optional encrypt/sign -> artifact TTL.

ReportDefinition: dataset query, sections/tables/charts/summaries,
locale/tz, template version, formatting, confidentiality, limits,
audiences/formats, snapshot, schedule. No SQL; declared dataset
only.

Mandatory reports: ФЛ/ЮЛ dossiers, holding composition, check
result+evidence, period stats, employee/decision/unit stats,
graphical, rejection, payment discipline/limits/deferments,
collateral + recheck monitor, usage + SLA.

Collateral statuses: >30d | overdue | <=30d | unassigned |
possibly-closed.

Security: per-chunk authz OR approved snapshot grant (TTL/
definition/fingerprint); schedule = own narrow grant; hidden
fields can't reach template.

Delivery: short-lived download authz, checksum, no-cache; e-mail =
link; attachments only encrypted/approved; ledger.

Retention: classification TTL/legal hold; recoverable removal.

Acceptance: round-trip IDs/Unicode/decimal/date; no formula
injection; no hidden field; 100k/1M flat memory; dup job -> no
dupes; localized errors; golden DOCX/XLSX/PDF."""),

    dict(
        kb_id="kb.docs.16", sort=16, category="spec",
        title="Архитектура Nuxt / Nuxt UI",
        summary="Nuxt 4 client architecture: SPA rendering mode, "
                "app/layers structure, the single generated API "
                "client wrapper, session/authorization context, "
                "metadata-driven form/list renderers, state rules, "
                "layout, search UX, performance, validation, "
                "accessibility, client security, error handling, and "
                "test coverage.",
        tags="nuxt,frontend,spa,metadata-driven,api-client",
        content="""File: docs/16-nuxt-architecture.md.

Stack: Nuxt 4, Vue 3 Composition API, TS strict, Nuxt UI 4, i18n,
Pinia/useState, generated typed client, Vitest+VTU+Playwright,
axe-core; lockfile-pinned.

Rendering: authenticated = SPA (ssr:false — no SSR/cache leak);
public pages may SSR. Nuxt server holds no business state, never
touches PG; Nitro proxy only manages cookies/headers/CSRF ->
Rocket.

Structure: app/{pages,layouts,components/{entity,graph,query,
workflow,admin},composables,stores,middleware,plugins,assets,
error.vue} + layers/{core,skr} + generated/api + i18n/locales +
tests.

API client (only wrapper): base URL, request-ID/locale/tz, CSRF,
timeout/abort, 401 refresh singleflight, typed errors, ETag/
If-Match, idempotency keys, cursor, no retry w/o key, redacted.
Components never call $fetch.

Session: GET /me/context -> profile, locale/tz, capabilities,
workspaces, policy revision, grants. Middleware = UX-only; server
re-decides. PolicyChanged -> invalidate/cancel/refetch/redirect.
No tokens in storage; no sensitive drafts in localStorage.

Metadata-driven UI: EntityFormRenderer (revision + ViewDefinition +
values + field permissions + locale + validation). Field registry:
text/textarea/rich(sanitized), boolean, integer/decimal/money/
measurement, date/time/datetime/tz, enum/dictionary, entity ref/
autocomplete, multi-value/repeating, address/contact/document,
file, derived, relation editor — unknown -> safe unsupported,
never raw JSON. EntityListRenderer: columns/filters/facets/sort/
actions/export from view; virtualization + cursor; prefs by ID.

State: key = tenant+resource+revision+view+locale+policy; draft
separate; optimistic only reversible; 412 -> conflict UI; jobs via
events+fallback; graph keyed + cursors + visited.

Layout: header (workspace/search/jobs/locale/profile), collapsible
nav by capabilities + schema menu, breadcrumbs, context panel,
command palette, environment banner. Brand tokens configurable.

Search: exact-ID + full-text + filters + recent/saved + keyboard +
filtered highlights + debounce; AST builder; batch/forbidden shown
w/ reason.

Performance: lazy routes, split admin/report/graph, cursor/virtual,
summary-first + lazy tabs, progressive graph, no N+1, CI budgets.

Validation: server authoritative; errors bound to predicate IDs;
draft survives 422/412; decimal string until submit.

A11y WCAG 2.2 AA: keyboard/focus, skip links, landmarks, label
associations, table headers, accessible graph tree, color not sole
carrier, reduced-motion, 200% zoom, RTL, SR announcements.

Client security: CSP no unsafe-eval, sanitize, no v-html,
rel=noopener, API downloads, sandboxed preview, no secrets in
runtimeConfig.public, SCA, clickjacking, auto-lock, step-up.

Errors: error.vue per class + request ID; no stack/payload."""),

    dict(
        kb_id="kb.docs.17", sort=17, category="spec",
        title="Спецификация пользовательских экранов",
        summary="User-screen specification: menu built from schema "
                "packages + capabilities, dashboard cards, the "
                "/objects/[typeAlias] catalog, /entities/[publicId] "
                "card with its tab set, create/edit flows with "
                "duplicate suggestion, related-info sections, graph/"
                "tree rules, global search, check request and run "
                "screens, risks/decision, reports, collateral "
                "monitoring statuses, persona visibility, jobs "
                "drawer, responsive rules, and UI acceptance.",
        tags="ui,screens,cards,graph,personas",
        content="""File: docs/17-ui-screen-specification.md.

Menu from packages + capabilities: Главная, Объекты, Заявки и
процессы, Проверки, Мониторинг, Риски и решения, Отчёты,
Документация, Администрирование, Интеграции, Аудит. Legacy items =
saved views/types.

Dashboard: my tasks/overdue, requests, checks, critical factors,
monitors due, reports/jobs, scoped stats, source health —
aggregate-leak-safe + drill-down.

Catalog /objects/[typeAlias]: saved views, simple/advanced search,
filters/facets, columns/sort, cursor table, bulk via server-side
token, capability-gated actions, status/freshness.

Card /entities/[publicId]: header (name + badges, lifecycle/risk/
classification, IDs by permission, freshness/conflict, actions,
revision/as_of). Tabs: Обзор | Связи | Граф/дерево | Проверки |
Риски | Процессы/задачи | Документы | История. Hidden tabs leave
no headers/counts.

Create/edit: revision form, required/conditional, repeating
values/relations, exact ref search, duplicate suggestion pre-
commit, raw/canonical preview for identity fields, draft, server
validation, unsaved guard, expected revision. Exact dup -> open
existing or explicit distinct; fuzzy -> candidates, never auto-
merge; hidden entity -> generic conflict.

Related: per-predicate groups; qualifiers/validity/source; add/
link; safe inline edit; counts; direction labels; effective/
historical.

Graph: toolbar (as_of, depth, predicates, direction, layout, hide
resolved, export); start depth 1; on-demand expand; Security
budget; cycle = link-back; edge label/period/source; high-degree
paged; hidden not inferable; accessible tree alternative.

Search: exact/text toggle, type chips, external = check job,
policy recents, safe highlights, hidden types absent.

Check request: new/existing, initiator, category, target/create,
product/limit/deferment, purpose/urgency, docs, due, profile;
timeline = assignment/sources/factors/decisions/SLA.

Check screen: summary, definition/version/status, source states
(error != "nothing found"), grouped results, conflicts/provenance,
criteria/factors, notes/evidence, run/retry, approve/reject,
report.

Risks/decision: factors; assessment (score/category/model/
freshness); decision form (outcome, reasons, conditions, limit/
deferment, validity, approvals); override = capability/reason/
expiry.

Reports: categories (ФЛ/ЮЛ checks, stats, graphical, rejections,
collateral); params scope/dates/employee/product/format/locale;
async + progress.

Collateral monitor: filters (client/ИНН, product, contract, type/
status, responsible, ranges, overdue, closed); statuses >30d/
overdue/<=30d/unassigned/possibly-closed + icon/text/token.

Personas: security service; manager (requests, limits, discipline,
ОДЗ/ПДЗ); risk officer (ЛТК, deferment, assessment, approvals).
Combined roles = policy intersection/union + deny-overrides.

Jobs drawer + deduped notifications. Responsive: narrow -> drawer
+ selected columns.

Acceptance: legacy scenarios reachable; publish -> live UI; hidden
data absent; 100k virtualized; cycle safe; a11y/RTL; status !=
color; dup -> existing ID; policy change clears cache."""),

    dict(
        kb_id="kb.docs.18", sort=18, category="spec",
        title="Административная панель",
        summary="Admin console specification: a managed editor of "
                "versioned resources with draft->validation->impact->"
                "approval->publish->audit on every change. Covers the "
                "schema catalog, form/view designer, migration "
                "center, users/identity, capability and data group "
                "editors, four-eyes publishing, connectors, criteria/"
                "workflow editors, reports, translations, jobs/"
                "projections, audit explorer, system settings, "
                "operator-error protections, and acceptance.",
        tags="admin,console,schema-editor,four-eyes,migration-center",
        content="""File: docs/18-admin-console.md.

Principle: managed versioned-resource editor; every change = draft
-> validation -> impact -> approval -> publish -> audit; high-risk
-> MFA/step-up.

Schema catalog: packages/types/predicates/revisions/deps/usage;
draft from published, add/inherit type, add predicate, cardinality/
domain/range/temporal, identity/normalization, index/search,
classification, translations, views, validate/impact/publish/
retire. Stable IDs immutable; confusable check.

Form/View designer: sections/tabs/columns, field components, safe-
DSL visibility, list filters/sorts, relation groups, capabilities,
responsive/persona/locale/RTL previews, a11y warnings; never
unhides policy-hidden field.

Migration center: source/target, steps, preview, affected rows,
invalid/collisions, estimates, dry run, approval, progress,
quarantine, verification, cutover/compensation. Blocked on unmapped
security rule or required issue.

Users: identity status, both group classes separately, intervals,
MFA/session/device, revoke, service accounts, review, impersonation/
delegation audit. Secrets never shown.

Capability groups: function/action matrix, default deny, acyclic,
effective preview w/ origin rules, no data selectors.

Data groups: target tenant/type/entity/predicate/relation/
classification; actions read/create/edit/retire/traverse/export/
history; conditions, fields/masks, validity/purpose, allow/deny,
owner/legal basis. Simulation: persona + example -> obligations +
impact + side-channel warnings; no raw SQL.

Four-eyes: author submits -> independent approver -> step-up ->
publish -> audit; no self-approval; emergency = break-glass.

Connectors: catalog/instance, ops, endpoint/proxy, secret presence/
rotate (write-only), license/purpose, quota, mapping version,
redacted health, test, enable/disable, retention, monitors.

Editors: criteria builder (typed expression, inputs, severity/
weight, explanation, evidence, tests, synthetic preview); workflow
(states/transitions, tasks, SLA/calendar, conditions, actions,
escalation, compensation) + linter (unreachable, no terminal,
unguarded loops, missing perms).

Reports: definitions/templates, params, dataset AST, formats,
classification, schedule, recipients, limits; synthetic/authorized
preview.

Translations: key x locale matrix, completeness/fallback, ICU
plural preview, overflow, RTL preview, bundle import/export,
draft/review/publish.

Jobs/projections: queues/lag/dead letters, retry/cancel,
generations/checksums; rebuild = capability/reason + new gen;
watermark not hand-edited.

Audit explorer: actor/action/resource/time/decision filters; safe
diff, revisions, actors, reason, integrity; export = separate
right.

Settings: locale/tz/retention, brand, limits, origins, OIDC refs,
calendars, feature flags, maintenance windows; DB URL/secrets not
editable.

Protection: env banner, destructive confirm w/ name/reason, impact/
dry run, optimistic concurrency, undo via new revision, estimate
before bulk, time-limited grants, stable-ID diffs, orphan-ref
prevention, observable publish locks.

Acceptance: schema built+published via UI; rename keeps ID/data;
collisions surfaced; person-deny simulation; both group classes;
secret unreadable; RTL preview; second approver; audit links
draft->approvals->publish."""),

    dict(
        kb_id="kb.docs.19", sort=19, category="spec",
        title="Международность, Unicode и локализация",
        summary="Internationalization: UTF-8 end-to-end, NFC "
                "canonical text with versioned normalizers, "
                "deterministic collation, LocalizedText translations "
                "with fallback chains, name/address/phone/email "
                "rules, UTC + IANA zone time handling, decimal/money "
                "protocol forms, ICU plurals, RTL, search profiles, "
                "localized reports/files, and the minimum test "
                "matrix.",
        tags="i18n,unicode,locale,rtl,collation,translations",
        content="""File: docs/19-internationalization.md.

Basics: UTF-8 end-to-end; PG UTF8; locale = BCP 47; tz = IANA;
API keys/aliases ASCII; labels = Unicode; raw value kept,
canonical/search separate; never change meaning via global lower/
trim/NFKC.

Normalization: default NFC; NFKC only per analyzed identity/
search field; normalizer version stored. Check bidi/zero-width,
confusables, grapheme-safe truncation, combining marks, emoji,
invalid UTF-8 before serde, filename normalization.

Collation: identity on canonical bytes (not OS collation); user
sort = versioned ICU + locale + public-ID tie-breaker; upgrade ->
REINDEX plan.

Translations: LocalizedText = resource kind/id + field + locale +
revision; fallback e.g. fr-CA -> fr -> default -> en; API = IDs +
localized display. Translated: labels/plurals/descriptions/help/
error keys/menu/templates. IDs never translated.

User data: lang-tagged variants; no auto MT without provenance;
legal name official; display/transliteration = separate
statements.

Names: no assumed first/middle/last — optional components, full
legal name, native script, transliteration, per-locale order,
mononym, patronymic, honorifics; legacy ФИО keeps components.

Address: country/areas/locality/postal/street/unit + formatted
lines + raw; composition country-dependent.

Phone: country-context parse, E.164, ext separate. E-mail: IDNA/
UTS-46 domain, local part per policy. URLs: IDN + canonical ASCII
host + Unicode display + anti-phishing.

Time: instant = UTC; zone stored; local date never forced UTC;
date-only stays date; schedule = rule + zone + DST policy; API
explicit on semantics.

Numbers/money: decimal-as-string w/ dot; locale parse + Intl;
money = amount + ISO currency; percents ratio/percent; units
versioned; Excel = typed cell value.

Messages: ICU params/plurals/select; no concatenation; message_key
+ typed params; HTML forbidden/sanitized.

RTL: dir by locale/content; logical CSS; semantic icon mirroring;
bidi isolation; user bidi controls visualized/sanitized.

Search: SearchProfile = analyzer + stemming + stop words +
transliteration; exact-ID separate; grapheme-safe filtered
highlight.

Reports/files: localized fonts (CJK/Arabic/Cyrillic), RTL DOCX/
PDF, XLSX typed values, CSV UTF-8, safe filenames + Unicode
titles, locale fallback.

Test matrix: ru-RU, en-US, fr-FR, ar (RTL), tr-TR, zh-CN, mixed/
emoji."""),

    dict(
        kb_id="kb.docs.20", sort=20, category="spec",
        title="Модель угроз и меры защиты",
        summary="Threat model: asset inventory, trust boundaries, "
                "STRIDE threats with countermeasures, specific "
                "attacks (SQLi, IDOR, mass assignment, XSS, CSRF, "
                "SSRF, file attacks, formula injection, graph "
                "inference, cache/log leaks, supply chain), "
                "encryption policy, data lifecycle, the security "
                "test gate list, and incident readiness.",
        tags="threat-model,stride,attacks,encryption,incident",
        content="""File: docs/20-threat-model.md.

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
decision IDs, reason, immutable events. Disclosure -> PEP rewrite
before SQL, RLS, allowlist, aggregate policy, tenant/policy cache
keys, redaction, classified search, encryption, short downloads.
DoS -> limits, cost guard, bounded queues, statement_timeout,
keyset, async jobs, scanning, quotas/breakers, backpressure.
Elevation -> default deny/deny-overrides, two classes, scoped
service identity, step-up/four-eyes, break-glass, simulation,
negative tests.

Specific: SQLi (no interpolation; catalog identifiers); IDOR (UUID
grants nothing; hidden -> 404); mass assignment (deny unknown
fields; system fields never from body); XSS (escaping, sanitizer,
CSP, sandbox); CSRF/CORS (token + SameSite + Origin allowlist);
SSRF (endpoint allowlist, DNS/IP validation, egress proxy,
redirect revalidation, blocked private ranges); files (signature
over extension, zip limits, macro detection, AV, isolated
converter); formula injection (escaping, allowlist, no execution);
graph inference (denied excluded each step; counts/facets/paths
authorized); cache leak (tenant/shape/locale/revision keys); log
leak (allowlist; no secrets/PII); supply chain (lockfile, audits,
signatures, SBOM, reproducible, secret scanning).

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
assess -> notify -> patch -> rotate -> verify -> postmortem."""),

    dict(
        kb_id="kb.docs.21", sort=21, category="spec",
        title="Наблюдаемость и эксплуатация",
        summary="Observability and operations: structured JSON logs "
                "with a strict allowlist, RED/USE metrics with "
                "bounded label cardinality, W3C traces, audit-vs-"
                "logs separation, health endpoints, SLO/error "
                "budget, the actionable alert list, dashboards, "
                "backup and disaster-recovery runbook with RPO/RTO "
                "goals, graceful shutdown, required runbooks, "
                "capacity forecasting, and JIT operational access.",
        tags="observability,logs,metrics,alerts,backup,dr,runbooks",
        content="""File: docs/21-observability-and-operations.md.

Logs (structured JSON): timestamp/level/service/version/env,
request/correlation/trace, hashed tenant, route/action/outcome/
status, duration/error ID, plan fingerprint, job/event/connector
IDs. Forbidden: tokens/secrets, raw req/res, SQL values, PII,
connector payload; debug via redaction allowlist.

Metrics: RED (API) + USE (DB/worker) — DbExecutor queue/pool/
saturation, SQL fingerprints, outbox lag/dead-letters, job queue/
leases/heartbeat, projection watermark/lag, connector latency/
quota/circuit, PDP latency/cache/deny, auth anomalies, report
rows/bytes, PG connections/WAL/replica/vacuum/bloat, storage
errors. Bounded label cardinality.

Traces: W3C context through Rocket -> PEP -> DbExecutor -> worker/
connector; no payloads; jobs via causation links.

Audit != logs: audit = evidence w/ retention/integrity + separate
access; logs = diagnostics w/ short TTL.

Health: /health/live (no external calls); /health/ready (config,
DB, PDP/policy, storage/queue, migrations); /health/startup;
detailed internal; public generic. Connector failure = degraded;
authz failure -> unready/fail closed.

SLO/budget: separate SLI per surface; maintenance per policy;
budget steers rollout.

Alerts (owner/severity/runbook/test): RLS canary (critical), auth
spike/reuse, PDP down/indeterminate, DB saturation/replica/WAL,
outbox/projection lag, stuck leases, connector auth/quota, backup
failure/age, audit chain failure, cert expiry, capacity.

Dashboards: executive SLO, API/DB perf, security/PDP, jobs/
projections, connectors, reports, PG maintenance, tenant fairness.

Backup: WAL + base backups, storage versioning/replication,
bundles/config refs, KMS backup, encrypted + geo-compliant,
automated restore validation, documented RPO/RTO — exists only
after a restore drill.

DR: declare/freeze -> restore -> verify migrations/audit ->
reconcile outbox/inbox/jobs -> rebuild projections -> canaries ->
reopen read then write -> document gap. Initial RPO <=15min,
RTO <=4h.

Shutdown: drop readiness, reject long work, drain HTTP, cancel/
wait DB jobs, flush telemetry; worker releases leases, saves
cursor; outbox retryable.

Runbooks: DB down/saturated, plan regression, RLS/policy issue,
leaked credential, connector outage/drift, projection rebuild,
stuck/dead-letter job, failed migration, disk/WAL growth, backup
restore, rollback, data leak, key rotation.

Capacity: monthly forecast + planned-scaling thresholds.

Ops access: JIT/time-limited, MFA, bastion, least privilege,
approval/audit; DBA != app user; raw query = ticket + masked
replica."""),

    dict(
        kb_id="kb.docs.22", sort=22, category="spec",
        title="Тестирование и качество",
        summary="Testing and quality: the 10-level pyramid, domain "
                "property tests, real-PostgreSQL persistence tests, "
                "the security persona matrix, API contract tests, "
                "query tests, connector fixtures, import/report "
                "tests, Nuxt tests, performance scenario manifests "
                "and soak, chaos/failure invariants, static/supply "
                "checks, synthetic-data rules, and DoD coverage.",
        tags="testing,quality,property-tests,security-matrix,chaos",
        content="""File: docs/22-testing-and-quality.md.

Pyramid: domain unit/property -> crate contract -> PG repository/
integration -> security negative/info-flow -> API/OpenAPI ->
connector fixtures -> Nuxt component/E2E/a11y -> migration/
upgrade/rollback -> perf/soak/failure -> pentest/DR.

Domain properties: normalization idempotent; canonical
deterministic; fingerprint; symmetric edge order; cardinality
under concurrency; merge/split acyclic; migration round-trip/
residual; decimal/time deterministic; same inputs -> same risk;
workflow invariants.

Persistence (real PG only): fresh + per-release upgrade;
constraints; identity/fingerprint races; optimistic locking;
rollback/outbox atomicity; SKIP LOCKED; RLS context; pooled
reset; partition pruning; plan snapshots; tz/Unicode/decimal;
serialization retry.

Security matrix (personas x actions x resources x fields x
relations x purposes): default deny; deny-overrides; two classes;
object/type/field/edge; GET/list/search/filter/sort/facet/count/
graph/history/blob; report/export/import error file; WS/event;
cache; worker/service account; impersonation/break-glass; policy
change mid-session. Negative tests check rows + DOM/network/
artifact/log/cache.

API contracts: OpenAPI snapshot/compat; strict DTO; statuses/
codes; ETag/If-Match; idempotency commit-before-response; cursor
tamper; limits/malformed; CSRF/CORS; cancellation; WS
reauthorization; consumer-driven tests.

Query: AST fuzz; operator typing; bind/no literals; security
filter in every subquery/step; cost guard; keyset completeness;
null/collation; cycles; aggregate inference; fingerprint
equivalence; injection strings.

Connectors: synthetic fixtures, sandbox smoke, drift, pagination,
rate/quota, retry/timeout, ambiguous, malformed, duplicates,
mapping version, legal flags, redaction, secret rotation.

Import/report: both supplied XLSX; multi-row founders; date/
decimal/Unicode; dedupe/retry; formula/macro/external-link; zip/
MIME; error coordinates; 100k/1M streaming memory; DOCX/XLSX/PDF
golden; hidden fields absent; locale/RTL/fonts.

Nuxt: renderer per kind; permission removes DOM/actions; personas;
412 conflict/draft; policy event clears state; virtual list;
graph cycle/hidden node; jobs/reconnect; keyboard/focus/zoom/RTL/
axe; CSP/XSS; visual regression.

Performance: scenario manifest (SHA, schema, seed/size, config/
hardware, concurrency, warmup/duration, SLO); profiles incl.
lookup, card, list, FTS, graph, writes, import/export, policy
cold/cache, catch-up. Soak 8-24h. Gates per kb.docs.11.

Chaos: kill per job phase; DB restart/failover; replica lag;
storage down; PDP/cache down; connector timeout; outbox dup/
reorder; clock skew; disk pressure; lost post-commit response.
Invariants: no duplicate, no unauthorized fallback, resumable,
explicit status.

Static/supply: fmt/clippy; test/nextest; deny/audit; vue-tsc/
eslint; npm audit/SCA; secret scan; SBOM; container scan/sign;
migration lint; OpenAPI breaking check.

Test data: synthetic; no prod dump w/o anonymization; versioned
classified fixtures; no real IDs.

DoD: coverage % != risk; critical code needs branch/property +
mutation where feasible; done = link + tests + telemetry +
migration/rollback + docs + security/perf review."""),

    dict(
        kb_id="kb.docs.23", sort=23, category="spec",
        title="Развёртывание и конфигурация",
        summary="Deployment and configuration: supported topologies "
                "(on-prem single site, HA multi-zone, dedicated "
                "tenant shard), container image rules, environment "
                "list, typed TOML config with secret refs, secret "
                "manager coverage, TLS/network policy, the CI/CD "
                "stage order, expand/contract DB deploy, rollout "
                "order, Nuxt asset delivery, readiness order, "
                "scaling rules, and the production checklist.",
        tags="deployment,configuration,containers,cicd,secrets",
        content="""File: docs/23-deployment-and-configuration.md.

Topologies: on-prem single site (LB -> api x N -> PG primary/
replica -> storage -> workers); HA multi-zone; dedicated tenant
shard (same release).

Containers: minimal non-root, pinned digest, read-only FS,
bounded tmp, no shell/pkg mgr, seccomp/cap drop, health endpoints,
SBOM/signature, UTC + needed locale/fonts, controlled CA trust.

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
storage, worker->DB/storage/egress; egress allowlist; cert
monitor.

CI/CD: lint/unit -> integration PG/security -> API/UI contract ->
SCA/secret/SBOM/container -> migration fresh/upgrade -> E2E/a11y
-> perf smoke -> signed artifacts -> staging -> canary -> approval
-> prod rolling/canary -> post-deploy checks.

DB deploy: expand first; sqlx migrate as separate job + advisory
lock + owner role; resumable backfill; contract after drain;
CONCURRENTLY/partition per runbook.

Rollout: compatible app first; flags off; canary; monitor;
gradual; rollback binary while compatible; irreversible ->
compensation.

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
legal/quotas; locale/fonts; load/pentest; rollback/compensation."""),

    dict(
        kb_id="kb.docs.24", sort=24, category="spec",
        title="Миграция существующего прототипа",
        summary="Migration off the legacy Rocket 0.5-rc prototype: "
                "the catalogued limitations, the strangler "
                "transition rule, stages 0-7 (repo security, "
                "workspace skeleton, security baseline, meta-kernel, "
                "data extraction, SKR package, Nuxt, dual-run), the "
                "type-mapping table, reconciliation requirements, "
                "API compatibility, the reuse/never-reuse lists, and "
                "migration acceptance.",
        tags="migration,prototype,strangler,legacy",
        content="""File: docs/24-current-code-migration.md.

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

Stage 0 repo security: rotate DB password; remove tracked secrets
-> ref/env; ban prototype for prod data; scanning; read-only
snapshot. History keeps secrets — rotation mandatory.

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
merges/candidates, statements by predicate, orphan links, tz,
null/empty, dup keys, checksum buckets, sampled compare; cutover
blocked on delta.

API compat: separate adapter (old paths -> new commands + security
decision + deprecation deadline); RawHtml/generic ERROR never
carried.

Reusable: domain knowledge/scenarios, Rocket organization, type/
field/link/dictionary concepts, auth reference, fixtures + SQL
model, Cargo.lock. Never reused: SQL string builders, secret
config, allow-by-absence, dynamic DDL.

Acceptance: rows accounted; old ID resolves; links/provenance/
history kept; no dup identity; permissions not widened; secrets
rotated; scenario equivalence; SLOs; rollback rehearsed;
decommission approved."""),

    dict(
        kb_id="kb.docs.25", sort=25, category="spec",
        title="План реализации и критерии приёмки",
        summary="Delivery plan and acceptance: the seven vertical "
                "phases with exit criteria, connector priority "
                "order, Definition of Done, the eleven end-to-end "
                "acceptance scenarios (A-K), performance/security/"
                "documentation acceptance, and the explicit "
                "non-criteria for readiness.",
        tags="delivery,phases,acceptance,dod,scenarios",
        content="""File: docs/25-delivery-plan-and-acceptance.md.

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
tax/company -> enforcement/court/procurement/registration/health
-> Internet/social last.

DoD: requirement link; designs; security classification/policy;
migrations/rollback; tests incl. negative; telemetry; i18n/a11y;
perf/limits; runbook; docs; clean deps; review.

E2E scenarios:
A universal "Склад" package published live, CRUD/search/report.
B 100 concurrent same-phone creates -> one entity; retries one
  ID; distinct extension per policy.
C both ЮЛ workbooks preview/dry-run; graph mapping; repeat import
  no dupes; relations w/ provenance.
D type split/reify/rename round-trip + residual + quarantine +
  concurrent writes in target projection.
E holdings-allowed/person-denied sees zero person data incl.
  counts/graph/report/WS/cache; DB instrumentation confirms.
F field security — one sees passport/phone, other nothing incl.
  sort/facet/export; mass-assign denied.
G check: source timeout != clean; provenance; idempotent factor/
  assessment/task/decision/report/monitor.
H monitor: DST run once; repeat result no dup; real change ->
  evidence/task.
I 100M/500M SLOs; no OFFSET/seq scan.
J i18n names/IDN/decimal/case/DST/RTL/emoji round-trip.
K audit/DR: tamper-evident links; RPO/RTO restore; rebuild;
  canaries.

Perf acceptance: P95 targets; zero unauthorized rows; bounded
graph/export; lag target; no queue collapse; soak stable.

Security acceptance: threat model reviewed; pentest; critical/high
fixed; RLS/PEP matrix; MFA/four-eyes/break-glass; rotation; scans;
drills; connector legal basis.

Docs acceptance: links exist; terms consistent; examples match
contracts; traced; ADR updated.

NOT readiness: small test only, UI-hidden button, SELECT-before-
INSERT, generic JSONB, post-filter permissions, manual SQL,
untested backup, field-policy-free report, plaintext secret,
missing perf dataset."""),

    dict(
        kb_id="kb.docs.26", sort=26, category="spec",
        title="Архитектурные решения и компромиссы",
        summary="The ADR register ADR-001 through ADR-023: modular "
                "monolith, statement graph + typed families, stable "
                "IDs, immutable canonical facts, residual facts, "
                "PostgreSQL source of truth, sqlx async-std bridge, "
                "logical security API, permissions-before-query, "
                "two group classes, default deny, idempotency + "
                "semantic keys, keyset, jobs/outbox, Nuxt SPA, "
                "metadata-driven UI, fuzzy-never-merge, holding-as-"
                "graph-view, international defaults, no physical "
                "delete, service-account isolation, report dataset "
                "security, and PostgreSQL 17+ baseline.",
        tags="adr,decisions,tradeoffs",
        content="""File: docs/26-decisions-and-tradeoffs.md.

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
+ impact + approval + dependent updates; old ADR superseded."""),

    dict(
        kb_id="kb.docs.27", sort=27, category="spec",
        title="Глоссарий",
        summary="The project glossary: canonical definitions for "
                "EngineR/СКР САНАР terms — entity, statement, "
                "predicate, meta-kernel, schema package/revision, "
                "residual fact, lineage, provenance, effective view, "
                "identity machinery, merge/split, projections, "
                "watermark, query AST, PEP/PDP/obligations, the two "
                "group classes, RLS, break-glass, outbox/inbox, "
                "jobs, connectors/observations/checks, risk terms, "
                "view definitions, IDs, cursors, and the Russian "
                "domain abbreviations (ЛТК, ОДЗ/ПДЗ, ЮЛ/ФЛ, холдинг).",
        tags="glossary,terms,definitions",
        content="""File: docs/27-glossary.md.

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
  type/entity/field/relation/row. Capability/data groups: only that
  class. RLS: PG row security barrier. Purpose of use: declared
  processing purpose. Break-glass: emergency access + step-up/
  audit.
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
- ЛТК: trade-credit limit. ОДЗ/ПДЗ: receivables indicators —
  semantics set by the package. ЮЛ/ФЛ: legal/natural person —
  package types, not core structs. Холдинг: temporal ownership/
  control/affiliation graph view."""),

    dict(
        kb_id="kb.docs.99", sort=99, category="spec",
        title="Ссылки и нормативные источники",
        summary="Normative external references: internal source "
                "documents, the Rocket/sqlx/Rust guides, PostgreSQL "
                "documentation anchors, Nuxt docs, and the standards "
                "(Unicode UAX-15/UTS-39, BCP 47, RFC 3339/9457, "
                "OWASP, WCAG 2.2, W3C Trace Context) plus the "
                "version currency rule.",
        tags="references,links,standards",
        content="""File: docs/99-references.md.

Internal materials (unchanged sources): Тезисы.docx, ТЗ для СКР.docx,
Экраны СКР.docx, Для СКР по ЮЛ.XLSX + variant.

Rust/Rocket/sqlx: Rocket v0.5 guide + async migration/runtime notes +
API docs; docs.rs/sqlx; Rust API Guidelines.

PostgreSQL anchors: current docs, ddl-rowsecurity, ddl-partitioning,
indexes, textsearch, collation, routine-vacuuming,
pg_stat_statements.

Nuxt: Nuxt 4 docs, Nuxt UI docs, Vue accessibility guide.

Standards: UAX #15 (normalization), UTS #39 (Unicode security),
BCP 47 (RFC 5646), RFC 3339 timestamps, RFC 9457 problem details
(reference pattern only — the project's error contract is its own),
OWASP ASVS + API Security, WCAG 2.2, W3C Trace Context.

Currency rule: versions/links re-checked on each dependency or
PostgreSQL/Nuxt upgrade; the normative set for a build is lockfile +
rust-toolchain + container digest + ADR — a "current" doc link never
authorizes an automatic production bump."""),

    dict(
        kb_id="kb.docs.readme", sort=100, category="spec",
        title="EngineR / СКР САНАР — комплект спецификаций",
        summary="The docs/ reading guide: recommended order for the "
                "normative spec set, the per-document map, the "
                "normativity keywords (ДОЛЖЕН/НЕЛЬЗЯ/СЛЕДУЕТ/МОЖЕТ), "
                "the fundamental invariants, and the source-material "
                "list.",
        tags="docs,index,readme,normativity",
        content="""File: docs/README.md.

Reading order: 00 vision + 01 sources -> 02 architecture -> 03
metamodel + 04 invariants + 10 PG schema + 11 performance -> 05
workspace + 06 API contract + 07 query -> 08/09 security + 20 threat
model -> 16/17/18 Nuxt/UI/admin -> 22 testing + 25 acceptance.

Document map: 00 vision/scope; 01 sources+traceability; 02 system
architecture; 03 universal domain model; 04 topology/unitarity/
idempotency; 05 Rust workspace; 06 API contract; 07 query engine;
08 security architecture; 09 security API; 10 PostgreSQL schema;
11 performance/scaling; 12 schema evolution; 13 events/jobs/
automation; 14 connectors/risk engine; 15 import/export/reporting;
16 Nuxt architecture; 17 UI screens; 18 admin console; 19 i18n;
20 threat model; 21 observability/operations; 22 testing/quality;
23 deployment/configuration; 24 prototype migration; 25 delivery
plan/acceptance; 26 ADRs; 27 glossary; 99 references.

Normativity: ДОЛЖЕН/НЕЛЬЗЯ/СЛЕДУЕТ/МОЖЕТ = requirement/prohibition/
recommendation/option. Source of truth = canonical entities +
statements — indexes/snapshots/marts/reports are rebuildable
projections. PEP bypass = release blocker. ФЛ/ЮЛ/холдинг/договор/
warehouse unit are never special tables or code branches in core.
Physical deletion of business data forbidden via ordinary API —
use validity completion, revocation, tombstone + regulated cleanup."""),
]

# kb_related edges to add: onboarding->kb.self, kb.self->schema.lifecycle,
# kb.docs.index->each new spec article.
EXTRA_RELATIONS = [
    ("kb.onboarding", "kb.self"),
    ("kb.self", "kb.schema.lifecycle"),
] + [("kb.docs.index", a["kb_id"]) for a in ARTICLES
     if a["kb_id"].startswith("kb.docs.")]


def resolve_ids():
    """kb_id -> public_id for every kb_article."""
    l = call("entities.list", {"type": "kb_article", "limit": 200})
    rows = l.get("data") or []
    out = {}
    for r in rows:
        f = r.get("fields") or {}
        if f.get("kb_id"):
            out[f["kb_id"]] = r.get("public_id") or r.get("id")
    return out


def update_index_content(docs_ids):
    """Append the spec-article id list to kb.docs.index content."""
    pid = docs_ids.get("kb.docs.index")
    if not pid:
        print("kb.docs.index missing — skip index update"); return
    g = call("entities.get", {"id": pid})
    fields = ((g.get("data") or {}).get("fields")) or {}
    cur = fields.get("kb_content") or ""
    marker = "Spec articles ingested as kb.docs.*"
    if marker in cur:
        print("index content already extended — skip"); return
    lines = "\n\n" + marker + " (entities.query kb_category eq \"spec\"):\n"
    for a in ARTICLES:
        if a["kb_id"].startswith("kb.docs."):
            lines += f"  {a['kb_id']} — {a['title']}\n"
    r = call("entities.update", {"id": pid,
                                 "fields": {"kb_content": cur + lines}})
    print("index update:", "FAILED " + show(r, 300) if failed(r) else "ok")


def main():
    rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "kb-docs", "version": "1"}})
    rpc("notifications/initialized")

    ids = resolve_ids()
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
        # Long scalars exceed the statement_active_fact_unique btree
        # tuple limit (~2.5KB of real prose). On that failure drop the
        # trailing paragraph and retry; the file path in line 1 always
        # stays, so the full text remains reachable.
        attempts = 0
        while True:
            r = call("entities.create", {"type_id": "kb_article",
                                         "fields": fields})
            if not failed(r):
                break
            txt = show(r, 400)
            if ("index row size" in txt or "internal_error" in txt) \
                    and attempts < 12:
                body = fields["kb_content"]
                cut = body.rfind("\n\n")
                if cut <= 0:
                    break
                fields["kb_content"] = (body[:cut].rstrip()
                                        + "\n[truncated: btree cap; "
                                        + "see source file]")
                attempts += 1
                continue
            break
        if failed(r):
            print(f"CREATE FAILED {a['kb_id']}:", show(r)); continue
        data = r.get("data") or r
        pid = data.get("public_id") or data.get("id")
        ids[a["kb_id"]] = pid
        note = f" (-{attempts} para)" if attempts else ""
        print(f"{a['kb_id']:>22} -> {pid} "
              f"({data.get('outcome', 'created')}){note}")

    ok = 0
    for subj, obj in EXTRA_RELATIONS:
        sid, oid = ids.get(subj), ids.get(obj)
        if not sid or not oid:
            print("skip relation", subj, "->", obj); continue
        r = call("relations.create", {
            "id": sid, "predicate": "kb_related",
            "target_entity_id": oid})
        if failed(r):
            txt = show(r, 200)
            if "409" in txt:
                ok += 1  # duplicate relation — idempotent
            else:
                print(f"REL FAILED {subj}->{obj}:", txt)
        else:
            ok += 1
    print(f"relations ok: {ok}/{len(EXTRA_RELATIONS)}")

    update_index_content(ids)


if __name__ == "__main__":
    main()
