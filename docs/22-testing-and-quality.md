# Тестирование и качество

## 1. Пирамида

1. Domain unit/property tests.
2. Crate contract tests.
3. PostgreSQL repository/integration tests.
4. Security negative/information-flow tests.
5. API/OpenAPI tests.
6. Connector sandbox/fixtures.
7. Nuxt component/E2E/a11y.
8. Migration/upgrade/rollback.
9. Performance/soak/failure.
10. Penetration/DR drills.

## 2. Domain property tests

Generators создают Unicode values, schema graphs, types, predicates, entities, temporal intervals и operations.

Свойства:

- normalization idempotent;
- canonical serialization deterministic;
- statement equality/fingerprint;
- symmetric edge order invariant;
- cardinality preserved under concurrency;
- merge/split lineage acyclic;
- migrations round-trip/residual complete;
- decimal/time calculations deterministic;
- risk evaluation same inputs → same result;
- workflow transition invariants.

## 3. Persistence tests

Запускаются на реальном supported PostgreSQL, не SQLite/mock:

- migrations fresh + upgrade from each supported release;
- constraints/FK/checks;
- unique identity/fingerprint races;
- optimistic locking;
- transaction rollback/outbox atomicity;
- SKIP LOCKED leases;
- RLS tenant/principal context;
- pooled connection context reset;
- partition pruning;
- index/plan snapshots;
- timezone/Unicode/decimal;
- fail/retry serialization.

## 4. Security matrix

Personas × actions × resources × fields × relations × purposes:

- default deny;
- deny overrides;
- two group classes;
- object/type/field/edge;
- direct GET/list/search/filter/sort/facet/count/graph/history/blob;
- report/export/import error file;
- WebSocket/event;
- cache;
- worker/schedule/service account;
- impersonation/break-glass;
- policy change mid-session.

Негативный тест проверяет не только response, но DB rows fetched, DOM/network/artifact/log/cache.

## 5. API contracts

- OpenAPI snapshot/backward compatibility;
- strict DTO/unknown fields;
- statuses/error codes;
- ETag/If-Match;
- idempotency commit-before-response;
- cursor tamper/stale;
- request limits/malformed/deep JSON;
- content-type/UTF-8;
- auth/CSRF/CORS;
- cancellation/deadlines;
- WebSocket reauthorization.

Consumer-driven tests для Nuxt/remote security clients.

## 6. Query tests

- AST parser fuzz/property;
- operator/scalar typing;
- SQL bind/no literals;
- security filter in every subquery/recursive step;
- cost guard;
- keyset completeness;
- null/collation;
- graph cycles/high degree;
- aggregate inference;
- equivalent AST fingerprint;
- injection strings.

## 7. Connector tests

Recorded synthetic fixtures (без production PII), sandbox live smoke, schema drift, pagination, rate/quota, retry/timeout, ambiguous result, malformed/huge response, duplicate observation, mapping version, legal flags, log redaction, secret rotation.

## 8. Import/report tests

- both supplied XLSX files parsed/mapped;
- multi-row founders/relations;
- date/decimal/Unicode;
- duplicate/retry;
- formula/macro/external link;
- zip bomb/MIME;
- error coordinates;
- 100k/1M streaming memory;
- DOCX/XLSX/PDF golden/visual;
- hidden fields absent from data, metadata, styles, charts;
- locale/RTL/fonts.

## 9. Nuxt tests

- dynamic renderer every kind;
- permissions remove DOM/actions;
- role persona scenarios;
- 412 conflict/draft preservation;
- policy event clears state;
- virtual cursor list;
- graph cycle/hidden node;
- jobs/reconnect;
- keyboard/focus/zoom/RTL/axe;
- CSP/XSS payloads;
- visual regression.

## 10. Performance

Scenario manifest фиксирует git SHA, schema, dataset seed/size, PostgreSQL config/hardware, concurrency, warmup/duration, SLO. Profiles include exact lookup, hot/cold card, list, full-text, graph, writes, import/export, policy cold/cache, projection catch-up.

Soak 8–24h проверяет leaks, pool starvation, bloat/lag, queues и tail latency. Results versioned; regression gate описан в [performance](./11-performance-and-scaling.md).

## 11. Chaos/failure

- kill API/worker at each job phase;
- DB restart/failover;
- replica lag;
- object storage unavailable;
- PDP/cache unavailable;
- connector timeout/partial;
- outbox duplicate/reorder;
- clock skew;
- disk pressure;
- response lost after commit.

Инварианты: no duplicate, no unauthorized fallback, resumable progress, explicit status.

## 12. Static/supply quality

- rustfmt/clippy -D warnings;
- cargo test/nextest;
- cargo deny/audit;
- TypeScript/vue-tsc/eslint;
- npm/pnpm audit/SCA;
- secret scanning;
- SBOM/license;
- container scan/sign;
- migration lint;
- OpenAPI breaking-change check.

## 13. Test data

Synthetic by default. Production dump запрещён без approved anonymization. Fixtures versioned and tagged classification. Faker поддерживает международные форматы, но не создаёт реальные идентификаторы случайно.

## 14. Coverage/DoD

Coverage percentage не заменяет risk. Critical domain/security/query code требует branch/property tests и mutation testing where feasible. Изменение считается готовым, когда есть requirements link, tests, telemetry, migration/rollback, docs, security/performance review по risk.
