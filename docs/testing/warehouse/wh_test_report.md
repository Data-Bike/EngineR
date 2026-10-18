# Warehouse «ТЕСТ» — отчёт о MCP-тестировании

**Итог: 1257/1257 кейсов пройдено.**

## Что проверено

- Схема склада опубликована через MCP: 10 типов, 12 edge-предикатов, derive identity-политик.
- 22 тестовых объекта + базовые связи через MCP.
- Сценарии: приёмка, отгрузка, инвентаризация — полные процессы со связями, документами, задачами.
- Администрирование: пользователи, роли, токены, capability-отказы, аудит.
- Расширенные операции: staged upload файлов, коннекторы, аудит-чекпоинты/verify/retention, полный жизненный цикл схемы (draft→validate→impact→publish→retire→restore).
- Per-tool негативы: каждый из 95 инструментов покрыт ≥10 кейсами.

## Сводка по областям

| Область | Кейсов | PASS | FAIL/ERR |
|---------|--------|------|----------|
| WH1.schema | 12 | 12 | 0 |
| WH10.negatives | 18 | 18 | 0 |
| WH11.coverage | 95 | 95 | 0 |
| WH12.extended | 141 | 141 | 0 |
| WH12.topup | 24 | 24 | 0 |
| WH2.objects | 15 | 15 | 0 |
| WH3.relations | 13 | 13 | 0 |
| WH4.search | 16 | 16 | 0 |
| WH5.inbound | 14 | 14 | 0 |
| WH6.outbound | 13 | 13 | 0 |
| WH7.inventory | 13 | 13 | 0 |
| WH8.process | 13 | 13 | 0 |
| WH9.admin | 19 | 19 | 0 |
| WHX.assessment.create | 10 | 10 | 0 |
| WHX.assessment.get | 10 | 10 | 0 |
| WHX.assessments.list | 3 | 3 | 0 |
| WHX.audit.checkpoint_create | 3 | 3 | 0 |
| WHX.audit.checkpoint_get | 3 | 3 | 0 |
| WHX.audit.query | 3 | 3 | 0 |
| WHX.audit.retention_get | 3 | 3 | 0 |
| WHX.audit.retention_set | 5 | 5 | 0 |
| WHX.audit.verify | 3 | 3 | 0 |
| WHX.checks.create | 9 | 9 | 0 |
| WHX.checks.evidence | 10 | 10 | 0 |
| WHX.checks.get | 10 | 10 | 0 |
| WHX.checks.list | 3 | 3 | 0 |
| WHX.connectors.create | 6 | 6 | 0 |
| WHX.connectors.delete | 10 | 10 | 0 |
| WHX.connectors.disable | 10 | 10 | 0 |
| WHX.connectors.enable | 10 | 10 | 0 |
| WHX.connectors.get | 10 | 10 | 0 |
| WHX.connectors.health | 10 | 10 | 0 |
| WHX.connectors.list | 3 | 3 | 0 |
| WHX.connectors.update | 10 | 10 | 0 |
| WHX.context.get | 3 | 3 | 0 |
| WHX.dashboard.stats | 3 | 3 | 0 |
| WHX.documents.delete | 17 | 17 | 0 |
| WHX.documents.download | 17 | 17 | 0 |
| WHX.documents.list | 10 | 10 | 0 |
| WHX.documents.upload | 14 | 14 | 0 |
| WHX.entities.create | 6 | 6 | 0 |
| WHX.entities.get | 10 | 10 | 0 |
| WHX.entities.graph | 10 | 10 | 0 |
| WHX.entities.history | 10 | 10 | 0 |
| WHX.entities.list | 3 | 3 | 0 |
| WHX.entities.merge | 13 | 13 | 0 |
| WHX.entities.query | 5 | 5 | 0 |
| WHX.entities.retire | 10 | 10 | 0 |
| WHX.entities.update | 10 | 10 | 0 |
| WHX.entity_checks.list | 10 | 10 | 0 |
| WHX.factors.list | 10 | 10 | 0 |
| WHX.factors.resolve | 17 | 17 | 0 |
| WHX.files.delete | 10 | 10 | 0 |
| WHX.files.download | 10 | 10 | 0 |
| WHX.files.get | 10 | 10 | 0 |
| WHX.files.upload_abort | 10 | 10 | 0 |
| WHX.files.upload_chunk | 18 | 18 | 0 |
| WHX.files.upload_complete | 14 | 14 | 0 |
| WHX.files.upload_init | 9 | 9 | 0 |
| WHX.files.upload_status | 10 | 10 | 0 |
| WHX.monitors.list | 3 | 3 | 0 |
| WHX.monitors.run | 10 | 10 | 0 |
| WHX.permissions.group_create | 9 | 9 | 0 |
| WHX.permissions.group_delete | 10 | 10 | 0 |
| WHX.permissions.group_update | 13 | 13 | 0 |
| WHX.permissions.groups | 3 | 3 | 0 |
| WHX.relations.create | 13 | 13 | 0 |
| WHX.relations.delete | 17 | 17 | 0 |
| WHX.relations.list | 10 | 10 | 0 |
| WHX.reports.generate | 6 | 6 | 0 |
| WHX.reports.list | 3 | 3 | 0 |
| WHX.requests.create | 9 | 9 | 0 |
| WHX.requests.list | 3 | 3 | 0 |
| WHX.schema.draft_create | 6 | 6 | 0 |
| WHX.schema.draft_delete | 10 | 10 | 0 |
| WHX.schema.draft_get | 10 | 10 | 0 |
| WHX.schema.draft_impact | 10 | 10 | 0 |
| WHX.schema.draft_publish | 13 | 13 | 0 |
| WHX.schema.draft_update | 14 | 14 | 0 |
| WHX.schema.draft_validate | 10 | 10 | 0 |
| WHX.schema.drafts | 3 | 3 | 0 |
| WHX.schema.predicates | 3 | 3 | 0 |
| WHX.schema.published | 3 | 3 | 0 |
| WHX.schema.revision_get | 10 | 10 | 0 |
| WHX.schema.revision_migration | 10 | 10 | 0 |
| WHX.schema.revision_retire | 10 | 10 | 0 |
| WHX.schema.type_get | 10 | 10 | 0 |
| WHX.schema.type_view | 13 | 13 | 0 |
| WHX.schema.types | 3 | 3 | 0 |
| WHX.tasks.cancel | 10 | 10 | 0 |
| WHX.tasks.complete | 10 | 10 | 0 |
| WHX.tasks.create | 13 | 13 | 0 |
| WHX.tasks.list | 10 | 10 | 0 |
| WHX.tasks.start | 10 | 10 | 0 |
| WHX.users.activity | 10 | 10 | 0 |
| WHX.users.create | 9 | 9 | 0 |
| WHX.users.delete | 10 | 10 | 0 |
| WHX.users.get | 10 | 10 | 0 |
| WHX.users.list | 3 | 3 | 0 |
| WHX.users.role_groups | 3 | 3 | 0 |
| WHX.users.token_create | 13 | 13 | 0 |
| WHX.users.token_revoke | 17 | 17 | 0 |
| WHX.users.tokens | 10 | 10 | 0 |
| WHX.users.update | 10 | 10 | 0 |
| WHX.workflows.create | 9 | 9 | 0 |
| WHX.workflows.get | 10 | 10 | 0 |
| WHX.workflows.tasks | 10 | 10 | 0 |
| WHX.workflows.transition | 14 | 14 | 0 |

## Покрытие инструментов

Инструментов: **95**, с <10 кейсами: **0**

<details><summary>Кейсов на инструмент</summary>

| Инструмент | Кейсов |
|-----------|--------|
| `assessment.create` | 12 |
| `assessment.get` | 13 |
| `assessments.list` | 11 |
| `audit.checkpoint_create` | 11 |
| `audit.checkpoint_get` | 11 |
| `audit.query` | 13 |
| `audit.retention_get` | 10 |
| `audit.retention_set` | 10 |
| `audit.verify` | 11 |
| `checks.create` | 11 |
| `checks.evidence` | 11 |
| `checks.get` | 11 |
| `checks.list` | 11 |
| `connectors.create` | 10 |
| `connectors.delete` | 12 |
| `connectors.disable` | 12 |
| `connectors.enable` | 12 |
| `connectors.get` | 13 |
| `connectors.health` | 12 |
| `connectors.list` | 10 |
| `connectors.update` | 13 |
| `context.get` | 10 |
| `dashboard.stats` | 10 |
| `documents.delete` | 21 |
| `documents.download` | 22 |
| `documents.list` | 11 |
| `documents.upload` | 19 |
| `entities.create` | 18 |
| `entities.get` | 17 |
| `entities.graph` | 14 |
| `entities.history` | 12 |
| `entities.list` | 11 |
| `entities.merge` | 14 |
| `entities.query` | 21 |
| `entities.retire` | 12 |
| `entities.update` | 17 |
| `entity_checks.list` | 11 |
| `factors.list` | 12 |
| `factors.resolve` | 20 |
| `files.delete` | 12 |
| `files.download` | 12 |
| `files.get` | 13 |
| `files.upload_abort` | 12 |
| `files.upload_chunk` | 20 |
| `files.upload_complete` | 16 |
| `files.upload_init` | 12 |
| `files.upload_status` | 14 |
| `monitors.list` | 11 |
| `monitors.run` | 14 |
| `permissions.group_create` | 11 |
| `permissions.group_delete` | 14 |
| `permissions.group_update` | 14 |
| `permissions.groups` | 10 |
| `relations.create` | 31 |
| `relations.delete` | 18 |
| `relations.list` | 14 |
| `reports.generate` | 15 |
| `reports.list` | 10 |
| `requests.create` | 10 |
| `requests.list` | 10 |
| `schema.draft_create` | 12 |
| `schema.draft_delete` | 11 |
| `schema.draft_get` | 19 |
| `schema.draft_impact` | 13 |
| `schema.draft_publish` | 15 |
| `schema.draft_update` | 17 |
| `schema.draft_validate` | 13 |
| `schema.drafts` | 10 |
| `schema.predicates` | 10 |
| `schema.published` | 11 |
| `schema.revision_get` | 13 |
| `schema.revision_migration` | 12 |
| `schema.revision_retire` | 13 |
| `schema.type_get` | 14 |
| `schema.type_view` | 14 |
| `schema.types` | 10 |
| `tasks.cancel` | 11 |
| `tasks.complete` | 12 |
| `tasks.create` | 16 |
| `tasks.list` | 11 |
| `tasks.start` | 12 |
| `users.activity` | 16 |
| `users.create` | 11 |
| `users.delete` | 12 |
| `users.get` | 11 |
| `users.list` | 10 |
| `users.role_groups` | 10 |
| `users.token_create` | 14 |
| `users.token_revoke` | 18 |
| `users.tokens` | 11 |
| `users.update` | 11 |
| `workflows.create` | 12 |
| `workflows.get` | 12 |
| `workflows.tasks` | 11 |
| `workflows.transition` | 17 |

</details>

## Найденные и исправленные дефекты

1. **Деривация identity-политик при публикации схемы** — `apply_definitions` регистрировал типы/предикаты, но не создавал `meta.identity_policy` для типов с identity-полями. Опубликованные через API типы молча теряли дедупликацию (`identity_mode=reject` не срабатывал). Исправлено в `apps/api/src/schema_admin_routes.rs` + регрессионный тест в `pg_scenarios.rs`.
2. **MCP-контракт `checks.create`** — каталог ожидал `kind`, API ждёт `check_kind` → `unknown argument`. Исправлено в тест-кейсах.
3. **MCP-контракт `assessment.create`** — каталог ожидал `target_entity_id`, API ждёт `id` (path, сущность). Исправлено в тест-кейсах.
4. **`documents.download`/`files.download`** — отдают сырое тело (не JSON-обёртку с base64) для текстового контента; ожидания кейсов скорректированы.
5. **`schema.revision_retire`** — угасить можно только текущую опубликованную ревизию; после публикации новой ревизии прежняя автоматически уходит в `retired` (state_invalid на повторный retire). Кейс переписан на полный цикл publish→retire→restore-publish.
6. **`permissions.group_create`** — поле называется `class` (capability|data), не `kind`.
7. **Идемпотентность сьюита** — identity-дедупликация делала повторные прогоны нестабильными (существующие doc_no/SKU → повторное создание связей → 409). Все identity-поля тестовых объектов получают run-unique суффикс.

## Проверки окружения

- Docker-стек up: postgres, api, mcp, web, worker.
- Логи api/mcp за время прогона: без паник и 5xx (только ожидаемые 4xx негативов).
- `cargo fmt --check`: чисто; `cargo check -p enginer-api`: ок; регрессионный тест `schema_draft_type_publish_and_entity_create`: ок.
- `cargo clippy`: падает на pre-existing линтах `cast_precision_loss` в `crates/risk-engine` (новый тулчейн 1.95, не связано с изменениями этой кампании).

## Воспроизведение

```bash
python docs/testing/warehouse/bootstrap.py    # сервис-учётки + токены → .wh_tokens.json
python docs/testing/warehouse/provision.py    # схема+объекты через MCP → .wh_objects.json
python docs/testing/warehouse/wh_test_runner.py
python docs/testing/warehouse/gen_wh_catalog.py  # регенерация этого отчёта
```