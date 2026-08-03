# Security API: контракты PDP/PEP

## 1. Назначение

Security API принимает JSON-описание субъекта, действия, ресурса/запроса и возвращает решение с обязательствами. Контракт одинаков для in-process LocalSecurityClient и mTLS internal HTTP.

Локальный transport всё равно сериализует/десериализует contract в contract tests, чтобы remote mode не расходился. В production hot path не выполняет HTTP к самому себе.

## 2. Endpoints

- POST /internal/security/v1/decisions/action;
- POST /internal/security/v1/decisions/query;
- POST /internal/security/v1/decisions/mutation;
- POST /internal/security/v1/decisions/blob;
- POST /internal/security/v1/simulations;
- GET /internal/security/v1/policies/revision;
- POST /internal/security/v1/cache/invalidate.

Internal endpoint требует service identity и не принимает principal только из body: caller identity проверяется mTLS/token, а delegated subject — отдельно.

## 3. Общий request context

~~~json
{
  "request_id": "01...",
  "tenant_id": "uuid",
  "principal": {
    "id": "uuid",
    "session_id": "uuid",
    "auth_strength": "mfa",
    "groups_revision": "91"
  },
  "purpose": "counterparty_due_diligence",
  "locale": "ru-RU",
  "time": "2026-08-03T10:00:00Z",
  "client_service": "enginer-api"
}
~~~

PDP не доверяет списку групп из клиента; он загружает membership либо проверяет подписанный snapshot.

## 4. Action decision

Request:

~~~json
{
  "context": {},
  "action": "report.export",
  "resource": {"kind": "report_definition", "id": "uuid"},
  "parameters": {"format": "xlsx", "estimated_rows": 100000}
}
~~~

Response:

~~~json
{
  "decision_id": "uuid",
  "effect": "allow",
  "policy_revision": "77",
  "obligations": {
    "max_rows": 50000,
    "watermark": "CONFIDENTIAL",
    "require_step_up": false,
    "audit_level": "full"
  },
  "expires_at": "2026-08-03T10:01:00Z"
}
~~~

## 5. Query decision

Request передаёт не raw SQL, а normalized AST summary:

~~~json
{
  "context": {},
  "action": "entity.query",
  "query": {
    "root_type_ids": ["holding-uuid", "person-uuid"],
    "selected_predicate_ids": ["name-uuid", "passport-uuid"],
    "relation_predicate_ids": ["member-uuid"],
    "operations": ["filter", "sort", "graph"],
    "requested_page_size": 100,
    "requested_graph_depth": 4,
    "query_fingerprint": "hex"
  }
}
~~~

Response:

~~~json
{
  "decision_id": "uuid",
  "effect": "allow_with_obligations",
  "policy_revision": "77",
  "obligations": {
    "allowed_root_type_ids": ["holding-uuid"],
    "denied_type_ids": ["person-uuid"],
    "allowed_predicate_ids": ["name-uuid"],
    "denied_predicate_ids": ["passport-uuid"],
    "allowed_relations": [
      {"predicate_id": "member-uuid", "directions": ["out"], "target_type_ids": ["legal-entity-uuid"]}
    ],
    "mandatory_filters": {
      "all": [
        {"system_field": "classification", "op": "lte", "value": "internal"},
        {"system_field": "business_unit_id", "op": "in_subject_scope", "scope": "business_units"}
      ]
    },
    "masks": [],
    "max_page_size": 50,
    "max_graph_depth": 2,
    "max_graph_nodes": 300,
    "aggregates": "deny"
  },
  "expires_at": "2026-08-03T10:01:00Z"
}
~~~

PEP обязан проверить policy revision перед использованием cache/cursor.

## 6. Mutation decision

Request включает command, target type/entity, changed predicates, relation targets, expected revision и batch size. Response даёт:

- allow/deny;
- allowed changed predicates;
- required values/approvals;
- forbidden relation targets;
- field transforms;
- max batch;
- required reason;
- audit level.

Если часть mutation запрещена, команда по умолчанию отклоняется целиком. Partial mutation разрешена только отдельным endpoint/явным client option и возвращает per-operation result.

## 7. Row filters

Mandatory filters — типизированный security AST, отличный от user AST. Он может ссылаться только на заранее зарегистрированные system fields/scopes. PDP не возвращает SQL. Query engine компилирует оба AST одним trusted compiler.

In-subject-scope разрешается через server-side scope table/CTE. Огромный список IDs нельзя передавать в JSON; используется scope handle/revision.

## 8. Masks

Виды:

- omit;
- null;
- fixed;
- partial(prefix/suffix);
- hash/token;
- bucket/range;
- aggregate_only.

Mask содержит predicate, algorithm ID/version и условия. Нельзя выполнять маскирование до сортировки/filter, если это создаёт side channel; обычно скрытое поле запрещается в этих операциях.

## 9. Decision binding

Decision связывается с:

- tenant/principal/session/auth strength;
- action;
- resource/query fingerprint;
- schema/policy/groups revisions;
- purpose;
- expiry.

Decision ID не является bearer permission. PEP не может применить его к другому запросу.

## 10. Ответы/ошибки

- allow;
- allow_with_obligations;
- deny;
- indeterminate → PEP трактует deny;
- challenge → требуется step-up.

Codes: security.unauthenticated, security.capability_denied, security.data_denied, security.step_up_required, security.policy_stale, security.pdp_unavailable, security.context_invalid.

Наружу основной API преобразует data_denied в 404, где требуется скрыть существование.

## 11. Audit protocol

PEP отправляет outcome после выполнения: decision_id, rows/bytes returned, mutation revision, status, latency и error class. Если callback потерян, decision log всё равно существует; outbox доставляет outcome позже.

## 12. Защита internal API

- mTLS + service allowlist;
- audience-bound short token;
- network policy;
- body/depth/rate limits;
- replay protection request ID + timestamp;
- no CORS/browser access;
- safe logs;
- readiness зависит от policy store;
- fuzz/contract tests для всех DTO.
