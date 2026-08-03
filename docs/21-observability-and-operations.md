# Наблюдаемость и эксплуатация

## 1. Три сигнала

### Logs

Structured JSON:

- timestamp/level/service/version/environment;
- request/correlation/trace;
- tenant hashed/approved ID;
- route/action/outcome/status;
- duration/error ID;
- query/plan fingerprint;
- job/event/connector IDs.

Запрещены token/cookie/password/secret, raw request/response, SQL with values, passport/ИНН/phone/email и connector payload. Debug fields проходят redaction allowlist.

### Metrics

RED для API, USE для DB/worker/resources:

- request rate/error/duration;
- DbExecutor queue/pool/execute/saturation;
- SQL fingerprints duration/rows/timeouts;
- outbox lag/attempt/dead-letter;
- job queue/lease/heartbeat/failure;
- projection watermark/lag/rebuild;
- connector latency/quota/circuit/error class;
- PDP latency/cache/deny/indeterminate;
- login/MFA/session anomalies;
- report/import/export rows/bytes;
- PostgreSQL connections/WAL/replica/vacuum/bloat;
- object storage errors/scan backlog.

Labels имеют bounded cardinality: нельзя label entity/user/request ID.

### Traces

W3C trace context через Rocket → PEP → DbExecutor → worker/connector. Spans не содержат sensitive payload. Async job связывается через links/causation ID, не искусственно вечный span.

## 2. Audit vs logs

Audit — бизнес/security доказательство с retention/integrity и отдельным доступом. Operational log — диагностика с коротким TTL. Нельзя считать application log аудитом или копировать весь audit в log system.

## 3. Health

- /health/live: process/event loop alive, без внешних calls.
- /health/ready: config valid, DB usable, PDP/policy loaded, critical storage/queue available, migrations compatible.
- /health/startup: initial schema/cache loading.
- internal detailed health protected; public response generic.

Connector failure обычно degraded, не снимает API readiness, если core может работать. Невозможность авторизации снимает readiness/fails closed.

## 4. SLO/error budget

SLI отдельно для authenticated API, mutations, jobs, connector checks и reports. Planned maintenance исключается только по policy. Error budget влияет на feature rollout и performance work. Availability не скрывает incorrect/unauthorized responses.

## 5. Alerting

Actionable alerts:

- cross-tenant/RLS test canary — critical;
- authentication spike/reuse;
- PDP indeterminate/unavailable;
- DB saturation/replica lag/WAL disk;
- outbox/projection lag;
- stuck leases/jobs;
- connector auth/quota exhaustion;
- backup failure/restore age;
- audit chain/checkpoint failure;
- certificate/key expiry;
- disk/object storage capacity.

Каждый alert имеет owner, severity, runbook, silence policy и test.

## 6. Dashboards

- executive SLO;
- API/DB performance;
- security/auth/PDP;
- jobs/events/projections;
- connectors/checks;
- imports/exports/reports;
- PostgreSQL maintenance/capacity;
- tenant fairness без раскрытия PII.

## 7. Backup

- PostgreSQL continuous WAL + scheduled base backups;
- object storage versioning/replication;
- schema bundles/templates/config refs;
- KMS/key backup согласно provider;
- encrypted, access-controlled, geographically compliant;
- automated restore validation в isolated environment;
- documented RPO/RTO.

Backup считается существующим только после успешного restore drill.

## 8. Disaster recovery

Runbook:

1. declare incident/freeze destructive operations;
2. choose restore point;
3. restore DB/object store/keys;
4. verify migrations/audit integrity;
5. reconcile outbox/inbox/jobs;
6. rebuild projections/search;
7. validate tenant/security canaries;
8. reopen read then write traffic;
9. document data gap.

Targets задаются deployment tier; production initial goal: RPO ≤15 минут, RTO ≤4 часа, затем ужесточается по бизнес-требованиям.

## 9. Graceful shutdown

API stops readiness, rejects new long work, drains HTTP, cancels/waits DB jobs, flushes telemetry. Worker stops leasing, heartbeats owned jobs, completes safe chunk или сохраняет cursor/releases lease. Outbox rows остаются retryable.

## 10. Runbooks

Обязательны:

- DB unavailable/saturated;
- slow query/plan regression;
- RLS/policy issue;
- leaked/compromised credential;
- connector outage/schema drift;
- projection lag/rebuild;
- stuck/dead-letter job;
- failed migration;
- disk/WAL growth;
- backup restore;
- policy/schema rollback;
- report/data leak;
- key/certificate rotation.

## 11. Capacity

Ежемесячно прогнозируются entities/statements/history, index/projection, WAL, backup, report/blob storage, DB CPU/IO/memory, worker backlog и connector quota. Threshold запускает planned scaling, не аварийное увеличение.

## 12. Operational access

Production access JIT/time-limited, MFA, bastion, least privilege, approval/audit. DBA не использует application user. Raw data query требует ticket/purpose и по возможности masked read replica.
