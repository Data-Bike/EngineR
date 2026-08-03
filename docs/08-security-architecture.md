# Архитектура безопасности

## 1. Разделение

Security реализуется отдельными crates security-domain, security-pdp и security-api. Основной domain не решает доступ. Каждый controller/use case/query/export/worker проходит Policy Enforcement Point (PEP).

Security — часть API, но hot path использует LocalSecurityClient по тому же JSON contract без loopback HTTP. RemoteSecurityClient применяется при выделении PDP или для других систем.

## 2. Два класса прав

### 2.1 System capabilities

Доступ к функциям:

- login/use_ui;
- entity.create/edit/retire/merge;
- query.advanced/aggregate;
- check.run/monitor.manage;
- report.run/export;
- bulk.import/export/mutate;
- schema.draft/publish;
- security.manage/simulate;
- connector.manage;
- audit.read;
- admin.pages.access.

Capability groups объединяют только system rights.

### 2.2 Data permissions

Доступ к данным:

- tenant/namespace;
- type;
- entity;
- predicate field;
- relation predicate/direction;
- statement/provenance;
- classification/tag/ownership;
- row condition;
- historical period;
- purpose of use.

Data groups объединяют только data rights. Пользователь может состоять в нескольких группах обоих классов.

## 3. Модель policy

Rule содержит:

- effect allow/deny;
- subject/group;
- action;
- resource selector;
- condition;
- fields/relations;
- mask/obligation;
- priority;
- valid interval;
- policy revision;
- reason/legal basis.

Default deny. Явный deny имеет преимущество над allow. Исключение break-glass не является обычным приоритетом: это отдельный step-up flow с причиной, сроком и уведомлением.

## 4. Evaluation

1. Проверить tenant, session, auth strength.
2. Собрать memberships на policy snapshot.
3. Проверить capability.
4. Собрать применимые data rules.
5. Отбросить rules с ложным condition/expired.
6. Применить deny-overrides.
7. Сформировать разрешённое множество и obligations.
8. Ограничить запрос/команду.
9. Записать decision audit с policy IDs/revision.

Нельзя делать «если нет правил объекта, разрешить». Всегда fail closed.

## 5. Уровни enforcement

- Router guard: authentication, coarse capability, rate limit.
- Application PEP: command/resource/action.
- Query PEP: type/row/field/relation rewrite.
- Repository: tenant и policy predicates обязательны.
- PostgreSQL RLS: tenant/row defense-in-depth.
- Serializer: final field allowlist.
- Blob download: отдельное решение.
- Worker/report/export: service identity + initiating subject snapshot/current recheck.

UI visibility помогает UX, но не является security boundary.

## 6. Пример «холдинги разрешены, ФЛ запрещены»

PDP возвращает:

- root types: holding, legal_entity;
- denied type: person;
- relation allowlist исключает edges, ведущие к person, либо заменяет endpoint redacted marker по policy;
- fields исключают PII predicates;
- row predicate ограничивает доступные холдинги.

Query compiler добавляет type deny в корень, EXISTS и recursive CTE. Person rows/fields не извлекаются. Counts/facets/tree также не раскрывают их. Direct GET person отвечает 404.

## 7. Field/edge policy

Поле может быть allow, omit, null-mask, partial-mask, tokenize или aggregate-only. Omit предпочтителен: ключ отсутствует. Masking применяется в SQL/response projection и явно помечается meta только если разрешено знать о поле.

Edge permission проверяет predicate, direction, source/target type, endpoints и context. Запрет edge означает, что нельзя вывести edge, traverse через него или косвенно сортировать по target.

## 8. PostgreSQL defense-in-depth

- отдельные DB roles для migration, API и worker;
- API role не является owner/superuser и не имеет BYPASSRLS;
- transaction-local settings tenant/principal/purpose устанавливаются безопасно;
- FORCE ROW LEVEL SECURITY на tenant tables;
- RLS гарантирует tenant/простые row rules;
- сложные dynamic field policies остаются в query compiler;
- connection возвращается в pool только после rollback/reset context;
- admin maintenance использует отдельный audited path.

RLS не заменяет PEP, потому что не фильтрует JSON field structure и functional capabilities.

## 9. Аутентификация

- OIDC federation и optional local emergency admin;
- Authorization Code + PKCE;
- MFA/step-up для publish, export, security, break-glass;
- Argon2id для local password;
- short access token, rotating refresh, reuse detection;
- session/device/IP risk signals;
- service accounts с scoped credentials/mTLS;
- ключи с kid, rotation и overlap.

Пароли, refresh tokens и API secrets хранятся hash/encrypted по назначению; plaintext не логируется.

## 10. Policy cache

Ключ включает tenant, principal, groups revision, policy revision, auth strength и purpose. Cache хранит compiled decision shape, не окончательное разрешение на произвольный resource. Событие policy/membership invalidates. TTL короткий; high-risk mutation может bypass cache.

## 11. Audit

Записываются authentication, decision, denied request, policy change/simulation, export, blob read, bulk action, break-glass. Поля:

- timestamp, tenant, actor/effective actor;
- action/resource/query fingerprint;
- decision/effect/reason IDs;
- policy revision/IDs;
- request/correlation;
- row count/bytes;
- client/service;
- tamper-evident chain/checkpoint.

Audit не хранит password/token и минимизирует PII.

## 12. Delegation и impersonation

Администратор не может молча «войти как пользователь». Support impersonation требует capability, ticket/reason, timebox, prominent UI banner, original/effective principal и полного аудита. Delegation является явным grant с interval/scope.

## 13. Break-glass

- отдельный route;
- step-up MFA;
- reason/ticket;
- минимальный scope/TTL;
- уведомление security owner;
- усиленный audit;
- нельзя отключить tenant isolation или экспортные лимиты;
- автоматическое завершение и review.

## 14. Изменение policies

Draft → validate → simulate → four-eyes approval → publish. Simulation показывает пользователей/queries, потерявших или получивших доступ, без раскрытия данных симулятору. Rollback создаёт новую revision, не стирает историю.

## 15. Fail modes

- PDP unavailable: fail closed; health/readiness отражают состояние.
- Audit unavailable: high-risk mutations/exports fail closed; low-risk reads — по tenant policy.
- Policy stale: high-risk command rechecks; read cursor становится stale.
- Clock uncertainty: token/policy validity консервативна.
