# Спецификация пользовательских экранов

## 1. Информационная архитектура

Меню формируется из schema packages + system capabilities:

- Главная;
- Объекты;
- Заявки и процессы;
- Проверки;
- Мониторинг;
- Риски и решения;
- Отчёты;
- Документация;
- Администрирование;
- Интеграции;
- Аудит (при наличии права).

Пункты «ФЛ», «ЮЛ», «холдинг», «документ», «адрес», «телефон», «банк», «транспорт» из legacy-экрана становятся saved views/types пакета, а не статическим кодом sidebar.

## 2. Главная / dashboard

Карточки:

- мои задачи и просроченные;
- новые заявки;
- проверки в работе/ошибке;
- выявленные критические факторы;
- monitors due;
- готовые отчёты/jobs;
- статистика по разрешённой области;
- health внешних источников, если разрешено.

Каждая цифра безопасна против aggregate leak. Виджеты настраиваются, имеют as_of/freshness и drill-down.

## 3. Каталог объектов

Route: /objects/[typeAlias].

- page title/description;
- saved view selector;
- simple/advanced search;
- filters/facets;
- columns/sort;
- cursor table;
- bulk selection limited server-side selection token;
- create/import/export actions по capabilities;
- row status/source freshness;
- empty/loading/error states.

Bulk selection «все результаты» хранится query token, не миллионы IDs в браузере.

## 4. Карточка entity

Route: /entities/[publicId].

Header:

- локализованное имя и type badges;
- lifecycle/risk/classification status;
- IDs с copy по permission;
- source freshness/conflict marker;
- actions edit/check/report/merge/retire;
- revision/as_of.

Tabs:

1. **Обзор** — view sections/fields.
2. **Связи** — grouped related objects.
3. **Граф/дерево** — progressive traversal.
4. **Проверки** — runs/sources/results.
5. **Риски** — factors/assessment/decisions.
6. **Процессы/задачи**.
7. **Документы**.
8. **История/provenance** — только с правом.

Скрытые tabs/fields не оставляют пустых headers или counts.

## 5. Создание/редактирование

- form по schema revision;
- required/conditional fields;
- repeating values/relations;
- entity reference exact search;
- duplicate suggestion до commit;
- raw/canonical preview для identity fields;
- save draft option;
- server validation;
- create-and-open/create-and-add-related;
- unsaved changes guard;
- expected revision.

Duplicate match:

- exact: предложить открыть существующий или явно создать distinct, если policy разрешает;
- fuzzy: candidate list с объяснением, без auto-merge;
- user не видит скрытого matching entity; server может вернуть generic conflict.

## 6. Связанные сведения

Информационная модель legacy «документы/адрес/телефоны» сохраняется:

- секции по relation predicate;
- items с title, qualifiers, validity, source;
- add/link existing;
- inline edit только для простого безопасного relation;
- counts/cursors;
- conflict/freshness;
- direction label;
- effective/historical switch.

Руководитель/учредитель — relation entity, если нужны доля/период/роль.

## 7. Граф и доступное дерево

Toolbar: as_of, depth, predicates, direction, layout, hide resolved, export image/data по permission.

- старт с depth 1;
- раскрытие узла по запросу;
- max budget от Security API;
- цикл отмечается link-back;
- node color не единственный type marker;
- edge label/period/source;
- high-degree node paged;
- hidden node не угадывается по gap/count;
- alternative table/tree view для keyboard/screen reader;
- selection открывает context panel, не обязательно navigation.

## 8. Глобальный поиск

- переключатель точный ID/текст;
- type chips;
- search in external sources создаёт check job, а не синхронный browser call;
- recent searches локально только если policy;
- safe highlighting;
- no hidden type in suggestions.

## 9. Заявка на проверку

Форма:

- новая/действующая;
- initiator/recommender;
- категория сотрудничества;
- target entity или создание;
- requested product/limit/deferment;
- purpose/urgency;
- required documents;
- due date;
- selected check profile;
- save draft/submit.

Timeline показывает assignment, sources, factors, decisions и SLA.

## 10. Проверка

- target summary;
- check definition/version/status;
- источники с queued/running/success/stale/error;
- результаты grouped по predicate/topic;
- conflicts/provenance;
- criteria/factors;
- manual notes/evidence;
- run/retry source;
- approve/reject/request info;
- report.

Ошибка источника отлична от «ничего не найдено».

## 11. Риски и решение

Factor list: severity, criterion, detected/age, status, evidence, explanation. Assessment: score/category/model/data freshness. Decision form: outcome, reason codes, conditions, established limit/deferment, validity, approvals. Override требует capability/reason/expiry.

## 12. Отчёты

### Каталог

Категории из материалов:

- проверки ФЛ;
- проверки ЮЛ;
- статистика по времени/сотруднику/решению;
- графическая статистика;
- отказы;
- залоги;
- прочие configured reports.

### Параметры

- scope: клиентские заявки/прочие/все;
- date interval;
- проверявший сотрудник;
- product/branch/decision;
- optional fields;
- include external refresh;
- format/locale/timezone;
- asynchronous run.

UI показывает estimated class/limits и job progress.

## 13. Мониторинг залогов

Filters: client name/ИНН, product, contract, collateral type/status, responsible, actual/planned check ranges, overdue account request, closed contracts.

Statuses:

- >30 дней;
- просрочено;
- ≤30 дней;
- не назначено;
- возможное закрытие договора.

Каждый статус имеет icon/text/token. Result table показывает next check, owner, contract/collateral, risk/freshness и actions.

## 14. Персоны

### Служба безопасности

Видит assigned checks, evidence, external sources, factors, formal/informal affiliations и reports по data policy.

### Менеджер

Видит/создаёт request, основные сведения, разрешённые лимиты, дисциплину, ОДЗ/ПДЗ и status; не видит закрытые поля ФЛ.

### Рисковик

Видит requested/established ЛТК, deferment, assessment, factors и approval workflow.

Один пользователь может совмещать роли; итог — пересечение/объединение policies с deny-overrides, не отдельная версия страницы.

## 15. Jobs/notifications

Global drawer показывает import/export/check/report/monitor jobs. Для каждого: state, phase, progress, created, owner, result/errors, retry/cancel по rights. Notifications deduped, mark/read и deep link.

## 16. Responsive

Desktop-first для сложных таблиц/графа, но карточка/tasks работают на tablet. На узком экране sidebar превращается в drawer, таблица — выбранные columns/card list. Нельзя просто уменьшить 35-column Excel форму.

## 17. UI acceptance

- все legacy scenarios достижимы;
- новый type/view появляется после publish без deployment;
- hidden person/field/relation не виден в DOM, network, counts, graph;
- 100k-result list остаётся cursor/virtualized;
- graph cycle не зависает;
- keyboard/a11y/RTL проходят;
- status не определяется одним цветом;
- create duplicate возвращает existing ID;
- policy change немедленно очищает screen/cache.
