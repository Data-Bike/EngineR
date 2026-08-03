# Архитектурные решения и компромиссы

## ADR-001: Модульный монолит

**Статус:** принято.

**Решение:** API/worker deployables и строгие crate boundaries, без микросервиса на каждый контекст.

**Причина:** транзакции/security/query hot path, простота on-prem и эксплуатации. Границы позволяют выделение при измеренной необходимости.

## ADR-002: Statement graph + typed physical families

**Статус:** принято.

**Решение:** единый logical statement, физические edge/text/number/date/... tables и projections.

**Причина:** доказуемая унитарность/эволюция при typed indexes и отсутствии sparse-wide table.

**Отклонено:** table per business type (hardcode/DDL churn); one JSONB (weak integrity/performance); pure RDF store (не соответствует PostgreSQL/sqlx constraint и predictable OLTP).

## ADR-003: Stable IDs, labels mutable

**Статус:** принято.

Переименование не мигрирует facts/policies/views. API использует UUID IDs, alias только ergonomic unique name.

## ADR-004: Canonical facts immutable semantically

**Статус:** принято.

Изменение создаёт revision/supersede/revoke. Проекции mutable/rebuildable.

## ADR-005: Residual facts для lossless transformation

**Статус:** принято.

Невыразимое target schema сохраняется с lineage. Это честная формализация «любой объект в любой без потерь».

## ADR-006: PostgreSQL source of truth, no dynamic DDL in request

**Статус:** принято.

Business schema живёт в meta tables. Index/projection DDL выполняет approved async schema compiler/migration.

## ADR-007: SQLx async-std bridge

**Статус:** принято как требование совместимости.

Rocket Tokio не блокируется; PgPool/transactions владеет AsyncStdDbExecutor. Runtime alignment МОЖЕТ быть отдельным будущим ADR после benchmark/approval, но текущая реализация обязана поддержать заданные features.

## ADR-008: Logical Security API, local hot-path transport

**Статус:** принято.

Core создаёт canonical JSON authorization request. Local client вызывает отдельный PDP crate без self-HTTP; internal mTLS endpoint предоставляет тот же contract другим services.

## ADR-009: Permissions before query

**Статус:** принято без исключений.

PDP obligations переписывают AST до SQL; RLS и serializer — дополнительные барьеры. Post-fetch filtering отклонено из-за утечки/стоимости.

## ADR-010: Два класса групп

**Статус:** принято.

Capability groups и data groups не смешиваются. Это делает review/least privilege понятными.

## ADR-011: Default deny / deny-overrides

**Статус:** принято.

Allow-by-absence запрещён. Break-glass — отдельный audited flow, не rule priority trick.

## ADR-012: Idempotency ledger + semantic keys

**Статус:** принято.

Транспортная идемпотентность не заменяет identity/fact uniqueness, и наоборот.

## ADR-013: Keyset pagination

**Статус:** принято.

OFFSET отклонён для больших изменяемых наборов. Opaque signed cursor binds query/schema/policy.

## ADR-014: Background jobs/outbox

**Статус:** принято.

Внешние calls, reports, bulk и monitoring не выполняются в request transaction. Exactly-once effects достигаются inbox/ledger, не обещанием broker.

## ADR-015: Nuxt authenticated SPA

**Статус:** принято.

Nuxt 4/Nuxt UI 4, ssr:false для app/admin. Публичные pages могут SSR. Уменьшает risk cache/session leakage; Rocket остаётся business API.

## ADR-016: UI metadata-driven, security not UI-driven

**Статус:** принято.

Views/forms declarative. Client capabilities улучшают UX, но API повторно решает доступ.

## ADR-017: Fuzzy matching never auto-merges by default

**Статус:** принято.

Exact verified keys могут return existing; probabilistic result создаёт candidate/review.

## ADR-018: Holding as graph view

**Статус:** принято.

Holding entity optional, composition вычисляется по ownership/control/affiliation temporal graph. Nested duplicate storage отклонено.

## ADR-019: International defaults

**Статус:** принято.

UTF-8/NFC/BCP47/IANA/decimal/typed values. Locale-specific assumptions запрещены core.

## ADR-020: No ordinary physical delete

**Статус:** принято.

Retire/revoke/tombstone сохраняют смысл/audit. Regulated erasure выполняется отдельным workflow с legal hold/key/backup plan.

## ADR-021: Service account does not inherit initiator data access

**Статус:** принято.

Job binds initiating approved snapshot/current authorization; schedule service principal имеет собственный narrow grant.

## ADR-022: Report/export security is dataset security

**Статус:** принято.

Templates cannot request undeclared/forbidden fields. Artifacts получают classification, TTL и download authorization.

## ADR-023: Minimum PostgreSQL feature baseline

**Статус:** принято.

Production baseline PostgreSQL 17+ (CI additionally validates current supported major). Возможности newer major используются только после compatibility ADR/migration. Deployment фиксирует exact major/minor policy.

## Изменение ADR

Любое изменение требует контекста, alternatives, consequences, migration, security/performance impact, approval и обновления зависимых документов/tests. Старый ADR не переписывается молча: статус superseded и ссылка на новый.
