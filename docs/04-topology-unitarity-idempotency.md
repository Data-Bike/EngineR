# Топологичность, унитарность и идемпотентность

## 1. Определения

**Унитарность:** один минимальный логический конструктор — statement «subject — predicate → object». Свойство отличается от связи только видом object.

**Топологичность:** между любыми опубликованными схемами существует конечный путь преобразований без уничтожения канонических фактов. Невыразимые в цели данные сохраняются как residual facts.

Без residual storage безусловное «любой объект в любой без потерь» невозможно: один boolean не вмещает произвольную карточку. Гарантируется сохранение полной информации в каноне и обратимость lineage.

**Идемпотентность:** f(f(x)) = f(x) по наблюдаемому состоянию. Она обеспечивается для команд, сущностей/facts, workers/imports и schema/projection migrations.

## 2. Доказательство унитарности

Пусть E — entities, P — predicates, V — дизъюнктное объединение E и canonical scalars. Данные — конечное S ⊆ E × P × V × M, где M — время, provenance/status.

- Поле: E × P × Scalar.
- Связь: E × P × E.
- Тип: E × instance_of × E_type.
- Поле схемы: statements о predicate entity.
- Холдинг: подграф или entity с membership/ownership.

Любой объект строится композицией одного атома; object_kind даёт обратное отображение.

## 3. Примитивы преобразования

1. AddType, RetireType, AliasType.
2. AddPredicate, RetirePredicate, RenameLabel.
3. AddMembership/RemoveMembership как temporal statements.
4. MapPredicate.
5. SplitPredicate с residual.
6. MergePredicates с lineage.
7. ConvertScalar с total parser или quarantine.
8. ReifyEdge — связь в relation entity.
9. CollapseRelationEntity с residual.
10. MergeEntity/SplitEntity.
11. MaterializeDerived/DiscardProjection.

Каждый шаг имеет forward, optional reverse, pre/postconditions, mapping version и отчёт.

## 4. Конструктивное доказательство топологичности

Для объекта A и целевого типа B:

1. Представить A конечным графом G_A.
2. Назначить membership B, не удаляя A до завершения.
3. Для required predicates B применить mapping/default/manual task.
4. Для неотображаемых statements записать migration_residual; исходник остаётся доступен.
5. Провалидировать B.
6. Атомарно опубликовать entity revision.
7. Сохранить bijection созданных statements и residual refs.

Reverse использует lineage и исходники. Значит, информация не уничтожается, меняется активная проекция. Композиция примитивов даёт путь между схемами.

## 5. Rename

Alias/labels меняются, type_id/predicate_id — нет. Переименование ИНН в Tax identifier не мигрирует facts/indexes. Display name запрещено использовать как DB/API key.

## 6. IdentityPolicy

Содержит scope, key components, normalizer, quality, exact/fuzzy mode, collision action и resurrection policy.

Exact-примеры:

- ЮЛ: country + registration number type + ОГРН; альтернативно country + проверенный ИНН;
- телефон: E.164 + extension;
- документ: country + document type + series/number;
- внешний объект: connector + namespace + source ID.

Имя, дата рождения и адрес по умолчанию дают same_as_candidate, не auto-merge.

## 7. Нормализация

Хранятся raw_value, canonical_value и normalizer_version.

- текст: UTF-8 NFC и field-specific trim; регистр сохраняется;
- search key: отдельный casefold;
- ИНН/ОГРН: digits/length/checksum;
- телефон: E.164, extension отдельно;
- e-mail: IDNA domain, заданная политика local part;
- URL: normalized scheme/host/port;
- decimal: canonical scale;
- instant: UTC + исходная IANA zone;
- symmetric edge: sorted endpoints и canonical predicate.

Новый normalizer запускает migration с collision report.

## 8. Fact fingerprint

~~~text
tenant | canonical_subject | predicate | object_kind | canonical_object |
valid_from | valid_to | qualifier_hash | identity_scope
~~~

Dedupe scope:

- global_fact: одинаковый смысл, несколько provenances;
- per_source: отдельное assertion на источник;
- per_observation: каждое наблюдение.

BLAKE2b-256 ускоряет поиск, но collision проверяется полными canonical columns/bytes.

## 9. Idempotency-Key

Для изменяющих endpoint:

- uniqueness: tenant + principal + route + key;
- request hash, status, response, revision, expiry;
- concurrent repeats синхронизируются unique index/lock;
- тот же payload возвращает исходный response;
- иной payload — 409;
- долгий вызов возвращает один job ID.

## 10. Конкурентный телефон

Два варианта номера нормализуются одинаково. INSERT identity_key ON CONFLICT допускает одного owner; второй читает его ID. Statements upsert по fingerprint. Оба ответа указывают один public ID, второй имеет deduplicated=true. SELECT-before-INSERT без unique constraint запрещён.

## 11. Merge/Split

Merge выбирает canonical entity, делает loser IDs aliases, разрешает statements через canonical ID, сохраняет конфликты, переносит keys и пишет EntityMerged mapping. Повтор той же пары — no-op.

Split создаёт entities, распределяет statements/keys по plan, сохраняет исходный lineage anchor и rebuilds projections. Повтор plan ID — no-op.

## 12. Worker

Inbox uniqueness: consumer + event_id + handler_version. Внешний запрос получает stable remote idempotency key; иначе используется request fingerprint/reconciliation. E-mail/file delivery имеет отдельный ledger.

## 13. Проверяемые свойства

- normalize(normalize(x)) == normalize(x);
- повтор CreateEntity возвращает тот же ID;
- порядок endpoints symmetric edge не меняет ID;
- forward + reverse восстанавливает source facts;
- двойной projection rebuild даёт один checksum;
- повтор merge не создаёт событие;
- retry после commit-before-response возвращает сохранённое;
- residual fact не становится недостижимым.
