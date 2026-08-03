# Архитектура системы

## 1. Стиль

Начальная поставка — **модульный монолит** с двумя Rust-приложениями:

- enginer-api: Rocket API, аутентификация, авторизация, синхронные команды/запросы;
- enginer-worker: задания, коннекторы, проекции, отчёты, мониторинг и реакции.

Оба используют общие domain/application crates. Security, query, persistence и connectors имеют независимые порты и могут быть выделены в сервис без изменения DTO.

## 2. Контексты

| Контекст | Ответственность |
|---|---|
| Identity | пользователи, субъекты, сессии, MFA, service accounts |
| Security | policies, группы, решения, obligations, аудит доступа |
| Schema Registry | типы, предикаты, ограничения, ревизии, переводы |
| Entity Graph | сущности, утверждения, версии, merge/split |
| Query | JSON AST, security rewrite, SQL plan |
| Workflow | процессы, задачи, SLA и реакции |
| Connector | источники, квоты, нормализация, provenance |
| Risk | критерии, факторы, оценки и решения |
| Reporting | снимок, шаблоны, генерация и выдача |
| Presentation | layouts, menus и saved views |
| Audit | неизменяемый журнал действий и решений |

## 3. Слои

### Controller

Rocket routes/guards ограничивают HTTP payload, создают RequestContext, десериализуют DTO, вызывают один use case и отображают типизированную ошибку. Здесь нет SQL, бизнес-правил или правил доступа.

### Application

Use cases управляют транзакцией, idempotency ledger, security decision, domain-командами, outbox и ответом. Зависимости — порты (traits).

### Domain

Чистые типы и инварианты: Entity, Statement, PredicateDefinition, SchemaRevision, IdentityPolicy, PolicyDecision, RiskAssessment. Нет зависимости от Rocket/sqlx/Nuxt.

### Persistence

Порты через sqlx: транзакции, query compiler, mapping, locks и проекции. SQL параметризован. Динамические identifiers выбираются из серверного каталога.

## 4. Поток команды CreateEntity

1. Guard проверяет сессию, CSRF/Origin для cookie auth, размер и media type.
2. Controller строит CommandEnvelope: tenant, actor, request ID, idempotency key, locale.
3. PEP проверяет capability entity.create и data permission на target type.
4. Use case загружает опубликованную schema revision.
5. Domain нормализует и валидирует.
6. Identity service вычисляет ключи.
7. Одна транзакция блокирует/создаёт identity key, entity и уникальные statements.
8. Записываются audit и outbox.
9. Результат сохраняется в idempotency ledger.
10. Ответ содержит public ID, revision, ETag и разрешённые поля.

Повтор того же request hash возвращает исходный ответ; иной payload с тем же ключом — 409.

## 5. Поток запроса

1. Принять Query AST или параметры endpoint.
2. Разрешить только опубликованные IDs/операторы.
3. Передать Security PDP структурированный authorization request.
4. Получить allowed types/fields/links, row predicates, masks и limits.
5. Пересечь AST с obligations.
6. Скомпилировать параметризованный SQL и keyset cursor.
7. PostgreSQL возвращает только разрешённое.
8. Response projector повторно применяет field allowlist.
9. Audit фиксирует fingerprint, policy revision, row count и latency.

## 6. Rocket и sqlx runtime

Rocket 0.5 работает на Tokio, заданные features sqlx выбирают async-std. Нельзя использовать block_on в Rocket worker или переносить runtime-зависимые ресурсы.

Адаптер AsyncStdDbExecutor:

- создаёт PgPool внутри async-std runtime;
- принимает typed repository closures через bounded runtime-neutral channel;
- выполняет транзакцию целиком в одном DB job;
- возвращает owned DTO/error через oneshot;
- не переносит Transaction, connection или borrowed row;
- ограничивает очередь, deadline и параллелизм;
- публикует queue/pool/execute metrics.

Порт DbExecutor позволяет позже выровнять runtime отдельным ADR без изменения use cases.

## 7. Согласованность и события

- Каноническая запись и outbox фиксируются в одной транзакции.
- Worker читает outbox через FOR UPDATE SKIP LOCKED.
- Снимки/поиск eventual-consistent.
- API возвращает commit_revision; запрос может указать min_revision.
- Правовые/денежные решения читают канон или проекцию с достаточным watermark.

## 8. Файлы

PostgreSQL хранит metadata/content hash, тело — в S3-compatible или защищённом on-prem storage:

1. upload intent;
2. поток с лимитом/антивирусом;
3. hash/MIME;
4. подтверждение immutable blob;
5. авторизованный короткоживущий download.

## 9. Сетевые границы

- Браузер видит Nuxt и /api/v1.
- /internal/security/v1, worker control и metrics — trusted network + mTLS.
- Nuxt не подключается к БД/источникам.
- Connectors выходят через allowlisted egress.
- PostgreSQL не публикуется наружу.

## 10. Масштабирование

- API stateless.
- Worker масштабируется по очередям и lease/SKIP LOCKED.
- Replica используется только с явной consistency policy.
- Крупный tenant перемещается через TenantLocator на выделенный shard.
- Проекции, поиск и отчёты отделены от transactional hot path.
