# Производительность и масштабирование

## 1. Подход

«Сотни миллионов» — не магическое свойство схемы, а проверяемый профиль. Для каждого релиза задаются dataset generator, queries, SLO, hardware, concurrency и explain plans. Оптимизация не нарушает универсальный канон: добавляются/заменяются проекции и индексы.

## 2. Профили нагрузки

| Профиль | Объём | Назначение |
|---|---:|---|
| dev | 100 тыс. statements | функциональная разработка |
| integration | 10 млн | regressions и migration |
| scale | 100 млн active + history | release benchmark |
| extreme | 500 млн+ | partition/shard validation |

Генератор воспроизводит skew: популярные типы, high-degree nodes, multi-valued fields, длинные тексты, tenant imbalance, history.

## 3. SLO измерения

Раздельные гистограммы:

- exact identity lookup;
- GET summary/card;
- filtered list first/next page;
- full-text;
- relation list;
- graph depth 1/2/3;
- create/update;
- policy decision/rewrite;
- job enqueue;
- projection lag.

P50/P95/P99, throughput, error/timeout, rows examined/returned, DB/queue/pool wait обязательны.

## 4. Connection и runtime budgets

- Rocket workers не равны DB connections.
- DbExecutor bounded queue предотвращает stampede.
- Pool size вычисляется из DB max connections, replicas и service instances.
- Acquire timeout короче request deadline.
- Долгий batch использует отдельный pool/workload class.
- Нельзя держать connection во время внешнего HTTP, render или ожидания пользователя.
- Cancellation Rocket propagates в DB job; statement_timeout устанавливается transaction-local.

## 5. Query discipline

- keyset pagination;
- selective type/tenant filter до EAV-like joins;
- EXISTS/LATERAL вместо exploding joins;
- projection только запрошенных predicates;
- limit на fan-out;
- precomputed summary для списков;
- prepared/statements where safe;
- bounded IN; большие scopes через temp/server tables;
- никаких SELECT *;
- никаких functions на indexed column без matching expression index;
- plan regression snapshots для critical queries.

## 6. Точечные и малые выборки

Exact identity: identity_key unique lookup → entity summary/current statements. Public ID: tenant+public UUID unique. Card loads predicates batches by storage family, не выполняет N+1. Related counts загружаются агрегированно/лениво.

## 7. Большие выборки

Interactive API никогда не возвращает миллионы строк. Export job:

1. фиксирует query/schema/policy snapshot;
2. использует server-side/keyset chunks;
3. записывает streaming file;
4. проверяет policy на каждом chunk или bound snapshot policy;
5. ограничивает bytes/time;
6. публикует artifact;
7. не держит одну огромную transaction, если snapshot policy этого не требует.

Для repeatable snapshot PostgreSQL exported snapshot применяется только контролируемо и с max age, чтобы не удерживать VACUUM.

## 8. Full-text и locale

Отдельный entity_search на locale/search profile. Search config выбирается по данным/tenant. Exact identifiers идут через B-tree, не full-text. Trigram включается только для полей с доказанным fuzzy use case. Sensitive text не смешивается с общим search vector.

## 9. Граф

- depth 1 adjacency B-tree;
- типовые иерархии closure projection;
- arbitrary traversal recursive CTE с budget;
- high-degree nodes возвращаются paged;
- counts могут быть projection;
- graph cache key включает entity revision, predicate set, as_of, policy shape;
- запрет traversal применяется на каждом шаге.

## 10. Партиции

Hash partition помогает parallelism/maintenance и локализует indexes, но не заменяет tenant filter. Количество выбирается benchmark. Audit/events range partitions дают pruning/retention.

Правила:

- partition key присутствует в unique constraints;
- запросы обязаны содержать tenant;
- default partition мониторится и обычно запрещена;
- будущие partitions создаются заранее;
- detach/archive runbook проверен;
- слишком мелкие partitions запрещены.

## 11. Проекции

Projection lifecycle:

draft → building generation N → verified → active → retiring old generation.

Verification:

- row/count/checksum by bucket;
- source watermark;
- sampled canonical comparison;
- security classification check;
- plan benchmark.

Write path публикует event; critical summary может обновляться transactionally. Для read-after-write API умеет ждать watermark или читать canonical fallback.

## 12. Cache

- L1 bounded per process: schema/policy definition, immutable by revision.
- Distributed cache optional: summaries/safe lookup.
- Cache key всегда tenant + resource revision + locale + policy shape.
- Negative cache короткий и не переносится между subjects.
- Секреты/полные PII responses не кешируются без отдельной модели.
- Invalidation event-based; TTL — предохранитель.
- Cache stampede: singleflight/jitter.
- Существующий самодельный LFU не используется до доказательства корректности/concurrency.

## 13. PostgreSQL maintenance

- autovacuum settings per high-write table;
- fillfactor только по измерению;
- monitor bloat, dead tuples, freeze age, WAL, checkpoints, replica lag;
- ANALYZE/statistics target для skewed predicate/type columns;
- extended statistics на correlated columns;
- REINDEX CONCURRENTLY/partition maintenance runbooks;
- pg_stat_statements и normalized query fingerprints;
- no long idle-in-transaction.

## 14. Write amplification

Каждый новый index/projection оценивается по insert latency/WAL/storage. Fact canonical tables не получают все возможные indexes. IndexProfile прикрепляется к predicates и активируется online job. Неиспользуемый index удаляется после review.

## 15. Sharding

До sharding применяются partitions, replicas, проекции и hardware. При необходимости TenantLocator выбирает database shard; tenant не разрезается между shards на первом этапе. Global schema bundles реплицируются. Cross-tenant analytics идут в отдельный authorized warehouse, не distributed OLTP join.

## 16. Backpressure

- request/bulk rate limits per tenant/principal;
- bounded DbExecutor/worker queues;
- connector concurrency/quotas;
- 429 для quota, 503 для saturation;
- adaptive disable тяжёлых optional features;
- circuit breakers на источники;
- UI показывает queued/degraded status.

## 17. Benchmark gates

Release блокируется, если:

- critical P95 регрессировал >10% без ADR;
- query делает sequential scan большой family без одобрения;
- rows examined/returned выросли кратно;
- policy filtering выполнен post-fetch;
- projection lag превышает SLO;
- migration lock превышает budget;
- load test обнаружил duplicates/tenant leak.
