# Структура Rust workspace и правила реализации

## 1. Целевая структура

~~~text
/
├─ Cargo.toml
├─ Cargo.lock
├─ rust-toolchain.toml
├─ apps/
│  ├─ api/
│  └─ worker/
├─ crates/
│  ├─ domain/
│  ├─ application/
│  ├─ contracts/
│  ├─ schema-registry/
│  ├─ identity/
│  ├─ query-engine/
│  ├─ persistence-postgres/
│  ├─ db-runtime-async-std/
│  ├─ security-domain/
│  ├─ security-pdp/
│  ├─ security-api/
│  ├─ connectors/
│  ├─ risk-engine/
│  ├─ workflow/
│  ├─ reporting/
│  ├─ files/
│  ├─ observability/
│  └─ test-support/
├─ migrations/
├─ schema-packages/
│  └─ skr-core/
├─ web/
└─ docs/
~~~

Зависимости направлены внутрь: apps → adapters/application → domain. Domain не импортирует adapters.

## 2. Ответственность crates

| Crate | Содержимое |
|---|---|
| domain | ID newtypes, entities, statements, value objects, invariants, domain events |
| application | commands, queries, use cases, ports, transactions |
| contracts | HTTP DTO, error codes, API version, JSON schema |
| schema-registry | draft/publish, validation, compatibility, migration plans |
| identity | normalizers, identity keys, match candidates, merge/split |
| query-engine | AST, validation, policy rewrite, SQL plan model |
| persistence-postgres | sqlx repositories, row mapping, outbox, projections |
| db-runtime-async-std | runtime bridge и PgPool owner |
| security-domain | principals, capabilities, data scopes, decisions |
| security-pdp | policy evaluation/obligations |
| security-api | internal Rocket controllers и clients |
| connectors | SDK, rate limit, raw payload/provenance |
| risk-engine | criteria DSL, factor/score/decision |
| workflow | state machine, tasks, timers, reactions |
| reporting | datasets, template rendering, export jobs |
| files | staged upload, object storage, scanning |
| observability | tracing, metrics, audit helpers |
| test-support | fixtures, fake clock, generators, DB harness |

## 3. Cargo workspace

- version/resolver/edition задаются в workspace.package/workspace.dependencies;
- production dependency pinning выполняется Cargo.lock;
- default features отключаются там, где расширяют attack surface;
- cargo-deny проверяет licenses, advisories, duplicates и banned crates;
- cargo-audit/обновление lock запускаются регулярно;
- unsafe запрещён workspace lint, кроме отдельно одобренного crate с ADR;
- warnings запрещены CI.

Требуемая совместимость sqlx фиксируется:

~~~toml
sqlx = { version = "0.5.13", default-features = false,
  features = ["runtime-async-std-native-tls", "postgres", "chrono", "json", "uuid", "migrate"] }
rocket = { version = "0.5.1", features = ["json", "secrets"] }
~~~

Дополнительные features добавляются только при реальном использовании. SQLx закрыт портом, поэтому его обновление не затрагивает domain/contracts.

## 4. Типы вместо строк

Обязательные newtypes:

- TenantId, EntityId, PublicEntityId;
- TypeId, PredicateId, SchemaRevisionId;
- StatementId, PolicyId, DecisionId;
- PrincipalId, GroupId, SessionId;
- RequestId, CorrelationId, IdempotencyKey;
- JobId, EventId, ProjectionRevision;
- LocaleTag, TimeZoneId, ContentHash.

Нельзя передавать идентификаторы как взаимозаменяемые String.

## 5. Application ports

Минимальные traits:

- UnitOfWork/TransactionRunner;
- EntityRepository, StatementRepository, SchemaRepository;
- IdentityKeyRepository;
- QueryExecutor;
- SecurityDecisionPoint;
- Outbox, AuditSink;
- Clock, IdGenerator;
- BlobStore, ConnectorInvoker, JobQueue.

Порт принимает/возвращает domain/application types. PgRow/rocket::Request/serde_json::Value не пересекают границу.

## 6. DbExecutor

~~~rust
#[async_trait]
pub trait DbExecutor: Send + Sync {
    async fn run<T, F, Fut>(&self, deadline: Deadline, f: F) -> Result<T, DbError>
    where
        T: Send + 'static,
        F: FnOnce(PgPoolHandle) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, DbError>> + Send + 'static;
}
~~~

Реальная сигнатура может использовать object-safe command enum или boxed job. Требования:

- pool создаётся/закрывается в async-std runtime;
- bounded queue и cancellation token;
- transaction closure не покидает runtime;
- panic в job перехватывается, канал не разрушается;
- graceful shutdown ждёт inflight с deadline;
- readiness false при недоступной БД/насыщенной очереди.

## 7. RequestContext

Содержит tenant, principal, session, request/correlation IDs, locale/timezone, auth strength, client metadata, deadline, policy snapshot revision и purpose of use. Создаётся guard-ом и передаётся явно; thread-local/global mutable state запрещён.

## 8. Ошибки

Каждый слой имеет enum:

- DomainError: validation/conflict/invariant;
- ApplicationError: not_found/precondition/idempotency/dependency;
- SecurityError: unauthenticated/denied/policy_unavailable;
- PersistenceError: conflict/timeout/unavailable/corrupt_data;
- ApiError: стабильный code, status, safe details.

Внешний ответ не содержит SQL, stack trace, секрет или факт существования скрытого объекта. Internal cause сохраняется в tracing с error ID.

## 9. Транзакционные правила

- use case объявляет одну transaction boundary;
- внешняя сеть/файл не вызываются внутри DB transaction;
- последовательность lock всегда identity → entity → statement → outbox;
- retry только для классифицированных serialization/deadlock errors с jitter и общим deadline;
- side effect публикуется через outbox;
- isolation по умолчанию READ COMMITTED; SERIALIZABLE только для доказанной гонки;
- optimistic concurrency через entity revision/If-Match.

## 10. Стиль и качество

- rustfmt, clippy -D warnings;
- public API документирован;
- функции hot path имеют ограниченную cyclomatic complexity;
- запрещены format-строки с пользовательскими значениями в SQL;
- секреты используют secrecy/zeroize-compatible wrappers;
- пароли — Argon2id с версионированными параметрами;
- время — UTC instant и явная IANA zone на границах;
- decimal, а не float, для финансовых величин;
- serde deny_unknown_fields для command DTO; tolerant readers только для событий.

## 11. Конфигурация

Typed settings загружаются в порядке defaults → file → environment → secret provider. Пароли не хранятся в config.toml репозитория. Startup валидирует:

- URL и TLS mode БД;
- pool/queue sizes;
- signing/encryption key IDs;
- public/internal origins;
- object storage;
- timeouts/limits;
- connector secret refs;
- audit availability.

Resolved config выводится только с redaction.

## 12. Feature flags

Release feature flag отделён от schema config. Флаг имеет owner, expiry, default, audience и audit. Нельзя использовать compile-time Cargo feature для tenant business capability.
