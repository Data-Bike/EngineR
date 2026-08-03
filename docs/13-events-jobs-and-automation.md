# События, задания и автоматизация

## 1. Domain events

Событие — факт о совершившемся commit:

- EntityCreated/Changed/Retired/Merged/Split;
- StatementAsserted/Revoked/Verified;
- SchemaPublished;
- PolicyPublished/MembershipChanged;
- CheckRequested/Completed/Failed;
- RiskFactorDetected/Resolved;
- AssessmentChanged;
- TaskCreated/Overdue/Completed;
- MonitorDue/Triggered;
- ReportReady;
- ProjectionAdvanced.

Envelope: event_id UUIDv7, type/version, tenant, aggregate/public ID, aggregate revision, occurred/recorded, actor, request/correlation/causation, classification, payload.

События не содержат больше PII, чем нужно consumer; sensitive detail читается повторно через PEP.

## 2. Transactional outbox

Canonical mutation + outbox в одной transaction. Dispatcher:

- SELECT ready FOR UPDATE SKIP LOCKED;
- marks lease;
- publishes/dispatches;
- writes attempt/error;
- exponential backoff + jitter;
- dead-letter после policy threshold;
- published_at only after acknowledgement.

Exactly-once transport не обещается; consumer effects идемпотентны.

## 3. Inbox

Каждый consumer сначала регистрирует consumer+event+version. completed возвращает previous outcome. processing lease можно перехватить после expiry. Handler не отмечает completed до фиксации локального эффекта/outbox.

## 4. Job state machine

queued → leased → running → waiting_external/waiting_user → succeeded/partially_succeeded/failed/cancelled.

Job хранит progress numerator/denominator, phase, heartbeat, attempts, deadline, lease, resumable cursor, result/error artifact. Cancellation cooperative; irreversible step сообщает cannot_cancel_after.

## 5. Worker queues

Раздельные workload classes:

- projections;
- imports/exports;
- connectors/checks;
- risk/workflow;
- reports;
- notifications;
- maintenance.

Отдельные concurrency/pool/priority предотвращают вытеснение interactive-critical projection большим экспортом.

## 6. Scheduler

Schedule хранит IANA time zone, calendar expression, next occurrence, misfire policy, concurrency policy и owner. DST:

- nonexistent local time: skip/run-next по policy;
- repeated time: once/twice policy;
- фактический run имеет planned UTC instant и uniqueness schedule+occurrence.

Leader/lease гарантирует один enqueue; сам job идемпотентен.

## 7. WorkflowDefinition

Декларативная state machine:

- states/transitions;
- trigger event/manual/schedule;
- conditions в safe DSL;
- required capabilities/data scope;
- actions;
- timers/SLA;
- assignment strategy;
- escalation;
- compensation;
- version.

Running instance остаётся на pinned workflow version либо проходит явно описанную migration.

## 8. Сценарий проверки контрагента

1. Менеджер создаёт Request для нового/действующего контрагента.
2. Identity разрешает существующего или создаёт candidate.
3. Workflow проверяет обязательные сведения.
4. СБ получает task.
5. Connector jobs запрашивают разрешённые источники.
6. Mapping создаёт provenance statements.
7. Risk engine пересчитывает factors/assessment.
8. Рисковик получает task при пороге/лимите/отсрочке.
9. Decision фиксируется отдельной entity с автором и доказательствами.
10. Уведомляются инициатор/подразделение.
11. Создаётся monitor schedule.
12. Report генерируется snapshot job.

Каждый шаг виден в timeline и audit.

## 9. Мониторинг

MonitorDefinition задаёт target query/entity, sources, interval, change detector, criteria, quiet period, notification/workflow и legal basis.

Run:

- фиксирует target/schema/policy;
- получает observations;
- сравнивает canonical normalized previous/current;
- подавляет технический шум;
- создаёт ChangeDetected;
- запускает risk/workflow;
- обновляет next_run.

Ошибка источника не интерпретируется как отсутствие риска. Staleness становится отдельным factor.

## 10. Реакции

Разрешённые actions registry:

- создать task/case;
- изменить category через command;
- запустить check/report;
- отправить e-mail/webhook/message;
- уведомить внешнюю систему;
- запросить approval;
- эскалировать.

Каждое действие имеет schema, permission, idempotency, timeout, retry и compensation. Arbitrary shell/script запрещён. On-prem specialised script запускается sandboxed connector runner с allowlist и resource limits.

## 11. Уведомления

Template version, locale, channel, recipient resolution, classification и redaction. Delivery ledger не допускает повторной отправки. Секретные данные не включаются в subject/push; ссылка ведёт в авторизованный UI. Bounce/failure создаёт observable state.

## 12. SLA

Business calendar versioned по tenant/location. Due instant вычисляется один раз и хранится вместе с calendar version. Изменение календаря не переписывает старые deadlines без explicit recalculation command.

## 13. Failures

Ошибки классифицируются transient, rate_limited, auth, validation, permanent, policy_denied, cancelled. Retry только transient/rate_limited. Auth блокирует connector и уведомляет owner; policy_denied не обходится service account.

## 14. Operations UI

Показывает queues, lag, leases, attempts, dead letters, source health, projection watermark и job details. Повтор требует capability и сохраняет original job/operation id.
