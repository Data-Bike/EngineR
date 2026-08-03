# Контракт Rocket JSON API

## 1. Общие правила

- Public base: /api/v1.
- Internal base: /internal/v1 и /internal/security/v1.
- HTTPS обязательно вне локальной разработки.
- JSON UTF-8; Content-Type application/json.
- Денежные/decimal значения передаются строкой.
- Instants — RFC 3339 UTC; local dates не получают фиктивную zone.
- Public IDs — UUID; internal bigint никогда не раскрывается.
- Неизвестные поля command DTO отклоняются.
- Ответы имеют X-Request-Id и X-Correlation-Id.
- OpenAPI/JSON Schema генерируются из contracts crate и проверяются snapshot-тестами.

## 2. Envelope

Успех:

~~~json
{
  "data": {},
  "meta": {
    "request_id": "01...",
    "schema_revision": "42",
    "commit_revision": "9012",
    "policy_revision": "77"
  },
  "links": {}
}
~~~

Ошибка:

~~~json
{
  "error": {
    "code": "entity.version_conflict",
    "message_key": "errors.entity.version_conflict",
    "parameters": {"expected": "8", "actual": "9"},
    "request_id": "01..."
  }
}
~~~

message_key локализует Nuxt; fallback message может быть передан, но код/параметры нормативны.

## 3. HTTP semantics

- GET безопасен и кешируем только при разрешённом Cache-Control.
- POST создаёт command/job или сложный query.
- PUT заменяет версионированный декларативный ресурс.
- PATCH использует собственный typed patch, не произвольный JSON Patch для statements.
- DELETE выполняет retire/revoke и возвращает tombstone; физическая очистка — отдельный approval workflow.
- 401 — нет/недостаточно сильная аутентификация.
- 403 — capability известна, но запрещена; скрытые data resources обычно отвечают 404.
- 409 — identity/version/idempotency conflict.
- 412 — неверный If-Match.
- 422 — семантическая validation.
- 429 — quota/rate limit с Retry-After.
- 503 — dependency/security fail-closed.

## 4. Authentication

Предпочтительно OIDC Authorization Code + PKCE и secure HttpOnly SameSite cookie для intranet. API также поддерживает short-lived bearer token для service accounts.

Endpoint:

- POST /auth/session/exchange;
- POST /auth/refresh;
- POST /auth/logout;
- GET /me;
- GET /me/context;
- POST /auth/step-up.

Refresh rotation/reuse detection обязательны. Cookie-changing request требует CSRF token/Origin policy.

## 5. Schema API

- GET /schemas/types;
- GET /schemas/types/{type_id};
- POST /schemas/drafts;
- PUT /schemas/drafts/{id};
- POST /schemas/drafts/{id}/validate;
- POST /schemas/drafts/{id}/impact;
- POST /schemas/drafts/{id}/publish;
- POST /schemas/revisions/{id}/retire;
- GET /schemas/revisions/{id}/migration-status.

Publish требует If-Match, privilege, impact report и audit reason.

## 6. Entity API

- POST /entities;
- GET /entities/{public_id};
- POST /entities/query;
- PATCH /entities/{public_id};
- POST /entities/{public_id}/retire;
- POST /entities/{public_id}/restore;
- POST /entities/{public_id}/merge;
- POST /entities/{public_id}/split;
- GET /entities/{public_id}/history;
- GET /entities/{public_id}/related;
- POST /entities/{public_id}/graph.

Create:

~~~json
{
  "type_id": "uuid",
  "schema_revision": "42",
  "identity_mode": "return_existing",
  "statements": [
    {"predicate_id": "uuid", "value": {"kind": "text", "value": "ООО Пример"}},
    {"predicate_id": "uuid", "value": {"kind": "text", "value": "7700000000"}}
  ]
}
~~~

Ответ указывает created/deduplicated, entity, revision, validation warnings и доступные actions.

## 7. Statement API

Statements обычно меняются через entity command, чтобы проверить cardinality/revision вместе. Специализированные endpoint:

- POST /entities/{id}/statements;
- POST /statements/{id}/revoke;
- POST /statements/{id}/verify;
- GET /statements/{id}/provenance.

Нельзя менять predicate/subject/object существующего assertion in-place.

## 8. Связи и граф

Related endpoint использует predicate allowlist, direction, as_of, cursor и page_size.

Graph request:

~~~json
{
  "as_of": "2026-08-03T00:00:00Z",
  "max_depth": 3,
  "max_nodes": 500,
  "predicates": ["uuid"],
  "direction": "both",
  "include_summaries": true
}
~~~

Сервер пересекает limits с security obligations. Цикл возвращается один раз как edge на уже видимый node. Продолжение — signed opaque cursor.

## 9. Query

POST /entities/query принимает AST из [query engine](./07-query-engine.md). Page size default 50, max 500 для UI. Cursor opaque, подписан, содержит sort keys, schema/policy/query fingerprints и expiry. OFFSET запрещён.

## 10. Mass operations

- POST /jobs/imports;
- POST /jobs/exports;
- POST /jobs/bulk-mutations;
- GET /jobs/{id};
- POST /jobs/{id}/cancel;
- GET /jobs/{id}/errors;
- GET /jobs/{id}/artifact.

Каждая операция имеет dry_run, schema revision, policy snapshot, per-row result, retry semantics и limits. Частичный успех явен. Browser не держит HTTP request до завершения.

## 11. Checks, monitoring and reports

- POST /checks;
- GET /checks/{id};
- POST /checks/{id}/run;
- POST /monitors;
- PATCH /monitors/{id};
- POST /reports;
- GET /reports/{id};
- GET /reports/{id}/download.

Запуск источника возвращает 202 + job. Синхронно допустим только локальный быстрый критерий.

## 12. Admin/security

Публичная admin surface использует contracts, отличные от internal PDP:

- /admin/users, /admin/service-accounts;
- /admin/capability-groups, /admin/data-groups;
- /admin/policies, /admin/policy-simulations;
- /admin/connectors, /admin/translations;
- /admin/audit, /admin/jobs.

Secrets никогда не возвращаются после записи, только present/version/last_rotated.

## 13. Optimistic concurrency

GET возвращает ETag вида "entity:uuid:revision". Mutation требует If-Match либо явную expected_revision. Если target изменился — 412 с безопасным conflict summary. Force update — отдельная capability и reason.

## 14. Caching

ETag учитывает entity revision, view, locale и policy scope. Ответы с персональными/секретными данными: private, no-store либо private revalidation. CDN/shared cache для authenticated API запрещён без доказанной partition key.

## 15. WebSocket/SSE

/api/v1/events после authentication:

- подписки только на разрешённые topics/resources;
- payload уже field-filtered;
- policy revision changes вызывают reauthorization/closure;
- reconnect по last event cursor;
- bounded buffer, heartbeat и backpressure;
- секретные значения не попадают в channel names/logs.

События уведомляют о revision/job state, но клиент после события перечитывает канонический ресурс.

## 16. Limits

Централизованный LimitProfile задаёт body size, statements per command, filters, sort keys, page size, graph depth/nodes, export rows/bytes, upload bytes, concurrent jobs и request deadline. Повышение — capability + quota policy.
