# Каталоги: второй уровень изоляции данных

> Статус: архитектурная спека. Реализация — отдельными фазами (§11).

## 1. Понятие

**Каталог** — именованная область данных внутри тенанта, объединяющая
объекты, их типы, предикаты, схемы и права. Примеры: «Склад», «HR»,
«Безопасность», «Финансы» — плюс произвольные пользовательские.

Инварианты:

- Каталог — **не сущность графа** и не тенант. Это первоклассная строка
  `core.catalog`; графовые сущности на каталог не ссылаются (кроме как
  значениями полей — для них ссылочной семантики нет).
- **Каждая сущность принадлежит ровно одному каталогу** на всю жизнь
  (`entity.catalog_id NOT NULL`). Пользователи (`sec.principal`) сущностями
  не являются и каталогу не принадлежат — это единственное исключение.
- Перенос сущности между каталогами в v1 **запрещён**: смена домена =
  retire + recreate в целевом каталоге (иначе переписывается смысл
  statement'ов, посчитанных под другой схемой).
- Типы/предикаты **каталого-зависимы**: alias `person` в «HR» и `person`
  в «Складе» — разные `type_id`, с разными полями, рёбрами и правами.
  Одноимённость — только удобство человека, не связанность данных.
- Связи (statement edge) **могут** пересекать границы каталогов
  (решение по §3): предикат принадлежит каталогу субъекта ребра, объект
  ребра — любому каталогу тенанта.

## 2. Модель данных

```sql
CREATE TABLE core.catalog (
    tenant_id    uuid NOT NULL REFERENCES core.tenant(id),
    catalog_id   bigint NOT NULL,           -- seq per tenant
    public_id    uuid NOT NULL,             -- uuidv7, tenant-unique
    alias        text NOT NULL,             -- ascii slug: 'wh', 'hr', 'sec'
    name         text NOT NULL,             -- отображаемое: 'Склад'
    description  text NOT NULL DEFAULT '',
    status       text NOT NULL DEFAULT 'active',  -- active|retired
    settings     jsonb NOT NULL DEFAULT '{}',
    created_at   timestamptz NOT NULL DEFAULT now(),
    created_by   bigint,
    retired_at   timestamptz,
    PRIMARY KEY (tenant_id, catalog_id),
    UNIQUE (tenant_id, public_id),
    UNIQUE (tenant_id, alias)
);
```

Колонки `catalog_id uuid NOT NULL REFERENCES` (логически — по public_id
или суррогатному id; выбранное решение зафиксировать в миграции):

| Таблица | Изменение | Семантика |
|---|---|---|
| `core.entity` | `+ catalog_id` | владелец сущности; NOT NULL, immutable |
| `core.statement` | `+ catalog_id` (денорм.) | = `subject.catalog_id`; индекс для фильтрации |
| `meta.type_definition` | `+ catalog_id` | тип принадлежит каталогу |
| `meta.predicate_definition` | `+ catalog_id` | предикат принадлежит каталогу |
| `meta.schema_revision` | `+ catalog_id` | ревизия каталога (§3) |
| `meta.identity_policy` | через тип | политика привязана к типу → каталогу |
| `meta.workflow_definition` | `+ catalog_id` | процессы каталожны |
| `audit.event` | `+ catalog_id NULL` | каталожный срез аудита; NULL = системное |

RLS остаётся тенантным; каталог — **прикладной** уровень изоляции через
`DataPerm` (как `row_condition`/field rules сегодня). Усиление до
`app.catalog_ids` GUC — отдельная задача поверх.

Индексы: `(tenant_id, catalog_id)` на entity/statement/type/predicate;
`UNIQUE (tenant_id, catalog_id) WHERE state='published'` на
schema_revision вместо текущего per-tenant.

## 3. Схема на каталог

У каждого каталога **свой поток ревизий**: draft → … → published,
нумерация и контент независимы (решение: «ревизия на каждый каталог»).

- `meta.schema_revision.catalog_id` — владелец; unique published per
  (tenant, catalog).
- Все type/predicate/identity-policy/workflow определения внутри ревизии
  принадлежат этому каталогу; `stable_alias` уникален **внутри каталога**
  (`UNIQUE (tenant_id, catalog_id, stable_alias)`).
- Одинаковый alias в разных каталогах — разные определения: у «ФЛ» в HR
  могут быть поля, которых нет у «ФЛ» в Складе, и наоборот.
- Draft/publish API принимает `?catalog=`; валидация, impact, publish —
  всё в контексте ревизии этого каталога.
- Пакеты (`schema-packages/*`) могут быть применены к любому каталогу;
  типовой сценарий — seed нового каталога из пакета + точечная доработка.
- **Системный каталог** `system` (alias `sys`, создаётся миграцией,
  неретайримый): владеет типами, обслуживающими саму платформу —
  person/principal-справочник, структурные сущности. Обычные пользователи
  в него не пишут; его published-ревизия — каркас домена.
- Каталожные типы могут ссылаться domain/range на типы других каталогов —
  range проверяется по `type_id` (глобально уникальному), не по alias.

## 4. Права

Новый resource-селектор в `sec.policy_rule.resource`:

```json
{"catalog": "wh"}            // конкретный каталог (alias или public_id)
{"catalog": "*"}             // все каталоги (административное право)
{"catalog": "hr", "admin": true}  // администрирование каталога
```

`DataPerm` пополняется `catalog_rules` (селектор → PermSet) и
каталожным измерением в type/field/relation-ключах. Реализованный
порядок проверки при доступе к данным:

1. **Catalog gate** — `can_catalog([public_id, alias], op)` вызывается
   до любой type/field-проверки. Нет — объекты каталога невидимы:
   list/query не возвращают их, прямой GET отвечает 404 (существование
   не раскрывается), явный `?catalog=` на list-ручках отвечает 403.
   Это реализует «нет прав на каталог = нет поиска, просмотра и
   изменения объектов каталога».
2. Type/field/relation правила — как сейчас; все селекторы могут
   сужаться `catalog` в `resource` (`{"type":"wh_stock","catalog":"wh"}`).
   Без `catalog` правило действует во всех доступных каталогах.
3. `row_condition`, маски — без изменений, внутри своего каталога.

Deny-by-default: нет подходящего catalog-правила ⇒ нет доступа ни к
одному каталожному объекту. `data.*`/`data.all.*` (wildcard) открывает
каталоги для обратной совместимости, но явный
`deny data.catalog.<op>` всегда сильнее wildcard-разрешения.
Системные операции (логин, `/me/context`, health) не каталожны и не
блокируются.

Capability: `catalog.manage` (`CapabilityAction::CatalogManage`) —
мутации `/api/v1/catalogs` (create/update/retire); `security.manage`
принимается как umbrella. Чтение каталога покрывается `entity.read` +
catalog gate. Действие `data.catalog.<op>` в `sec.policy_rule` — само
право на каталог; селектор `{"catalog": alias|public_id|"*"}`.
Миграция раздаёт `{"catalog":"*"}` только data-классовым группам —
capability-группа с грантом каталога «палит» реестр каталогов
(имена в `/me/context` и `/catalogs`) всем членам.

Меню выбора: `/me/context` возвращает `catalogs: [{public_id, alias,
name}]`; UI с >1 доступным каталогом показывает селектор (выбор
персистится в localStorage, `catalogParams()` подставляет `?catalog=`);
с 0 — рабочий стол без данных.

## 5. Кросс-каталожные связи

Рёбра между каталогами **разрешены** (решение по запросу). Модель:

- Statement (включая edge) принадлежит каталогу **субъекта** —
  `statement.catalog_id = subject.catalog_id`. Создание требует прав на
  каталог субъекта + `relation` permission на предикат.
- Объект ребра — `public_id` любой сущности тенанта; резолюция не
  проверяет каталог объекта.
- **Резолюция предиката при записи** — catalog-first с tenant-fallback:
  предикат ищется в реестре каталога субъекта; при отсутствии —
  tenant-wide (детерминированно: меньший `catalog_id`, затем меньший
  `predicate_id`). Это позволяет субъектам одного каталога использовать
  общие скаляры (`name`, `inn`) и рёбра (`wh_keeper` на `person` в
  `general`), не дублируя определения. Алиасы типов fallback'а **не**
  имеют — тип всегда каталожен.
- **Чтение** ребра видимо, если виден и предикат (каталог субъекта), и
  объект (каталог объекта): пользователь без прав на чужой каталог не
  видит ни сам объект, ни ребра к нему — `relations.list` фильтрует по
  `allowed_catalogs` на обоих концах. Промежуточная форма: ребро видно,
  но объект отдаётся как `{public_id, catalog, display_name:"—"}` без
  полей; v1 выбирает полное скрытие (проще, строже).
- Индексация/поиск ребёр — в контексте каталога субъекта.

## 6. Контекст запроса `?catalog=`

- Значение: `alias` или `public_id` каталога; резолвится в `catalog_id`.
- `entities.create`: необязателен — без него каталог берётся из каталога
  самого типа (типы каталожны и однозначны); с параметром — тип ищется в
  ревизии этого каталога и `entity.catalog_id` ставится явно.
- `entities.get`: необязателен; параметр, не совпадающий с каталогом
  сущности, отвечает 404 — каталожная принадлежность не раскрывается.
- `entities.update/retire/merge`, `relations.*`, `entities.history`:
  необязателен — каталог берётся из сущности; без чтения каталога
  сущности — 404 (существование не раскрывается); при читаемом, но
  отличающемся `?catalog=` — 409 `catalog.mismatch`; без права записи —
  403.
- `entities.list`, `entities.query`, поиск: необязательный фильтр; без
  параметра — объединение доступных каталогов; с параметром — один
  каталог (403 `catalog.denied`, если недоступен).
- `schema.*` (draft/validate/publish/impact, types, predicates, view):
  `catalog` выбирает поток ревизий; без него листинги объединяют все
  читаемые каталоги (каждая запись несёт `catalog`), чтение типа — первый
  доступный каталог с таким alias.
- Несуществующий/retired каталог → 422 `catalog.unknown`; создание с
  дублирующим alias → 409; CRUD-модель retire непустого каталога → 409
  `catalog.not_empty`.

## 7. API

```
GET    /api/v1/catalogs                    → список каталогов, читаемых caller'ом
POST   /api/v1/catalogs                    → create {alias,name,description?,settings?}
GET    /api/v1/catalogs/{ref}              → карточка + entity_count/type_count
PUT    /api/v1/catalogs/{ref}              → name/description/settings/status
POST   /api/v1/catalogs/{ref}/retire       → retire (409 пока есть живые объекты)
```

`{ref}` = alias или public_id. Мутации требуют `catalog.manage`
(umbrella — `security.manage`), чтение — `entity.read` + catalog gate.

`/me/context.catalogs` — доступные каталоги с флагами прав; при 2+ UI
показывает селектор.

Изменённые подписи: `pg_create_entity(?catalog=…)`, `pg_query_entities/
list(?catalog=…)`, schema-admin routes (`?catalog=`), `load_type_def`
/`published_schema_revision` принимают `catalog_id`. Identity/dedup —
внутри каталога (политика принадлежит типу → каталогу), повторный код в
другом каталоге создаёт **другую** сущность.

## 8. MCP

- Все entity-инструменты получают необязательный аргумент `catalog`
  (шим кладёт его в query `?catalog=`); `entities.create` — необязателен,
  как и в REST (тип сам задаёт каталог).
- `catalogs.list/create/get/update/retire` — обычные инструменты из API
  spec; схема-инструменты принимают `catalog`.
- Подсказка агентам в описании инструмента: «catalog = область данных;
  типы и права каталого-зависимы».

## 9. Запросы, поиск, аудит

- `entities.list`/query: `AND catalog_id = $n` (или `IN (allowed)`);
  индекс `(tenant_id, catalog_id, type_id)` на entity.
- Полнотекст/автокомплит (`ops.entity_search`) — `catalog_id` в
  projection + фильтр.
- Аудит: `audit.event.catalog_id` (NULL-able — внекаталожные операции
  его не имеют) + индекс; `audit.query` фильтрует события по видимости
  каталога, а `?catalog=<ref>` — явный срез одного каталога.
- Outbox/jobs: `job.catalog_id` — отложено (v2): рабочие задачи пока
  тенант-глобальны.

## 10. Миграция существующих данных

1. `core.catalog` + seq (per tenant); каталоги `general` («Основной»),
   `sys` («Системный»), `wh` («Склад»).
2. `+catalog_id` колонки NULL → backfill на entity/statement/
   type/predicate/schema_revision → NOT NULL; unique-индексы alias и
   published-ревизии расширяются `catalog_id` (ревизия уникальна внутри
   `(tenant, catalog)`).
3. Раскладка по каталогу **типа**: типы с `wh_*` alias → `wh`, прочие →
   `general`; сущности следуют каталогу своего `type_id`, statement'ы —
   субъекту; published-ревизия со склада → `wh`, остальные → `general`.
   Разовая миграция владения — не смена домена.
4. Policy rules: текущим **data-классовым** группам выдаётся
   `data.catalog.* {"catalog":"*"}` — поведение не деградирует;
   capability-группы гранта не получают (ADR-010: классы не смешиваются,
   иначе реестр каталогов утекает всем членам).
5. `core.catalog` получает ENABLE+FORCE RLS **после** backfill —
   строковые вставки внутри миграции идут с установленным
   `app.tenant_id`, иначе FORCE-политика их отклонит.

## 11. План реализации

| Фаза | Содержание |
|---|---|
| 1. DDL | `core.catalog`, колонки `catalog_id`, backfill, индексы, миграция `2026xxxx0000xx_core_catalog.sql` |
| 2. Домен | `Catalog` aggregate + `CatalogId`, `crates/domain`; каталог-контекст в `RequestContext` |
| 3. Schema-registry | ревизии/драфты per catalog, alias scope per catalog, load_type_def/published_revision(catalog) |
| 4. Права | `{"catalog":…}` в resource, `allowed_catalogs` в `DataPerm`, gate на entity/relation/type, фильтрация списков |
| 5. API | `/api/catalogs` CRUD, `?catalog=` на entity/schema routes, `/me/context.catalogs` |
| 6. MCP | аргумент `catalog`, catalogs.* tools |
| 7. Данные | seed-каталоги, перенос `wh_*` в «Склад», policy rules `*` |
| 8. Веб/Nuxt | типы `CatalogRecord`, селектор каталога, страница админки каталогов |
| 9. Тесты | изоляция каталогов (нет прав → 403/невидимо), cross-catalog edges, schema per catalog, alias collision, dedup per catalog, CRUD lifecycle |
| 10. Документы | правки в docs/02 (уровни изоляции), 03 (каталог в метамодели), 06 (контракт), 08 (права), 10 (DDL), 16 (UI), AGENTS.md |

## 12. Открытые вопросы

- **Shared-справочник**: нужен ли позже «Общий» каталог с чтением всем?
  (юрлица, справочники) — v1 решает через выдачу `read` на каталог группам.
- **Иерархия каталогов**: не вводится (плоский список); при запросе —
  `settings.parent_catalog_id`.
- **GUC-изоляция**: `app.catalog_ids` в RLS — отдельная задача упрочнения.
- **Statement.catalog_id при кросс-ребре**: хранится каталог субъекта;
  «срез данных каталога» = statements своих сущностей + входящие ребра из
  других каталогов (видимость по правам на объект).
- **Retire непустого каталога**: запрещён до retire всех активных
  сущностей (безопаснее каскада).
