# План реализации и критерии приёмки

## 1. Вертикальные этапы

### Phase 0 — Foundation

Workspace, CI, typed config/secrets, Rocket health/error/context, DbExecutor, PostgreSQL migrations, observability, local dev stack.

**Exit:** безопасное приложение подключается к DB, migration/backup smoke, no secret, baseline SCA.

### Phase 1 — Security/meta-kernel

Tenant/identity/session, two group classes, PDP/PEP/RLS/audit; type/predicate/schema draft/publish.

**Exit:** persona matrix default-deny; schema публикуется; cross-tenant/field/edge leak tests проходят.

### Phase 2 — Entity graph

Entity/statements, identity/idempotency, CRUD/history, query AST/keyset, relations/graph, projections/search.

**Exit:** arbitrary type created without code; duplicate phone/company prevented; 100M benchmark profile meets agreed SLO.

### Phase 3 — Nuxt core/admin

Auth/layout, dynamic lists/forms/cards/relations/tree, jobs; schema/security admin, i18n/a11y.

**Exit:** новый type появляется после publish без deploy; role UI/E2E/RTL/keyboard pass.

### Phase 4 — SKR business package

ФЛ/ЮЛ/holding, request/check/risk/workflow, Excel import, required views.

**Exit:** scenarios from DOCX/XLSX complete; no hardcoded core object.

### Phase 5 — Connectors/monitoring

Connector SDK, prioritized sources, provenance, schedules, risk criteria/reactions.

**Exit:** sandbox checks, stale/error semantics, idempotent monitoring, legal/secret/quota controls.

### Phase 6 — Reporting/production

Reports/exports, performance tuning, HA/backup/DR, penetration, migration/cutover.

**Exit:** production checklist, restore/rollback drill, signed release, UAT.

## 2. Priority of connectors

Order is approved by business/legal/availability:

1. existing internal DB;
2. supplied structured file imports;
3. contracted authoritative tax/company source;
4. enforcement/court/procurement/registration/health sources;
5. Internet/social sources last due legal/noise risk.

## 3. Definition of Done feature

- requirement/source link;
- domain/API/schema design;
- security classification/policy;
- migrations/backfill/rollback;
- tests including negative;
- logs/metrics/traces/audit;
- i18n/a11y;
- performance/limits;
- operational runbook;
- OpenAPI/user/admin docs;
- no warnings/vulnerable deps;
- review/approval.

## 4. End-to-end acceptance scenarios

### A. Универсальность

Admin создаёт package «Склад»: Item, Location, Batch, ownership/movement fields/relations, identity/views/rights. Publish без Rust/Nuxt deploy; CRUD/search/report работают.

### B. Идемпотентный телефон

100 concurrent creates с разным format одного номера и одним identity scope создают одну entity/key; retries получают один ID; different extension создаёт distinct по policy.

### C. ЮЛ Excel

Обе supplied workbooks проходят preview/dry run. Поля маппятся в graph, repeat import no duplicate. Руководитель/учредитель/доля/аффилированность — relations с provenance.

### D. Топологическая миграция

Тип изменяется split/reify/rename. Forward + reverse восстанавливают source facts; residual accessible; invalid rows quarantine; concurrent changes попадают в target projection.

### E. Скрытие ФЛ

Пользователь с holdings allow/person deny:

- видит разрешённые холдинги/ЮЛ;
- не находит ФЛ direct/list/search;
- не видит person fields/edges/count/facet/path;
- отчёт/export/WebSocket/cache не содержат их;
- DB instrumentation подтверждает, что person rows не materialized.

### F. Полевая безопасность

Два пользователя открывают то же ЮЛ: один видит passport/phone руководителя, второй не видит keys, DOM, network, sort/filter/facet/export. Update скрытого field запрещён mass assignment.

### G. Проверка

Менеджер submit request; СБ запускает sources; один source timeout не равен clean result; observations имеют provenance; factor/assessment/task/decision/report/monitor создаются идемпотентно.

### H. Мониторинг

Плановая проверка на DST boundary запускается по policy один раз; repeated external result не создаёт duplicate factor/notification; реальное изменение создаёт evidence/task.

### I. Большие данные

На agreed 100M/500M dataset exact/small/list/graph/write SLO выполняются, memory bounded, no OFFSET, no unapproved sequential scan.

### J. Международность

Russian/Arabic/Chinese names, IDN e-mail, French decimal, Turkish case, DST time, RTL forms/reports и emoji сохраняются/ищутся/экспортируются без corruption.

### K. Audit/DR

Export/decision/policy publish связаны в tamper-evident audit. Backup восстанавливается до target RPO/RTO, projections rebuild, outbox reconciled, security canaries pass.

## 5. Performance acceptance

Конкретные thresholds утверждаются на reference hardware. Минимум:

- exact lookup/card/list P95 targets из vision;
- zero unauthorized rows;
- bounded graph/export;
- projection lag target;
- no connection queue collapse at planned concurrency;
- soak без growing memory/bloat beyond threshold.

## 6. Security acceptance

- threat model reviewed;
- independent penetration;
- critical/high remediated;
- RLS/PEP matrix;
- MFA/four-eyes/break-glass;
- secrets/key rotation;
- dependency/container scan;
- incident/restore drill;
- legal basis/retention for enabled connectors.

## 7. Documentation acceptance

Все links существуют, terms согласованы, diagrams/examples не противоречат contracts, source requirements traced, ADR updated, migration/runbook present. Изменение архитектуры одновременно меняет docs/tests.

## 8. Не критерий готовности

Не считается готовым: «работает на малом тесте», UI скрывает кнопку, один SELECT перед INSERT, generic JSONB, post-filter permissions, ручной SQL в production, непроверенный backup, отчёт без field policy, connector с plaintext secret, отсутствие performance dataset.
