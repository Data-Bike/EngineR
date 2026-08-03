# Международность, Unicode и локализация

## 1. Базовые правила

- UTF-8 end-to-end: source, API, DB, files, logs и UI.
- PostgreSQL database encoding UTF8.
- Locale — BCP 47 tag; timezone — IANA ID.
- API keys/aliases — ASCII lower kebab/snake policy; labels — Unicode translations.
- Raw user value сохраняется; canonical/search forms отдельны.
- Нельзя менять смысл текста глобальным lower/trim/NFKC.

## 2. Unicode normalization

Default canonical text — NFC. NFKC применяется только к конкретному identity/search field после анализа, потому что может объединить семантически разные символы. Normalizer version хранится.

Проверяются:

- control/zero-width/bidi controls;
- confusable aliases в admin;
- grapheme-safe truncation;
- combining marks;
- emoji/non-BMP;
- invalid UTF-8 rejected before serde;
- filename normalization без использования имени как storage key.

## 3. Collation

DB equality identity основано на canonical bytes/rules, не меняющейся OS collation. User sort использует versioned ICU collation/profile с locale и deterministic tie-breaker public ID. Обновление ICU/collation требует REINDEX/verification plan.

## 4. Переводы metadata

LocalizedText ключ: resource kind/id, field, locale, revision. Fallback chain настраивается, например fr-CA → fr → tenant default → en. API возвращает IDs и локализованный display, чтобы client не терял смысл.

Переводятся labels, plural labels, descriptions, help, validation/error message keys, menu, report/workflow templates. Alias/ID не переводится.

## 5. User data

Имя/адрес/наименование может иметь несколько language-tagged variants, но система не создаёт машинный перевод автоматически без provenance. Legal name сохраняется как официальное значение; display/transliteration — отдельные statements.

## 6. Имена людей

Нельзя предполагать first/middle/last:

- structured components optional;
- full legal name;
- native script;
- transliteration;
- order/display style per locale;
- mononym supported;
- patronymic/multiple family names;
- honorific/suffix отдельны.

Legacy ФИО mapping сохраняет компоненты, но UI способен показать full_name.

## 7. Адрес

Address entity содержит country, administrative areas, locality, postal code, street, building/unit и formatted lines; raw/source address сохраняется. Состав зависит от страны. Нельзя требовать российскую структуру глобально.

## 8. Телефоны/e-mail/домены

Phone parsing requires country context; canonical E.164 where possible, extension separately, raw retained. E-mail domain IDNA/UTS-46; local part не lower-case без policy. URLs поддерживают internationalized domain, но canonical storage использует safe ASCII host и display Unicode с anti-phishing checks.

## 9. Время

- instant — UTC;
- user/tenant/source IANA zone хранится;
- local date не преобразуется в midnight UTC;
- date-only birthdays/registration dates остаются date;
- schedule хранит local rule + zone + DST policy;
- reports показывают выбранную zone;
- API всегда явно указывает offset/zone semantics.

## 10. Числа и деньги

API decimal строкой с точкой как protocol canonical. UI парсит locale separators и форматирует Intl. Money = amount + ISO currency; проценты хранятся как ratio/percent по predicate contract; units versioned. Excel parsing использует cell typed value/locale, не визуальную строку.

## 11. Plurals/message format

Message catalog использует ICU-style named parameters/plurals/select. Нельзя конкатенировать переведённые фрагменты. Server отдаёт message_key + typed params. HTML в translations запрещён либо sanitised allowlist для help.

## 12. RTL

- dir определяется locale/content;
- logical CSS properties;
- icons arrows mirror only semantically;
- numbers/IDs isolate bidi;
- tables/forms/graph alternative tested;
- user-provided bidi controls visualized/sanitized по field policy.

## 13. Поиск

SearchProfile определяет language analyzer, stemming, stop words, accent/transliteration. Exact ID поиск отдельный. Cross-script search optional и provenance-neutral. Highlight grapheme-safe и security-filtered.

## 14. Reports/files

- localized fonts embedded/available;
- CJK/Arabic/Cyrillic coverage;
- RTL DOCX/PDF;
- locale formats;
- XLSX stores typed values and invariant format codes;
- CSV encoding UTF-8 with optional BOM by profile;
- filenames safe but display title Unicode;
- all templates have locale/fallback.

## 15. Test matrix

Минимум: ru-RU, en-US, fr-FR, ar (RTL), tr-TR case rules, zh-CN, mixed scripts/emoji. Проверяются sort, normalization, identity, forms, search, reports, import/export, filenames, dates/DST, decimal/currency и accessibility.
