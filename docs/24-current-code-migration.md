# Миграция существующего прототипа

## 1. Исходное состояние

В репозитории уже есть Rocket 0.5-rc prototype, модели Object/ObjectType/Field, Link/LinkType, Dictionary, User/Group/Permission, repositories, sqlx/PostgreSQL, LFU cache и generated SQL model.

Это полезное доказательство концепции, но не production baseline.

## 2. Обнаруженные ограничения

- Cargo dependencies устаревшие/смешанные: sqlx, postgres, sled, async-std, Rocket rc.
- config.toml содержит plaintext DB password.
- SQL helpers строят SQL format-строками из values: injection/correctness risk.
- Pool создаётся lazy static с block_on и малым hardcoded max.
- IDs часто String, теряется type safety.
- Controllers местами возвращают RawHtml/RawJson строки и generic ERROR.
- FromData парсит JSON вручную и выдаёт 500 для client validation.
- Permission model частично allow-by-absence и непоследователен.
- Field security проверяется после materialization/на уровне объекта, а не query rewrite.
- ФЛ/ЮЛ ещё не реализованы универсальным schema bundle.
- Object хранит заполненный ObjectType, смешивая definition/value.
- Link/field/property физически разделены без единого statement invariant.
- Динамическое создание/alter table в persistence противоречит безопасной метамодели.
- Cache macros/invalidation требуют доказательства thread safety/tenant/policy keys.
- User serialization скрывает secret строкой hidden, но secret всё ещё живёт в broad domain type.
- Миграции/DDL из RISE должны быть заменены управляемым sqlx migration baseline.
- Нет Nuxt-клиента, jobs/outbox, policy API, provenance и production observability.

## 3. Правило перехода

Не переписывать prototype «на месте» большими коммитами. Создать новый workspace рядом, использовать strangler approach и миграционный importer. Старые исходники остаются reference до принятия нового контура.

## 4. Этап 0: безопасность репозитория

1. Немедленно заменить DB password/credential, считать опубликованный скомпрометированным.
2. Удалить secret из tracked config и подключить secret ref/env.
3. Запретить prototype для production data.
4. Включить secret/dependency scanning.
5. Зафиксировать read-only snapshot старой схемы/fixtures.

Удаление секрета из текущего файла не удаляет его из Git history; rotation обязательна.

## 5. Этап 1: workspace skeleton

Создать apps/crates по [Rust workspace](./05-rust-workspace.md). Поднять Rocket 0.5.1 health, typed config/error/request context, DbExecutor и sqlx migrations. Старые routes не подключать автоматически.

## 6. Этап 2: новый security baseline

Реализовать tenant/principal/session, capability/data groups, PDP/PEP, RLS и audit. До этого новый data API не публикуется. Legacy permissions импортируются в draft policies и проходят ручную simulation, потому что их semantics непоследовательны.

## 7. Этап 3: meta-kernel/canonical graph

Создать meta/core tables, domain types, statement families, identity/idempotency/outbox. Реализовать schema publish и basic entity create/get/query.

ObjectType → TypeDefinition.
Field → PredicateDefinition.
Object.filled.fields → Entity + scalar statements.
LinkType/Link → edge predicate/statement или reified relation.
DictionaryType/Dictionary → classifier type/entities/statements.

## 8. Этап 4: legacy data extraction

Read-only extractor:

- читает старые tables параметризованно;
- создаёт immutable migration source records;
- нормализует IDs/timestamps/values;
- маппит users/groups/objects/links/dictionaries;
- пишет row-level error/quarantine;
- создаёт old ID aliases;
- сверяет counts/relations/hashes.

Importer идемпотентен по source DB fingerprint + table + primary key + mapping version.

## 9. Этап 5: SKR package

Из материалов создать schema bundle ФЛ/ЮЛ/холдинг/contact/document/address/request/check/risk/collateral; views, identity, workflow, translations и Excel mapping. Никаких Rust structs Person/LegalEntity в core.

## 10. Этап 6: Nuxt

Реализовать login/layout, dynamic list/card/form/relations/tree, checks/reports/jobs, затем admin. Параллельно legacy UI остаётся read-only или перенаправляет по feature flag.

## 11. Этап 7: dual-run

- new writes только в new canon;
- при необходимости legacy получает one-way compatibility projection, не dual-master;
- сравнение approved read scenarios;
- users/UAT;
- performance/security/DR;
- freeze legacy writes;
- final delta import;
- switch;
- archive legacy read-only;
- decommission по retention.

## 12. Data reconciliation

Для каждого table/type:

- source rows;
- imported/quarantined/skipped;
- identity merges/candidates;
- statements by predicate;
- orphan links;
- timestamp precision/timezone;
- null/empty;
- duplicate keys;
- checksum buckets;
- sampled business comparison.

Cutover блокируется при unexplained delta.

## 13. API compatibility

Если legacy consumers существуют, создать отдельный compatibility adapter, который переводит старые paths/payload в new commands и получает security decision. Он имеет deadline deprecation/telemetry. Нельзя тащить RawHtml/generic ERROR в v1.

## 14. Что можно переиспользовать

- знание предмета/тестовые сценарии;
- Rocket stage organization после адаптации;
- концепции type/field/link/dictionary;
- часть authentication strategy как reference;
- fixtures и SQL model как источник mapping;
- Cargo.lock snapshot для forensic reproduction.

SQL string builders, secret config, allow-by-absence security и physical dynamic table DDL не переиспользуются.

## 15. Acceptance миграции

- все source rows accounted;
- old ID resolves canonical ID;
- links/provenance/history сохранены;
- no duplicate identity;
- permissions не расширились молча;
- secrets rotated;
- critical scenarios функционально эквивалентны;
- new performance/security SLO;
- rollback/cutover rehearsed;
- legacy decommission approved.
