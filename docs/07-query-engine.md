# Язык запросов и безопасная компиляция

## 1. Цель

Query engine даёт универсальные выборки без произвольного SQL. Пользователь задаёт JSON AST по стабильным type/predicate IDs. Сервер валидирует, применяет security obligations, оценивает стоимость и строит параметризованный SQL.

## 2. AST

~~~json
{
  "from": {"type_id": "legal-entity-uuid", "alias": "e"},
  "select": [
    {"field": "name-predicate-uuid", "as": "name"},
    {"field": "tax-id-predicate-uuid", "as": "tax_id"}
  ],
  "where": {
    "and": [
      {"field": "tax-id-predicate-uuid", "op": "eq", "value": "7700000000"},
      {"field": "status-predicate-uuid", "op": "in", "values": ["active"]}
    ]
  },
  "sort": [
    {"field": "name-predicate-uuid", "direction": "asc"},
    {"system": "public_id", "direction": "asc"}
  ],
  "page": {"size": 50, "after": null},
  "as_of": "2026-08-03T00:00:00Z"
}
~~~

Поддерживаются from, select, where, exists/related, sort, aggregates, group_by, facets, as_of и keyset page. Join задаётся relation path, не именем таблицы.

## 3. Operators

Операторы разрешаются по scalar kind:

- eq/ne/in/is_null;
- lt/lte/gt/gte/between;
- contains/prefix/full_text;
- date overlap/valid_at;
- relation exists/not_exists;
- geo within/distance при подключённом profile.

Regex, script, SQL fragment и uncontrolled wildcard запрещены. Contains на больших text без подходящего index может быть отклонён cost guard.

## 4. Pipeline

Порядок неизменен:

1. parse + syntactic limits;
2. resolve schema revision;
3. semantic type-check;
4. normalize literals;
5. request capability check;
6. PDP decision;
7. security AST rewrite;
8. projection/index selection;
9. cost estimate;
10. parameterized SQL compile;
11. execution deadline/cancellation;
12. response allowlist;
13. audit.

Нельзя компилировать пользовательский AST, а затем приклеивать security WHERE строкой.

## 5. Security rewrite

PDP возвращает:

- allowed root types;
- allowed select predicates;
- allowed relation predicates/directions;
- mandatory row predicates;
- masks;
- max page/depth/export;
- forbidden aggregates/facets.

Rewriter:

- пересекает select;
- добавляет mandatory filters в корень и каждый subquery;
- запрещает order/facet по скрытому полю, чтобы не дать side channel;
- удаляет запрещённые edges;
- ограничивает node summaries;
- маркирует plan policy revision.

Если root type полностью запрещён, запрос компилируется в false/no rows или возвращает 404 по endpoint policy.

## 6. SQL compiler

- identifiers берутся только из trusted catalog;
- values всегда bind parameters;
- предикаты группируются по физическим typed families;
- current statements используют partial index/status;
- temporal query добавляет valid_from/valid_to;
- joins имеют tenant_id в условии;
- type membership фильтруется до широких joins;
- duplicate rows устраняются предсказуемым EXISTS/LATERAL, а не случайным DISTINCT;
- query tag содержит request/plan hash, не user text.

SQLx dynamic QueryBuilder допускается только внутри persistence adapter с bind. Compile-time macros используются для статических queries.

## 7. Keyset pagination

Sort всегда завершается уникальным public_id/entity_id tie-breaker. Cursor содержит:

- query/schema/policy fingerprints;
- normalized last sort values;
- direction;
- tenant;
- issued/expiry.

Cursor подписан/зашифрован, не принимает изменения клиента. Null ordering фиксирован. Изменение policy/schema делает cursor недействительным с 409 cursor_stale.

## 8. Cost control

До выполнения считаются:

- число predicates/paths;
- максимальная relation depth;
- наличие selective filters;
- подходящий index/projection;
- ожидаемый fan-out;
- aggregate cardinality;
- requested page/export size.

Plan class: interactive, extended, batch-only, rejected. Batch-only создаёт job. EXPLAIN без ANALYZE может использоваться сервером для approved templates, но не на каждый запрос.

## 9. Graph traversal

Recursive CTE разрешён только с:

- max_depth/max_nodes;
- predicate allowlist;
- tenant + temporal + security filters на каждом шаге;
- visited path для cycle detection;
- deterministic ordering;
- statement/edge cursor;
- timeout.

Для частых иерархий используется closure projection с revision/watermark. Произвольный граф не заставляется быть деревом.

## 10. Aggregates и утечки

COUNT/SUM/facets могут раскрывать наличие скрытых данных. Поэтому aggregate permission отдельна; filters/select/group fields должны быть разрешены. Для чувствительных классов PDP может потребовать minimum group size, округление или запрет.

## 11. Saved queries/views

Saved query хранит AST, owner/scope, schema revision, migrations, presentation config. При выполнении всегда применяется текущая policy. Сохранение query не сохраняет доступ.

## 12. Plan cache

Ключ: tenant, normalized AST hash, schema revision, policy shape hash, projection catalog revision. Значения данных/subject-specific row filter не попадают в небезопасно общий cache. Invalidation — публикация схемы/policy/projection.

## 13. Ошибки

Стабильные codes:

- query.syntax_invalid;
- query.unknown_type/predicate;
- query.operator_not_supported;
- query.field_forbidden;
- query.too_complex;
- query.requires_batch;
- query.cursor_invalid/stale;
- query.projection_lag;
- query.timeout.

## 14. Тесты

- AST fuzzing не вызывает panic/unbounded allocation;
- SQL snapshots не содержат literal user input;
- каждый join содержит tenant/security constraints;
- hidden fields нельзя использовать в select/sort/filter/facet;
- keyset не пропускает/дублирует строки при стабильном snapshot;
- graph cycles завершаются;
- эквивалентные AST дают одинаковый fingerprint.
