# Ссылки и нормативные источники

## Внутренние материалы

- [Тезисы.docx](./Тезисы.docx)
- [ТЗ для СКР.docx](./ТЗ%20для%20СКР.docx)
- [Экраны СКР.docx](./Экраны%20СКР.docx)
- [Для СКР по ЮЛ.XLSX](./Для%20СКР%20по%20ЮЛ.XLSX)
- [Для СКР по ЮЛ (1).XLSX](./Для%20СКР%20по%20ЮЛ%20%281%29.XLSX)

## Rust / Rocket / sqlx

- Rocket v0.5 Guide: https://rocket.rs/guide/v0.5/
- Rocket v0.5 async migration/runtime: https://rocket.rs/guide/v0.5/upgrading/
- Rocket API: https://api.rocket.rs/v0.5/rocket/
- SQLx documentation: https://docs.rs/sqlx/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/

## PostgreSQL

- Current PostgreSQL documentation: https://www.postgresql.org/docs/current/
- Row security: https://www.postgresql.org/docs/current/ddl-rowsecurity.html
- Partitioning: https://www.postgresql.org/docs/current/ddl-partitioning.html
- Indexes: https://www.postgresql.org/docs/current/indexes.html
- Full-text search: https://www.postgresql.org/docs/current/textsearch.html
- Collation support: https://www.postgresql.org/docs/current/collation.html
- Routine vacuuming: https://www.postgresql.org/docs/current/routine-vacuuming.html
- pg_stat_statements: https://www.postgresql.org/docs/current/pgstatstatements.html

## Nuxt

- Nuxt 4 documentation: https://nuxt.com/docs/4.x/getting-started/introduction
- Nuxt UI documentation: https://ui.nuxt.com/docs/getting-started/installation/nuxt
- Vue accessibility: https://vuejs.org/guide/best-practices/accessibility.html

## Стандарты

- Unicode Standard Annex #15, Normalization: https://unicode.org/reports/tr15/
- Unicode Security Mechanisms: https://unicode.org/reports/tr39/
- BCP 47 language tags: https://www.rfc-editor.org/rfc/rfc5646
- RFC 3339 timestamps: https://www.rfc-editor.org/rfc/rfc3339
- RFC 9457 problem details (использован как справочный паттерн, контракт проекта собственный): https://www.rfc-editor.org/rfc/rfc9457
- OWASP ASVS: https://owasp.org/www-project-application-security-verification-standard/
- OWASP API Security: https://owasp.org/www-project-api-security/
- WCAG 2.2: https://www.w3.org/TR/WCAG22/
- W3C Trace Context: https://www.w3.org/TR/trace-context/

## Правило актуализации

Версии и ссылки проверяются при каждом major/minor dependency или PostgreSQL/Nuxt upgrade. Нормативным для конкретной сборки является lockfile, rust-toolchain, container digest и ADR; ссылка «current» не разрешает автоматическое обновление production.
