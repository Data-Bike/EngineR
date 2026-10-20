# EngineR / СКР САНАР — комплект спецификаций

> **Лозунг:** гибкость без потери производительности.

Этот каталог — нормативное описание реализации EngineR: универсальной платформы данных и автоматизации, первым прикладным пакетом которой является Система Контроля Рисков — Система Автоматизации Нахождения Аннуитетных Рисков (СКР САНАР).

Документы покрывают доменную модель, Rocket JSON API, PostgreSQL/sqlx, отдельный контур безопасности, фоновые процессы, интеграции, отчётность, Nuxt/Nuxt UI, администрирование, эксплуатацию, тестирование и переход с существующего прототипа.

## Как читать

1. Начать с [видения и границ](./00-vision-and-scope.md) и [трассировки материалов](./01-source-materials-and-requirements.md).
2. Для общей картины прочитать [архитектуру](./02-system-architecture.md).
3. Перед реализацией данных изучить [метамодель](./03-universal-domain-model.md), [доказательство инвариантов](./04-topology-unitarity-idempotency.md), [схему PostgreSQL](./10-postgresql-schema.md) и [производительность](./11-performance-and-scaling.md).
4. Ядро реализуется по [структуре Rust workspace](./05-rust-workspace.md), [контракту API](./06-api-contract.md) и [языку запросов](./07-query-engine.md).
5. Безопасность реализуется строго по [архитектуре безопасности](./08-security-architecture.md), [Security API](./09-security-api.md) и [модели угроз](./20-threat-model.md).
6. Клиентская команда использует [архитектуру Nuxt](./16-nuxt-architecture.md), [спецификацию экранов](./17-ui-screen-specification.md) и [административную панель](./18-admin-console.md).
7. Завершение определяется [тестированием](./22-testing-and-quality.md) и [критериями приёмки](./25-delivery-plan-and-acceptance.md).

## Карта документов

| Файл | Назначение |
|---|---|
| [00-vision-and-scope.md](./00-vision-and-scope.md) | Назначение, принципы, границы продукта |
| [01-source-materials-and-requirements.md](./01-source-materials-and-requirements.md) | Выжимка из DOCX/XLSX и трассировка |
| [02-system-architecture.md](./02-system-architecture.md) | Контексты, слои и потоки |
| [03-universal-domain-model.md](./03-universal-domain-model.md) | Универсальная графовая метамодель |
| [04-topology-unitarity-idempotency.md](./04-topology-unitarity-idempotency.md) | Формальные инварианты и дедупликация |
| [05-rust-workspace.md](./05-rust-workspace.md) | Crates, зависимости и правила кода |
| [06-api-contract.md](./06-api-contract.md) | REST/JSON/WebSocket и массовые операции |
| [07-query-engine.md](./07-query-engine.md) | Безопасный JSON AST и SQL compiler |
| [08-security-architecture.md](./08-security-architecture.md) | Аутентификация, два класса прав, PDP/PEP |
| [09-security-api.md](./09-security-api.md) | Контракты решений и маскирования |
| [10-postgresql-schema.md](./10-postgresql-schema.md) | Логическая/физическая схема БД |
| [11-performance-and-scaling.md](./11-performance-and-scaling.md) | Индексы, партиции, проекции, SLO |
| [12-schema-evolution.md](./12-schema-evolution.md) | Версии схем и безопасные миграции |
| [13-events-jobs-and-automation.md](./13-events-jobs-and-automation.md) | Outbox, задания, мониторинг и реакции |
| [14-connectors-and-risk-engine.md](./14-connectors-and-risk-engine.md) | Источники, проверки и критерии риска |
| [15-import-export-and-reporting.md](./15-import-export-and-reporting.md) | Импорт/экспорт и отчёты |
| [16-nuxt-architecture.md](./16-nuxt-architecture.md) | Nuxt 4 + Nuxt UI 4 |
| [17-ui-screen-specification.md](./17-ui-screen-specification.md) | Пользовательские экраны |
| [18-admin-console.md](./18-admin-console.md) | Схемы, права, интеграции и локализация |
| [19-internationalization.md](./19-internationalization.md) | UTF-8, локали, время и переводы |
| [20-threat-model.md](./20-threat-model.md) | Угрозы и меры защиты |
| [21-observability-and-operations.md](./21-observability-and-operations.md) | Логи, метрики, трассировка и runbook |
| [22-testing-and-quality.md](./22-testing-and-quality.md) | Тесты и quality gates |
| [23-deployment-and-configuration.md](./23-deployment-and-configuration.md) | Среды, секреты и конфигурация |
| [24-current-code-migration.md](./24-current-code-migration.md) | Переход с текущего прототипа |
| [25-delivery-plan-and-acceptance.md](./25-delivery-plan-and-acceptance.md) | Этапы и приёмочные сценарии |
| [26-decisions-and-tradeoffs.md](./26-decisions-and-tradeoffs.md) | Архитектурные решения |
| [27-glossary.md](./27-glossary.md) | Словарь терминов |
| [28-catalogs.md](./28-catalogs.md) | Каталоги: второй уровень изоляции данных |
| [99-references.md](./99-references.md) | Нормативные внешние ссылки |

## Нормативность

- **ДОЛЖЕН**, **НЕЛЬЗЯ**, **СЛЕДУЕТ**, **МОЖЕТ** означают обязательное требование, запрет, рекомендацию и допустимую опцию.
- Источник истины — канонические сущности и утверждения. Индексы, снимки, витрины и отчёты — восстанавливаемые проекции.
- Обход Security Policy Enforcement Point — release blocker.
- ФЛ, ЮЛ, холдинг, договор и складская единица не кодируются в ядре специальными таблицами или ветвями.
- Физическое удаление бизнес-данных запрещено обычным API; используется завершение действия, отзыв, tombstone и регламентированная очистка.

## Исходные материалы

Исходники сохранены без изменений:

- [Тезисы.docx](./Тезисы.docx)
- [ТЗ для СКР.docx](./ТЗ%20для%20СКР.docx)
- [Экраны СКР.docx](./Экраны%20СКР.docx)
- [Для СКР по ЮЛ.XLSX](./Для%20СКР%20по%20ЮЛ.XLSX)
- [Для СКР по ЮЛ (1).XLSX](./Для%20СКР%20по%20ЮЛ%20%281%29.XLSX)
