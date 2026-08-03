# EngineR

> **Лозунг:** гибкость без потери производительности.

Универсальная платформа данных и автоматизации. Первый прикладной пакет —
СКР САНАР (Система Контроля Рисков). Полные спецификации — в [`docs/`](./docs).

Этот репозиторий содержит **Phase 0 + Phase 1** фундамент новой архитектуры
(см. [`docs/25-delivery-plan-and-acceptance.md`](./docs/25-delivery-plan-and-acceptance.md)):

- Cargo workspace `apps/` + `crates/` (docs/05);
- типизированные newtypes, доменные типы, ошибки (docs/03, docs/05);
- контракты API: envelope, error codes, DTO (docs/06);
- application ports + `RequestContext` (docs/02, docs/05);
- `AsyncStdDbExecutor` — мост Tokio (Rocket) ↔ async-std (sqlx PgPool) (docs/02 §6, ADR-007);
- persistence: RLS context, репозитории security/schema, outbox (docs/08 §8, docs/10);
- security: два класса прав, PDP/PEP, default-deny, audit (docs/08, docs/09);
- schema registry: draft → validate → publish (docs/12);
- identity: нормализаторы, identity keys (docs/04 §6-7);
- `enginer-api` (Rocket) и `enginer-worker` (скелет);
- миграции PostgreSQL для схем `core/meta/sec/ops/audit` (docs/10);
- `docker-compose.yml` для локального PostgreSQL;
- тесты инвариантов (default-deny, RLS, идемпотентность, schema publish).

Старый прототип перенесён в [`legacy-prototype/`](./legacy-prototype/) и исключён
из workspace (reference / mapping source).

## Быстрый старт

```bash
docker compose up -d postgres
cargo test --workspace
cargo run -p enginer-api
```

См. [`AGENTS.md`](./AGENTS.md) для команд проверки.
