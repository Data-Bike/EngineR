# Универсальная доменная модель

## 1. Базовый тезис

Любые данные представлены ориентированным атрибутированным мультиграфом. Узлы — сущности; рёбра и свойства — утверждения. Типы, поля, связи, формы, правила и переводы также являются сущностями/утверждениями метауровня.

Это не означает «всё в одном JSON»: унифицирована логика, а физическое хранение специализируется.

## 2. Entity

EntityId — стабильный внутренний bigint, PublicId — UUIDv7 для API. Сущность — якорь идентичности, а динамические поля являются statements.

Минимум:

- tenant_id, entity_id, public_id;
- entity_class: data, schema, policy, workflow, system;
- created_at/by;
- retired_at/by/reason;
- head_revision;
- canonical_entity_id после merge.

## 3. Statement

~~~rust
struct Statement {
    id: StatementId,
    tenant_id: TenantId,
    subject: EntityId,
    predicate: PredicateId,
    object: StatementObject,
    validity: ValidTime,
    recorded: RecordedTime,
    provenance: ProvenanceRef,
    confidence: Confidence,
    status: StatementStatus,
    revision: Revision,
}

enum StatementObject {
    Entity(EntityId),
    Scalar(CanonicalScalar),
}
~~~

RecordedTime — системная история. ValidTime — период истинности в предметной области. Отзыв создаёт новую запись/статус, не стирая прошлое.

## 4. PredicateDefinition

Один предикат описывает свойство или связь:

- stable ID и ASCII alias;
- domain/range;
- directed/symmetric/transitive/inverse;
- cardinality и uniqueness scope;
- required/optional и temporal mode;
- участие в identity;
- normalization/validation;
- indexing/search profile;
- confidentiality class;
- labels/help translations;
- presentation hints, не влияющие на смысл.

ИНН — scalar statement. «Является руководителем» — entity statement. Типизация — statement instance_of.

## 5. TypeDefinition

Тип — именованный набор ограничений:

- допустимые/обязательные predicates;
- composition/inheritance;
- identity policies;
- validation rules;
- layouts/search/report profiles;
- lifecycle state machine;
- published schema revision.

Entity может иметь несколько типов: лицо одновременно контрагент, сотрудник, учредитель и бенефициар.

## 6. Метауровень

Закрытый meta-kernel содержит Entity, Statement, TypeDefinition, PredicateDefinition, IDs, revisions, primitive scalar kinds и правила метасхемы. Прикладные типы строятся поверх него. Изменение meta-kernel требует миграции кода/БД; прикладной схемы — нет.

## 7. Scalar kinds

- Unicode text и language-tagged text;
- boolean;
- integer и arbitrary precision decimal;
- date/time/UTC instant/local datetime + IANA zone;
- UUID/opaque ID;
- URI/e-mail/phone;
- money (decimal + ISO 4217);
- measurement (decimal + unit);
- geo point/shape;
- structured JSON только по явной схеме;
- immutable blob reference.

f64 запрещён для денег, долей и детерминированных баллов.

## 8. Provenance

Внешний факт хранит:

- connector/source record;
- fetch/effective timestamps;
- raw payload hash;
- legal basis/license;
- parser/mapping version;
- actor/job/correlation;
- confidence/verification;
- evidence blobs/links.

Источники не перезаписывают друг друга. Resolution policy формирует effective view и сохраняет конфликт.

## 9. ViewDefinition

Определяет root types, sections/fields, related collections, columns/facets, computed values, actions, permission requirements и translations. View не предоставляет доступ: security сначала ограничивает view, затем строится SQL.

## 10. Пакет СКР

Schema bundle skr-core:

- person, legal_entity, holding_view, address, contact_point, document, license;
- request, check, risk_factor, assessment, decision, task, contract, collateral;
- has_tax_id, managed_by, founded_by, owns_share, affiliated_with, has_address, checked_by;
- identity policies для ИНН/ОГРН, документов, телефонов/source IDs;
- workflows, forms, lists, tree и reports.

Ядро не импортирует crate пакета: подписанный bundle загружается Admin API.

## 11. Холдинг

Холдинг — вычисляемая компонента над владением, контролем, управлением и аффилированностью на дату и порог. Именованная holding entity допустима, но состав выражается assertions. Это поддерживает циклическое владение, доли, интервалы, исторический состав и различие юридического контроля/аналитической группы.

## 12. Инварианты

1. Statement содержит ровно один object: entity или scalar.
2. Subject/predicate/entity-object принадлежат tenant, кроме global schema refs.
3. Domain/range проверяются по schema revision команды.
4. Активный statement не меняется семантически in-place.
5. Canonical ID после merge не образует цикл.
6. Projection не является единственным хранилищем.
7. Хранятся canonical form, а при необходимости raw form.
