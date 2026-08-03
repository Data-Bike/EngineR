# Эволюция схем и преобразование объектов

## 1. Состояния ревизии

draft → validating → impact_ready → approved → publishing → published → retired/failed.

Published revision immutable. Исправление создаёт новую. Одновременно существует один current revision на package/tenant, но API может читать историческую.

## 2. Draft validation

Проверяются:

- aliases/IDs/translation completeness;
- domain/range/cardinality;
- cycles inheritance/composition;
- inverse/symmetric coherence;
- identity normalizers/checksum;
- migration totality;
- index/search profiles;
- security classification;
- form/view references;
- report/workflow dependencies;
- Unicode/confusable aliases;
- cost and storage impact.

## 3. Compatibility classes

- presentation-only: labels/layout/help;
- additive: optional type/predicate/view;
- conditionally compatible: required field with backfill/default;
- semantic: range/cardinality/normalizer/identity;
- breaking: retire/type split/meaning change;
- meta-kernel: code/DDL release required.

API consumers see compatibility report и migration deadline.

## 4. Publish plan

1. Freeze draft content hash.
2. Simulate on production statistics/sample without exposing rows.
3. Build needed indexes/projection generation concurrently.
4. Deploy reader supporting old+new.
5. Backfill deterministic mappings in resumable chunks.
6. Quarantine invalid rows.
7. Verify counts/checksums/invariants.
8. Activate revision/projection atomically.
9. Emit SchemaPublished.
10. Retire old write path after observation window.

## 5. Expand/contract

Нельзя одновременно удалить старый predicate и потребовать новый. Сначала add/dual-read or mapping, затем backfill, switch writes, verify, finally retire old presentation. Canonical source остаётся lineage-accessible.

## 6. MigrationPlan

Содержит:

- source/target revisions;
- ordered primitive steps;
- deterministic function/normalizer versions;
- preconditions/postconditions;
- conflict/quarantine policy;
- chunk key/size;
- retry/idempotency key;
- expected counts;
- reverse/compensation;
- index/projection dependencies;
- owner/approval.

Никакого user-provided executable code. Functions выбираются из versioned safe registry/DSL.

## 7. Required field

Допустимые стратегии:

- deterministic default from existing facts;
- connector enrichment with review;
- manual task before activation;
- required_for_new_only with legacy exception marker;
- quarantine/read-only until repaired.

Фиктивное значение «неизвестно» запрещено, если это искажает смысл. Unknown моделируется отсутствием или explicit semantic value по schema.

## 8. Scalar conversion

Converter возвращает converted, ambiguous, invalid или overflow. Raw сохраняется. Только converted публикуется в target predicate. Остальные создают migration issue с row/entity context, не останавливая весь resumable job, если policy допускает.

## 9. Identity normalizer change

1. Вычислить новые keys в shadow table.
2. Найти collisions/splits.
3. Классифицировать auto-safe/manual/blocking.
4. Review.
5. Атомарно переключить policy version.
6. Сохранить старые keys как aliases/lineage.
7. Не выполнять silent auto-merge.

## 10. Split/Merge predicate

Split сохраняет исходный statement, создаёт target statements и mapping. Merge хранит источники и precedence. Если несколько значений конфликтуют, effective view помечает conflict вместо выбора по последней записи.

## 11. Reify edge

Связь с атрибутами преобразуется:

A —owns→ B

в relation entity R с A —owner_role→ R, R —owned_role→ B и facts доля/период/source. R хранит link_to_original_statement. Reverse возможен, если target edge способен выразить qualifiers; иначе qualifiers residual.

## 12. Rollback

Schema rollback — публикация компенсирующей revision и переключение active projection generation. Нельзя удалять уже записанные facts. Если внешние side effects произошли, применяется compensation workflow, а не DB rollback illusion.

## 13. Concurrent writes

Во время backfill новые команды валидируются по current revision и записывают event. Dual projector применяет их к building generation. Cutover возможен, когда source watermark и catch-up равны.

## 14. API migration

DTO v1 сохраняется до declared deprecation. Server mapping переводит aliases/old predicate IDs через compatibility registry. Deprecation headers и usage telemetry определяют removal. Stable IDs предпочтительнее versioned field names.

## 15. Acceptance

- round-trip preserves canonical/residual facts;
- repeated migration no duplicates;
- resume from every chunk boundary;
- concurrent write visible after cutover;
- invalid data reported, not silently coerced;
- old API works in promised window;
- security policies map to target predicates before activation.
