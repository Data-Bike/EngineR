# Импорт, экспорт и отчётность

## 1. Общая модель

Любая тяжёлая операция — job. Upload/download отделены от запуска. Входной файл immutable и имеет hash. Mapping/definition/schema/policy versions фиксируются. Результаты воспроизводимы и аудируемы.

## 2. Поддерживаемые форматы

Импорт: XLSX, CSV, TSV, NDJSON, JSON package. Экспорт: XLSX, CSV, NDJSON, JSON package, DOCX, PDF. Legacy XLS при необходимости преобразуется изолированным scanner/converter, не парсится основным API.

Файлы считаются недоверенными: size/zip-bomb/formula/macro/external-link/MIME checks и antivirus.

## 3. Import pipeline

1. upload intent и scan;
2. structure inspection: sheets/headers/types/merged cells;
3. выбор mapping profile;
4. preview + validation sample;
5. dry run со статистикой create/update/dedupe/conflict/error;
6. approval при массовом изменении;
7. chunk execution с per-row idempotency;
8. reconciliation/checksum;
9. error workbook/JSON;
10. audit и artifact retention.

## 4. MappingProfile

- source sheet/range/header rows;
- column → predicate/path;
- type/locale/date/decimal parsing;
- normalizers;
- identity policy;
- relation creation;
- source/provenance;
- empty/null semantics;
- default/lookup;
- conflict action;
- formula handling;
- row filter;
- version/tests.

Empty cell не означает delete. Delete/revoke требует явного operation marker.

## 5. Нормативный mapping Excel ЮЛ

| Excel | Target |
|---|---|
| №/дата/инициатор/категория | check_request entity |
| ОПФ/наименование/бренд | legal_entity statements |
| ИНН/ОГРН | identity statements/keys |
| ОКВЭД | relation to activity classifier with valid interval |
| уставный капитал | money value + as_of/currency |
| даты/регистрирующий орган | registration statements/organization relation |
| юридический/фактический адрес | address entity + typed relation |
| лицензия | license entity + issuer/period/status |
| www/e-mail/телефон | contact entities, normalized/idempotent |
| руководитель | person entity + management relation + period/source |
| учредитель/доля | person/legal entity + ownership relation entity |
| формальная/неформальная связь | distinct affiliation predicates + evidence |

Несколько учредителей не должны упаковываться в одну строку: importer поддерживает repeat rows/related sheets или создаёт несколько relation records из structured cell только по явному parser.

## 6. Dedupe при импорте

Row idempotency key = file hash + mapping version + sheet + row + logical operation. Semantic identity дополнительно предотвращает дубли между разными файлами. Повтор job возвращает прежние row outcomes. Fuzzy candidates попадают в review queue.

## 7. Формулы

По умолчанию импортируется вычисленное значение и сохраняется формула/provenance metadata, но формула не исполняется сервером. External links/macros запрещены. Если formula-driven workbook нормативен, отдельный controlled recalculation service фиксирует engine/version.

## 8. Экспорт

ExportDefinition содержит Query AST, columns/paths, translations, formats, sorting, template, limits и classification. Runtime:

- reauthorize;
- bind policy/schema snapshot;
- stream chunks;
- apply masks before writer;
- escape formula injection в CSV/XLSX;
- watermark/classification;
- encrypt/sign if required;
- artifact TTL/download decision.

Любое значение, начинающееся =, +, -, @ в CSV/Excel text context экранируется по policy, чтобы исключить formula injection.

## 9. ReportDefinition

- dataset query/query parameters;
- sections, tables, charts, summaries;
- locale/timezone;
- DOCX/XLSX/PDF template version;
- field formatting;
- confidentiality/watermark;
- page/row limits;
- allowed audiences/formats;
- snapshot consistency;
- schedule/delivery.

Template не содержит SQL и не может обращаться к произвольному object path вне declared dataset.

## 10. Обязательные отчёты СКР

- карточка/досье ФЛ;
- карточка/досье ЮЛ;
- состав/связи холдинга;
- результат проверки и доказательства;
- проверки по дням/месяцам;
- статистика по сотруднику/решению/подразделению;
- графическая статистика;
- статистика отказов;
- платёжная дисциплина/лимиты/отсрочки;
- мониторинг залогов и сроков повторной проверки;
- использование системы и SLA задач.

Список расширяется definitions, не кодом.

## 11. Мониторинг залогов

Report filters повторяют материалы: клиент/ИНН, продукт, договор, вид/статус залога, ответственный, фактические/плановые даты, просрочка, закрытые договоры. Статус вычисляется rule version:

- следующая проверка позже 30 дней;
- просрочена;
- в ближайшие 30 дней;
- не назначена;
- возможно, договор закрыт.

Цвет — presentation token; смысл задаётся status code/label, доступен screen reader.

## 12. Security

Report/export не использует service account как способ обойти права инициатора. Есть два режима:

- current authorization на каждый chunk;
- approved snapshot grant с TTL/definition/fingerprint.

Расписание работает как service principal, но имеет собственный ограниченный data grant и проверяет получателей. Скрытое поле нельзя вывести шаблоном.

## 13. Delivery

Download — short-lived authorization, Range support при допустимости, checksum и no-cache. E-mail обычно содержит ссылку, не вложение. Вложение допустимо только encrypted/policy-approved. Каждая доставка в ledger.

## 14. Retention

Source, error и report artifacts имеют classification-based TTL/legal hold. Истечение удаляет blob recoverably/cryptographically по policy, metadata/audit сохраняются в разрешённом объёме.

## 15. Acceptance

- round-trip IDs/Unicode/decimal/date;
- no formula injection;
- no hidden field in file/metadata/chart/cache;
- 100k/1M row streaming без memory growth;
- duplicate file/job no duplicate entities;
- per-row errors локализованы и содержат source coordinates;
- generated DOCX/XLSX/PDF проходит structural/visual golden checks.
