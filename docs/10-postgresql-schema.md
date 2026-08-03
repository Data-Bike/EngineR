# PostgreSQL: нормативная схема

## 1. Принципы

- PostgreSQL — источник истины.
- Все обращения приложения выполняются через sqlx.
- DDL меняется только миграциями.
- Dynamic business schema не создаёт произвольный DDL в request path.
- Канон нормализован; JSONB используется для ограниченных metadata/raw payload, не как единственное хранилище.
- Внешние UUID генерируются Rust UUIDv7; внутренние bigint обеспечивают компактные индексы.
- Каждая tenant table содержит tenant_id и проверяется RLS.
- Время — timestamptz UTC; domain date — date.
- Финансы — numeric(precision, scale), не float.

## 2. PostgreSQL schemas

| Schema | Назначение |
|---|---|
| core | tenant, entity, statements, identity |
| meta | type/predicate/schema revisions, layouts/translations |
| sec | principals, groups, policies, decisions |
| ops | outbox, inbox, jobs, projections, idempotency |
| audit | append-only audit |
| integration | connectors, source records, provenance |
| report | definitions, runs, artifacts |

Application DB role получает только нужные grants; migration owner отделён.

## 3. Tenant и последовательности

### core.tenant

- id uuid PK;
- alias text unique (ASCII lower);
- default_locale text;
- default_time_zone text;
- data_region text;
- status text;
- created_at/retired_at.

Возможен global catalog tenant для подписанных schema bundles, но data entity всегда принадлежит конкретному tenant.

### ID generation

Отдельные bigint sequences для entity/statement или identity column. Public UUID unique. Shard routing не кодируется в public ID.

## 4. Meta registry

### meta.schema_revision

- tenant_id, revision_id bigint/public_id uuid;
- state: draft, validating, ready, published, retired, failed;
- parent_revision_id;
- semantic_version;
- content_hash;
- created/published actor/time;
- migration_plan_id;
- compatibility result.

Только одна current published revision на tenant/package через partial unique index.

### meta.type_definition

- tenant_id, type_id uuid;
- stable_alias text;
- entity_class;
- created_revision, retired_revision;
- system flags.

### meta.type_revision

- tenant_id, type_id, schema_revision_id;
- lifecycle/identity/presentation profile refs;
- validation metadata;
- definition_hash.

### meta.predicate_definition / predicate_revision

Стабильный predicate ID + versioned domain/range/cardinality, direction, inverse, temporal, normalization, validation, identity, index/search/classification profiles.

### meta.type_predicate

- type_id, predicate_id, schema_revision_id;
- required;
- min/max cardinality;
- order/group;
- default expression ref;
- inheritance source.

### meta.localized_text

- tenant_id;
- resource_kind/resource_id;
- field: label, plural_label, description, help;
- locale BCP 47;
- text;
- schema_revision;
- unique resource+field+locale+revision.

## 5. Entity

### core.entity

- tenant_id uuid NOT NULL;
- entity_id bigint NOT NULL;
- public_id uuid NOT NULL;
- entity_class smallint;
- created_at timestamptz;
- created_by uuid;
- retired_at/by/reason;
- head_revision bigint;
- canonical_entity_id bigint nullable;
- classification_id uuid nullable;
- business_unit_id uuid nullable;
- PRIMARY KEY (tenant_id, entity_id);
- UNIQUE (tenant_id, public_id);
- CHECK canonical_entity_id != entity_id.

Canonical chains periodically path-compress; trigger/application prevents cycles. FK всегда включает tenant_id.

### core.entity_alias

Хранит loser public IDs/external aliases после merge: tenant, alias_kind, alias_value/hash, canonical entity, valid interval, reason. Unique active alias.

### core.entity_revision

Append-only metadata каждой команды: entity, revision, schema revision, actor, command/event, request, timestamp, previous revision, content hash. Unique entity+revision.

## 6. Утверждения: логически единый, физически типизированный набор

Чтобы не хранить широкую sparse row и не заставлять один индекс обслуживать всё, используются families с одинаковым заголовком:

- core.statement_edge;
- core.statement_text;
- core.statement_number;
- core.statement_boolean;
- core.statement_date;
- core.statement_instant;
- core.statement_geo;
- core.statement_blob;
- core.statement_json (только разрешённые structured predicates).

Общие колонки:

- tenant_id;
- statement_id bigint;
- public_id uuid;
- subject_entity_id;
- predicate_id uuid;
- schema_revision_id;
- entity_revision;
- valid_from/valid_to;
- recorded_at;
- status: active, superseded, revoked, quarantined;
- provenance_set_id;
- confidence numeric;
- canonical_hash bytea;
- qualifier_hash bytea;
- created_by/request_id;
- supersedes_statement_id/revoked_by_statement_id.

Payload:

- edge: object_entity_id и canonical endpoint fields;
- text: value_raw, value_nfc, value_search, language_tag;
- number: value_numeric, unit_id/currency;
- boolean: value_boolean;
- date: value_date;
- instant: value_instant, source_zone;
- geo: PostGIS value при включённом profile;
- blob: blob_id;
- json: value_json + schema hash и строгий size limit.

CHECK/FK гарантируют payload. Активный uniqueness fingerprint включает tenant, subject, predicate, canonical payload, validity/qualifier и dedupe scope.

### Почему не одна таблица

Логическая унитарность реализуется domain enum/repository. Физическое разделение уменьшает row width/index bloat, даёт typed operators и независимую partition/index policy.

## 7. Type membership и current state

Типизация канонически является statement, но hot projection:

### core.entity_type_current

- tenant_id, entity_id, type_id;
- statement_id;
- schema_revision;
- valid_from/to;
- PRIMARY KEY tenant+type+entity;
- indexes tenant+entity+type и tenant+type+entity.

### core.statement_current

Не создаётся универсальной копией всех payload. Current partial indexes status=active используются в family tables; для частых joins возможен компактный header projection.

## 8. Identity и dedupe

### core.identity_policy

Versioned policy metadata/normalizer/dedupe action.

### core.identity_key

- tenant_id;
- type_id;
- policy_id/version;
- key_hash bytea;
- canonical_key bytea или encrypted canonical;
- owner_entity_id;
- state active/retired/collision/review;
- quality;
- created_at;
- unique active tenant+type+policy+key_hash;
- полное canonical equality проверяется при hash match.

Чувствительные key values шифруются/токенизируются; hash keyed per tenant, чтобы не использовать таблицу как rainbow oracle.

### core.match_candidate

entity pair, algorithm/version, features summary, score, status, reviewer/reason. Не является merge.

## 9. Provenance

### integration.source_system

connector instance, legal basis, license ref, jurisdiction, retention, trust rank, status.

### integration.source_record

tenant/source/namespace/external_id, fetched/effective time, payload blob/hash, parser version, status. Unique source+external ID+observation scope.

### integration.provenance_set / provenance_member

Нормализует many-to-many statement ↔ source record/evidence. Provenance добавляется идемпотентно.

## 10. Проекции

### ops.entity_summary

tenant, entity, type, locale, title/subtitle, status, selected safe fields, source entity revision, projection revision. Read model, восстанавливаемый.

### ops.entity_search

entity/type/locale, exact tokens, tsvector, optional trigram columns, updated revision. Не хранит скрытые поля в общем search document; чувствительные search shards/columns разделены по classification.

### ops.typed_value_index

Если family partial index недостаточен, projection catalog создаёт логические typed index rows: tenant, predicate, type, canonical typed value, entity. Используется только approved index profiles.

### ops.projection_catalog/checkpoint

Определение, state, source/schema revision, watermark, build generation, checksum, active generation. Rebuild пишет новую generation и атомарно переключает.

### Hot type materialization

Для доказанно горячих views schema compiler может создать управляемую read-model table с typed columns. Она не source of truth, имеет generation/checkpoint и полностью перестраивается.

## 11. Security tables

- sec.principal, user_identity, session, credential metadata;
- sec.capability_group, data_group;
- sec.principal_capability_group, principal_data_group;
- sec.policy, policy_revision, policy_rule;
- sec.subject_scope_membership;
- sec.decision_log/outcome;
- sec.break_glass_grant.

Membership имеет valid interval/revision. Secret ciphertext хранится отдельно с key ID.

## 12. Operations

### ops.idempotency_record

tenant, principal, route, key_hash, request_hash, state, status_code, response body/blob, result ref, created/expires. Unique tenant+principal+route+key.

### ops.outbox_event

event_id, aggregate, revision, type/version, payload jsonb, trace, available, attempts, published. Payload versioned и size-limited.

### ops.inbox_delivery

consumer, event_id, handler_version, state/result. Unique triplet.

### ops.job

type, tenant, initiator/effective service, state, progress, parameters hash/ref, policy/schema snapshots, lease owner/expiry, attempts, next_run, result/error artifact.

### ops.job_item

Для row-level imports/bulk: ordinal, source ref, idempotency key, state, entity/result/error.

## 13. Audit

audit.event — append-only и time-partitioned:

- event_id/time/tenant;
- actor/effective actor/session/service;
- action/resource;
- decision/request/correlation;
- before/after hashes или approved diff;
- row/byte counts;
- reason/legal basis;
- previous hash/event hash/checkpoint.

UPDATE/DELETE для app roles запрещены. Long-term архив имеет integrity manifest.

## 14. Files/reports

- core.blob: content hash, size, MIME, storage key, scan status, encryption key, retention;
- core.blob_reference: owner/resource/predicate/permission context;
- report.definition/revision;
- report.run: dataset/query fingerprints, schema/policy snapshots, state;
- report.artifact: blob, format, checksum, expires, download count.

## 15. Партиционирование

- statement families: HASH по tenant bucket; крупные deployments выбирают 32/64 partitions. Unique constraints включают partition key.
- audit/outbox/source observations: RANGE по recorded/created month с retention/archive.
- jobs: state/time indexes; partitioning только после измерения.
- Нельзя создавать тысячи partitions на tenant автоматически.
- Смена partition scheme выполняется online copy/dual write/checksum/cutover.

## 16. Индексы

Минимальный каталог:

- entity tenant+public_id; tenant+canonical ID; active classification/business unit;
- entity_type tenant+type+entity;
- each statement tenant+subject+predicate active;
- edge tenant+object+predicate active;
- exact typed tenant+predicate+canonical value active;
- temporal tenant+predicate+valid range при необходимости GiST;
- identity active unique;
- search GIN tsvector и selective trigram;
- outbox available/state/time;
- job state/next_run/lease;
- audit tenant/time/action.

Каждый новый index имеет owner query, ожидаемую selectivity, write/storage cost и usage review. Индекс на весь JSONB «на всякий случай» запрещён.

## 17. RLS context

В начале transaction persistence выполняет безопасный set_config(..., true) для tenant/principal/purpose/policy revision. RLS policy читает только transaction-local setting и отклоняет отсутствие. После transaction выполняется rollback/reset; тест подтверждает отсутствие context leakage между pooled connections.

## 18. Миграции

sqlx migrate запускается отдельным migration job/role до readiness новой версии. Миграции expand/contract, resumable backfill и idempotent verification. Приложение не выполняет CREATE TABLE из user request.
