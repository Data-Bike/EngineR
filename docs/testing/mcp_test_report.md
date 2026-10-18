# EngineR MCP — отчёт о прогоне

Результат: **1241/1241 passed**.

## Найденные и исправленные дефекты (эта кампания)

### Баги API

1. **`PUT /schemas/drafts/{id}` затирал содержимое черновика** — `types`/`predicates`/`changes` имели `#[serde(default)]`: metadata-only апдейт (display_name + semantic_version) обнулял весь каталог типов. Опубликованная после этого ревизия отдавала пустой `/schemas/types`. Исправлено на `Option<Vec>`: отсутствующее поле сохраняет контент, явный `[]` — очищает. Регрессия покрыта тестом в `pg_scenarios` (metadata-only PUT не трогает types).
2. **`POST /workflows` принимал любой `definition_id`** — инстанс создавался для несуществующего определения. Добавлена таблица `meta.workflow_definition` (миграция 23), сид 4 определений из SKR-бандла, валидация на старте (404 на неизвестный id) и проверка легальности перехода `{from,to,trigger}` по определению в `transition_workflow` (422 на нелегальный переход).
3. **`POST /entities/query` молча игнорил невалидный `query`** — строка/число вместо объекта превращались в match-all выборку. Теперь 422 на не-объектном теле и не-объектном `query`.
4. **Рассинхрон capability на read-эндпоинтах схемы** — `GET /schemas/published` и `GET /schemas/revisions/{id}` попадали под catch-all `schema.draft`, хотя это read-only каталог того же уровня, что `/schemas/types` (`entity.read`). Analyst видел tool в `tools/list`, но получал 403. Маппинг исправлен на `entity.read`.
5. **Неоднородные ответы на ghost-parent в sub-list эндпоинтах** — `GET /entities/{id}/documents`, `/entities/{id}/checks`, `GET /users/{id}/tokens`, `/users/{id}/activity` возвращали `[]` на несуществующий родитель, тогда как `relations.list`/`tasks.list`/`factors.list`/`entities.history` отдавали 404. Добавлены проверки существования — поведение унифицировано к 404.
6. **`POST /connectors` молча клампил `trust_rank`** (101 → 100) и принимал отрицательный `rate_limit_rps`. Теперь вне-диапазонные значения — 422, и в `PATCH /connectors/{id}` тоже.

### Баги MCP-каталога/адаптера

7. `requests.create`: `requested_product` не был required — API отвечал 422. Помечен обязательным.
8. `workflows.transition`: `to` и `trigger` помечены обязательными (API 422 без них).
9. `assessment.create`: не хватало `force`/`desired_level`/`decision` — добавлены.
10. `inputSchema` теперь несёт `x-location` (path/query/body) на параметр — позволяет клиентам и тест-генератору отличать path-аргументы от query/body.

## Сводка по областям

| Область | Кейсов | PASS | FAIL/ERR | Медиана, мс |
|---|---|---|---|---|
| A.entities.list/search | 11 | 11 | — | 18 |
| B.entities.get | 10 | 10 | — | 16 |
| C.entities.create | 12 | 12 | — | 22 |
| D.entities.update | 10 | 10 | — | 19 |
| E.entities.retire/merge | 11 | 11 | — | 20 |
| F.relations | 12 | 12 | — | 22 |
| G.documents/files | 15 | 15 | — | 18 |
| H.tasks | 11 | 11 | — | 18 |
| I.checks | 11 | 11 | — | 19 |
| J.requests | 10 | 10 | — | 21 |
| K.workflows | 11 | 11 | — | 26 |
| L.reports | 10 | 10 | — | 23 |
| M.connectors | 12 | 12 | — | 18 |
| N.schema | 17 | 17 | — | 22 |
| O.users/tokens | 17 | 17 | — | 19 |
| P.permission groups | 12 | 12 | — | 20 |
| Q.audit | 11 | 11 | — | 16 |
| R.monitoring/misc | 12 | 12 | — | 18 |
| S.protocol/security | 12 | 12 | — | 4 |
| T.targeted | 101 | 101 | — | 17 |
| X.context.get | 3 | 3 | — | 14 |
| X.dashboard.stats | 3 | 3 | — | 17 |
| X.entities.list | 11 | 11 | — | 18 |
| X.entities.get | 10 | 10 | — | 2 |
| X.entities.create | 6 | 6 | — | 4 |
| X.entities.update | 10 | 10 | — | 3 |
| X.entities.retire | 10 | 10 | — | 3 |
| X.entities.merge | 13 | 13 | — | 3 |
| X.entities.query | 5 | 5 | — | 3 |
| X.entities.graph | 10 | 10 | — | 3 |
| X.entities.history | 10 | 10 | — | 3 |
| X.relations.list | 10 | 10 | — | 3 |
| X.relations.create | 13 | 13 | — | 3 |
| X.relations.delete | 17 | 17 | — | 3 |
| X.documents.list | 10 | 10 | — | 3 |
| X.documents.upload | 14 | 14 | — | 3 |
| X.documents.delete | 17 | 17 | — | 2 |
| X.files.get | 10 | 10 | — | 3 |
| X.files.delete | 10 | 10 | — | 2 |
| X.files.upload_init | 9 | 9 | — | 3 |
| X.files.upload_status | 10 | 10 | — | 2 |
| X.files.upload_abort | 10 | 10 | — | 2 |
| X.files.upload_chunk | 18 | 18 | — | 3 |
| X.files.upload_complete | 14 | 14 | — | 3 |
| X.files.download | 10 | 10 | — | 2 |
| X.documents.download | 17 | 17 | — | 3 |
| X.tasks.list | 10 | 10 | — | 2 |
| X.tasks.create | 13 | 13 | — | 3 |
| X.tasks.start | 10 | 10 | — | 3 |
| X.tasks.complete | 10 | 10 | — | 3 |
| X.tasks.cancel | 10 | 10 | — | 3 |
| X.factors.list | 10 | 10 | — | 2 |
| X.factors.resolve | 17 | 17 | — | 3 |
| X.assessment.get | 10 | 10 | — | 4 |
| X.assessment.create | 10 | 10 | — | 3 |
| X.assessments.list | 9 | 9 | — | 25 |
| X.checks.list | 13 | 13 | — | 27 |
| X.checks.get | 10 | 10 | — | 3 |
| X.checks.evidence | 10 | 10 | — | 3 |
| X.checks.create | 9 | 9 | — | 4 |
| X.entity_checks.list | 10 | 10 | — | 3 |
| X.schema.types | 3 | 3 | — | 27 |
| X.schema.type_get | 10 | 10 | — | 3 |
| X.schema.type_view | 13 | 13 | — | 3 |
| X.schema.predicates | 3 | 3 | — | 20 |
| X.schema.published | 3 | 3 | — | 30 |
| X.schema.drafts | 3 | 3 | — | 4 |
| X.schema.draft_get | 10 | 10 | — | 2 |
| X.schema.draft_create | 6 | 6 | — | 3 |
| X.schema.draft_update | 14 | 14 | — | 3 |
| X.schema.draft_delete | 10 | 10 | — | 3 |
| X.schema.draft_validate | 10 | 10 | — | 3 |
| X.schema.draft_impact | 10 | 10 | — | 3 |
| X.schema.draft_publish | 13 | 13 | — | 3 |
| X.schema.revision_get | 10 | 10 | — | 3 |
| X.schema.revision_retire | 10 | 10 | — | 3 |
| X.schema.revision_migration | 10 | 10 | — | 3 |
| X.requests.list | 9 | 9 | — | 26 |
| X.requests.create | 9 | 9 | — | 4 |
| X.workflows.get | 10 | 10 | — | 3 |
| X.workflows.tasks | 10 | 10 | — | 3 |
| X.workflows.create | 9 | 9 | — | 3 |
| X.workflows.transition | 14 | 14 | — | 3 |
| X.monitors.list | 3 | 3 | — | 3 |
| X.monitors.run | 10 | 10 | — | 2 |
| X.reports.list | 3 | 3 | — | 16 |
| X.reports.generate | 6 | 6 | — | 3 |
| X.connectors.list | 3 | 3 | — | 4 |
| X.connectors.get | 10 | 10 | — | 3 |
| X.connectors.update | 10 | 10 | — | 3 |
| X.connectors.delete | 10 | 10 | — | 2 |
| X.connectors.create | 6 | 6 | — | 3 |
| X.connectors.enable | 10 | 10 | — | 3 |
| X.connectors.disable | 10 | 10 | — | 3 |
| X.connectors.health | 10 | 10 | — | 3 |
| X.audit.query | 23 | 23 | — | 21 |
| X.audit.checkpoint_get | 3 | 3 | — | 3 |
| X.audit.checkpoint_create | 3 | 3 | — | 3 |
| X.audit.verify | 3 | 3 | — | 3 |
| X.audit.retention_get | 3 | 3 | — | 3 |
| X.audit.retention_set | 5 | 5 | — | 3 |
| X.permissions.groups | 3 | 3 | — | 3 |
| X.permissions.group_create | 9 | 9 | — | 4 |
| X.permissions.group_update | 13 | 13 | — | 3 |
| X.permissions.group_delete | 10 | 10 | — | 3 |
| X.users.role_groups | 3 | 3 | — | 3 |
| X.users.list | 15 | 15 | — | 20 |
| X.users.get | 10 | 10 | — | 3 |
| X.users.create | 9 | 9 | — | 3 |
| X.users.update | 10 | 10 | — | 3 |
| X.users.delete | 10 | 10 | — | 3 |
| X.users.activity | 10 | 10 | — | 4 |
| X.users.tokens | 10 | 10 | — | 5 |
| X.users.token_create | 13 | 13 | — | 5 |
| X.users.token_revoke | 17 | 17 | — | 4 |

## Детали по каждому кейсу

### A.entities.list/search

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| A1 list all | `entities.list` | ok | PASS | 20 |
| A2 type filter person | `entities.list` | ok | PASS | 20 |
| A3 free-text search | `entities.list` | ok | PASS | 22 |
| A4 search no hit | `entities.list` | ok | PASS | 19 |
| A5 limit=1 pagination | `entities.list` | ok | PASS | 17 |
| A6 cursor page 2 | `entities.list` | ok | PASS | 17 |
| A7 limit wrong type | `entities.list` | ok | PASS | 19 |
| A8 unknown arg rejected | `entities.list` | tool_err | PASS | 4 |
| A9 type nonexistent alias | `entities.list` | ok | PASS | 17 |
| A10 analyst read allowed | `entities.list` | ok | PASS | 18 |
| A11 filter legal_entity | `entities.list` | ok | PASS | 17 |

### B.entities.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| B1 get seeded person | `entities.get` | ok | PASS | 17 |
| B2 get legal_entity | `entities.get` | ok | PASS | 17 |
| B3 nonexistent uuid → 404 | `entities.get` | fail | PASS | 16 |
| B4 malformed id | `entities.get` | fail | PASS | 14 |
| B5 path traversal rejected | `entities.get` | tool_err | PASS | 3 |
| B6 numeric id accepted→404 | `entities.get` | fail | PASS | 14 |
| B7 structured id rejected | `entities.get` | tool_err | PASS | 3 |
| B8 retired entity readable, status=retired | `entities.get` | ok | PASS | 17 |
| B9 empty id rejected | `entities.get` | tool_err | PASS | 3 |
| B10 analyst get | `entities.get` | ok | PASS | 18 |

### C.entities.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| C1 create person | `entities.create` | ok | PASS | 24 |
| C2 create legal_entity | `entities.create` | ok | PASS | 30 |
| C3 missing type_id | `entities.create` | tool_err | PASS | 3 |
| C4 unknown type alias | `entities.create` | fail | PASS | 16 |
| C5 duplicate INN → return_existing | `entities.create` | ok | PASS | 18 |
| C6 identity_mode=reject on dup | `entities.create` | fail | PASS | 19 |
| C7 create document | `entities.create` | ok | PASS | 28 |
| C8 create check entity | `entities.create` | ok | PASS | 22 |
| C9 analyst denied | `entities.create` | deny | PASS | 4 |
| C10 unknown arg | `entities.create` | tool_err | PASS | 4 |
| C11 create request entity | `entities.create` | ok | PASS | 23 |
| C12 create_candidate creates new | `entities.create` | ok | PASS | 25 |

### D.entities.update

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| D1 update field | `entities.update` | ok | PASS | 31 |
| D2 stale revision | `entities.update` | fail | PASS | 17 |
| D3 nonexistent id | `entities.update` | fail | PASS | 17 |
| D4 update retired | `entities.update` | fail | PASS | 21 |
| D5 single predicate form | `entities.update` | ok | PASS | 19 |
| D6 unknown arg | `entities.update` | tool_err | PASS | 4 |
| D7 analyst denied | `entities.update` | deny | PASS | 4 |
| D8 verify persisted | `entities.get` | ok | PASS | 22 |
| D9 update le field | `entities.update` | ok | PASS | 31 |
| D10 no fields no-op | `entities.update` | ok | PASS | 19 |

### E.entities.retire/merge

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| E1 create loser | `entities.create` | ok | PASS | 32 |
| E2 merge into canonical | `entities.merge` | ok | PASS | 27 |
| E3 merge idempotent repeat | `entities.merge` | ok | PASS | 19 |
| E4 loser marked (not 404) | `entities.get` | ok | PASS | 21 |
| E5 merge nonexistent loser | `entities.merge` | fail | PASS | 20 |
| E6 self merge | `entities.merge` | fail | PASS | 20 |
| E7 retire with reason | `entities.retire` | ok | PASS | 25 |
| E8 retire again → 404 | `entities.retire` | fail | PASS | 19 |
| E9 analyst retire denied | `entities.retire` | deny | PASS | 4 |
| E10 merge analyst denied | `entities.merge` | deny | PASS | 3 |
| E11 retire nonexistent | `entities.retire` | fail | PASS | 18 |

### F.relations

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| F1 works_for | `relations.create` | ok | PASS | 26 |
| F2 owns | `relations.create` | ok | PASS | 26 |
| F3 for_entity request→le | `relations.create` | ok | PASS | 26 |
| F4 related_to wildcard | `relations.create` | ok | PASS | 22 |
| F5 missing predicate | `relations.create` | tool_err | PASS | 4 |
| F6 unknown predicate | `relations.create` | fail | PASS | 19 |
| F7 missing target | `relations.create` | fail | PASS | 19 |
| F8 list outgoing | `relations.list` | ok | PASS | 22 |
| F9 list incoming on le | `relations.list` | ok | PASS | 21 |
| F10 delete relation | `relations.delete` | ok | PASS | 22 |
| F11 delete nonexistent | `relations.delete` | fail | PASS | 18 |
| F12 analyst create denied | `relations.create` | deny | PASS | 3 |

### G.documents/files

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| G1 documents.upload | `documents.upload` | ok | PASS | 22 |
| G2 documents.list | `documents.list` | ok | PASS | 18 |
| G3 documents.download | `documents.download` | ok | PASS | 19 |
| G4 upload_init | `files.upload_init` | ok | PASS | 20 |
| G5 upload_chunk base64 | `files.upload_chunk` | ok | PASS | 20 |
| G6 upload_status | `files.upload_status` | ok | PASS | 16 |
| G7 upload_complete | `files.upload_complete` | ok | PASS | 21 |
| G8 files.get | `files.get` | ok | PASS | 15 |
| G9 files.download | `files.download` | ok | PASS | 16 |
| G10 upload_init bad hash len | `files.upload_init` | fail | PASS | 13 |
| G11 upload_chunk bad base64 | `files.upload_chunk` | tool_err | PASS | 4 |
| G12 upload_abort | `files.upload_abort` | fail | PASS | 16 |
| G13 files.delete | `files.delete` | ok | PASS | 18 |
| G14 documents.delete | `documents.delete` | ok | PASS | 18 |
| G15 analyst upload denied | `documents.upload` | deny | PASS | 3 |

### H.tasks

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| H1 tasks.create | `tasks.create` | ok | PASS | 21 |
| H2 tasks.list | `tasks.list` | ok | PASS | 19 |
| H3 tasks.start | `tasks.start` | ok | PASS | 19 |
| H4 tasks.complete | `tasks.complete` | ok | PASS | 20 |
| H5 create+cancel | `tasks.create` | ok | PASS | 19 |
| H6 tasks.cancel | `tasks.cancel` | ok | PASS | 18 |
| H7 missing kind → 422 | `tasks.create` | fail | PASS | 4 |
| H8 complete completed → error | `tasks.complete` | fail | PASS | 16 |
| H9 task on ghost entity → 404 | `tasks.create` | fail | PASS | 13 |
| H10 analyst create denied | `tasks.create` | deny | PASS | 4 |
| H11 tasks.list analyst | `tasks.list` | ok | PASS | 18 |

### I.checks

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| I1 checks.create kyc | `checks.create` | ok | PASS | 19 |
| I2 checks.create sanctions | `checks.create` | ok | PASS | 21 |
| I3 checks.list | `checks.list` | ok | PASS | 31 |
| I4 checks.list kind filter | `checks.list` | ok | PASS | 23 |
| I5 checks.list status filter | `checks.list` | ok | PASS | 19 |
| I6 checks.get | `checks.get` | ok | PASS | 19 |
| I7 checks.evidence | `checks.evidence` | ok | PASS | 18 |
| I8 entity_checks.list | `entity_checks.list` | ok | PASS | 18 |
| I9 missing check_kind | `checks.create` | tool_err | PASS | 3 |
| I10 ghost target → 404 | `checks.create` | fail | PASS | 17 |
| I11 analyst denied | `checks.create` | deny | PASS | 4 |

### J.requests

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| J1 requests.create | `requests.create` | ok | PASS | 21 |
| J2 requests.list | `requests.list` | ok | PASS | 35 |
| J3 status filter | `requests.list` | ok | PASS | 34 |
| J4 missing target | `requests.create` | tool_err | PASS | 4 |
| J5 ghost target → 404 | `requests.create` | fail | PASS | 4 |
| J6 limit param | `requests.list` | ok | PASS | 21 |
| J7 unknown arg | `requests.create` | tool_err | PASS | 4 |
| J8 create minimal | `requests.create` | ok | PASS | 26 |
| J9 analyst list | `requests.list` | ok | PASS | 39 |
| J10 analyst create denied | `requests.create` | deny | PASS | 5 |

### K.workflows

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| K1 workflows.create | `workflows.create` | ok | PASS | 33 |
| K2 workflows.get | `workflows.get` | ok | PASS | 25 |
| K3 workflows.tasks | `workflows.tasks` | ok | PASS | 27 |
| K4 transition valid | `workflows.transition` | ok | PASS | 31 |
| K5 transition invalid state | `workflows.transition` | fail | PASS | 26 |
| K6 missing definition_id | `workflows.create` | tool_err | PASS | 5 |
| K7 unknown definition | `workflows.create` | fail | PASS | 26 |
| K8 get ghost → 404 | `workflows.get` | fail | PASS | 24 |
| K9 analyst create denied | `workflows.create` | deny | PASS | 6 |
| K10 create without target → 404 | `workflows.create` | fail | PASS | 6 |
| K11 transition ghost | `workflows.transition` | fail | PASS | 32 |

### L.reports

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| L1 generate json | `reports.generate` | ok | PASS | 37 |
| L2 reports.list | `reports.list` | ok | PASS | 29 |
| L3 generate csv | `reports.generate` | ok | PASS | 27 |
| L4 missing entity_id | `reports.generate` | tool_err | PASS | 3 |
| L5 ghost entity → 404 | `reports.generate` | fail | PASS | 17 |
| L6 generate pdf | `reports.generate` | ok | PASS | 22 |
| L7 generate no format | `reports.generate` | ok | PASS | 23 |
| L8 second report new id | `reports.generate` | ok | PASS | 25 |
| L9 analyst denied | `reports.generate` | deny | PASS | 4 |
| L10 reports.list analyst | `reports.list` | ok | PASS | 19 |

### M.connectors

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| M1 create | `connectors.create` | ok | PASS | 19 |
| M2 list | `connectors.list` | ok | PASS | 18 |
| M3 get | `connectors.get` | ok | PASS | 18 |
| M4 update disable | `connectors.update` | ok | PASS | 23 |
| M5 enable | `connectors.enable` | ok | PASS | 19 |
| M6 disable | `connectors.disable` | ok | PASS | 18 |
| M7 health | `connectors.health` | ok | PASS | 16 |
| M8 missing type → tool_err | `connectors.create` | tool_err | PASS | 3 |
| M9 get ghost → 404 | `connectors.get` | fail | PASS | 15 |
| M10 update ghost → 404 | `connectors.update` | fail | PASS | 16 |
| M11 analyst denied | `connectors.create` | deny | PASS | 3 |
| M12 delete | `connectors.delete` | ok | PASS | 18 |

### N.schema

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| N1 schema.types | `schema.types` | ok | PASS | 30 |
| N2 type_get alias | `schema.type_get` | ok | PASS | 21 |
| N3 type_view form | `schema.type_view` | ok | PASS | 22 |
| N4 schema.predicates | `schema.predicates` | ok | PASS | 21 |
| N5 schema.published | `schema.published` | ok | PASS | 30 |
| N6 draft_create | `schema.draft_create` | ok | PASS | 26 |
| N7 draft_get | `schema.draft_get` | ok | PASS | 22 |
| N8 draft_update no etag → 412 | `schema.draft_update` | fail | PASS | 21 |
| N9 draft_update +etag | `schema.draft_update` | ok | PASS | 25 |
| N10 draft_validate | `schema.draft_validate` | ok | PASS | 23 |
| N11 draft_impact | `schema.draft_impact` | ok | PASS | 24 |
| N11b refresh etag | `schema.draft_get` | ok | PASS | 21 |
| N12 draft_publish +etag | `schema.draft_publish` | ok | PASS | 61 |
| N13 revision_get | `schema.revision_get` | ok | PASS | 19 |
| N14 type_get ghost → 404 | `schema.type_get` | fail | PASS | 19 |
| N15 analyst schema.types | `schema.types` | ok | PASS | 26 |
| N16 analyst draft denied | `schema.draft_create` | deny | PASS | 3 |

### O.users/tokens

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| O1 users.list | `users.list` | ok | PASS | 19 |
| O2 q filter | `users.list` | ok | PASS | 16 |
| O3 status filter | `users.list` | ok | PASS | 19 |
| O4 users.get | `users.get` | ok | PASS | 22 |
| O5 create user | `users.create` | ok | PASS | 35 |
| O6 create service acct | `users.create` | ok | PASS | 20 |
| O7 dup login rejected | `users.create` | fail | PASS | 29 |
| O8 token_create | `users.token_create` | ok | PASS | 17 |
| O9 tokens list no secret | `users.tokens` | ok | PASS | 15 |
| O10 token_revoke | `users.token_revoke` | ok | PASS | 18 |
| O11 users.update groups | `users.update` | ok | PASS | 22 |
| O12 users.get roles | `users.get` | ok | PASS | 20 |
| O13 users.activity | `users.activity` | ok | PASS | 22 |
| O14 users.delete | `users.delete` | ok | PASS | 19 |
| O15 analyst denied users.list | `users.list` | deny | PASS | 3 |
| O15b role catalog | `users.role_groups` | ok | PASS | 16 |
| O16 short password rejected | `users.create` | fail | PASS | 14 |

### P.permission groups

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| P1 groups list | `permissions.groups` | ok | PASS | 17 |
| P2 group_create capability | `permissions.group_create` | ok | PASS | 25 |
| P3 group_create data | `permissions.group_create` | ok | PASS | 23 |
| P4 invalid class → 422 | `permissions.group_create` | fail | PASS | 14 |
| P5 missing name → tool_err | `permissions.group_create` | tool_err | PASS | 3 |
| P6 group_update principals | `permissions.group_update` | ok | PASS | 26 |
| P7 group_update parent | `permissions.group_update` | ok | PASS | 24 |
| P8 delete special role | `permissions.group_delete` | fail | PASS | 17 |
| P9 delete qa role | `permissions.group_delete` | ok | PASS | 21 |
| P10 analyst denied | `permissions.groups` | deny | PASS | 4 |
| P11 delete ghost → 404 | `permissions.group_delete` | fail | PASS | 18 |
| P12 delete data role | `permissions.group_delete` | ok | PASS | 20 |

### Q.audit

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| Q1 audit.query | `audit.query` | ok | PASS | 22 |
| Q2 filter action | `audit.query` | ok | PASS | 16 |
| Q3 filter effect | `audit.query` | ok | PASS | 20 |
| Q4 checkpoint_get | `audit.checkpoint_get` | ok | PASS | 16 |
| Q5 checkpoint_create | `audit.checkpoint_create` | ok | PASS | 19 |
| Q6 audit.verify | `audit.verify` | ok | PASS | 25 |
| Q7 retention_get | `audit.retention_get` | ok | PASS | 16 |
| Q8 retention_set | `audit.retention_set` | ok | PASS | 17 |
| Q9 retention bad days → 422 | `audit.retention_set` | fail | PASS | 13 |
| Q10 analyst denied | `audit.query` | deny | PASS | 3 |
| Q11 restore retention | `audit.retention_set` | ok | PASS | 16 |

### R.monitoring/misc

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| R1 context.get | `context.get` | ok | PASS | 18 |
| R2 dashboard.stats | `dashboard.stats` | ok | PASS | 21 |
| R3 monitors.list | `monitors.list` | ok | PASS | 16 |
| R4 monitors.run | `monitors.run` | ok | PASS | 18 |
| R5 factors.list | `factors.list` | ok | PASS | 17 |
| R6 assessments.list | `assessments.list` | ok | PASS | 29 |
| R7 assessment.create | `assessment.create` | ok | PASS | 23 |
| R7b invalid level → 422 | `assessment.create` | fail | PASS | 13 |
| R8 entities.history | `entities.history` | ok | PASS | 17 |
| R9 entities.query AST | `entities.query` | ok | PASS | 21 |
| R10 analyst context | `context.get` | ok | PASS | 18 |
| R11 analyst dashboard | `dashboard.stats` | ok | PASS | 25 |

### S.protocol/security

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| S1 no token → 401 | `__raw__` | http401 | PASS | 4 |
| S2 garbage token → 401 | `__raw__` | http401 | PASS | 3 |
| S3 unknown method → -32601 | `__rpc__` | fail | PASS | 4 |
| S4 unknown tool → -32602 | `no.such.tool` | fail | PASS | 3 |
| S5 analyst tools/list smaller | `__tools_list__` | ok | PASS | 4 |
| S6 analyst admin tool denied | `audit.retention_set` | deny | PASS | 3 |
| S7 path arg '?x' rejected | `entities.get` | tool_err | PASS | 4 |
| S8 path arg 'a/b' rejected | `entities.get` | tool_err | PASS | 5 |
| S9 initialize | `__rpc__` | ok | PASS | 3 |
| S10 missing required arg | `entities.get` | tool_err | PASS | 3 |
| S11 tools/list admin 90+ | `__tools_list__` | ok | PASS | 5 |
| S12 token revoked mid-flight → 401 | `__raw__` | http401 | PASS | 6 |

### T.targeted

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| reports.list has data | `reports.list` | ok | PASS | 17 |
| reports.list has ids | `reports.list` | ok | PASS | 17 |
| reports.list status field | `reports.list` | ok | PASS | 18 |
| reports.list repeat | `reports.list` | ok | PASS | 17 |
| reports.list limit probe | `reports.list` | tool_err | PASS | 3 |
| connectors.create trust>100 | `connectors.create` | fail | PASS | 13 |
| connectors.create negative rps | `connectors.create` | fail | PASS | 14 |
| connectors.list has data | `connectors.list` | ok | PASS | 16 |
| connectors.list has conn | `connectors.list` | ok | PASS | 16 |
| connectors.list repeat | `connectors.list` | ok | PASS | 16 |
| connectors.list analyst deny2 | `connectors.list` | deny | PASS | 3 |
| connectors.create t-conn | `connectors.create` | ok | PASS | 17 |
| connectors.list sees t-conn | `connectors.list` | ok | PASS | 16 |
| connectors.list bogus | `connectors.list` | tool_err | PASS | 4 |
| schema.types person | `schema.types` | ok | PASS | 26 |
| schema.types legal_entity | `schema.types` | ok | PASS | 28 |
| schema.types request | `schema.types` | ok | PASS | 28 |
| schema.types document | `schema.types` | ok | PASS | 27 |
| schema.types contract | `schema.types` | ok | PASS | 28 |
| schema.predicates works_for | `schema.predicates` | ok | PASS | 20 |
| schema.predicates owns | `schema.predicates` | ok | PASS | 22 |
| schema.predicates inn | `schema.predicates` | ok | PASS | 21 |
| schema.predicates full_name | `schema.predicates` | ok | PASS | 23 |
| schema.predicates has_contract | `schema.predicates` | ok | PASS | 22 |
| schema.predicates related_to | `schema.predicates` | ok | PASS | 21 |
| schema.published revision_id | `schema.published` | ok | PASS | 29 |
| schema.published semver | `schema.published` | ok | PASS | 29 |
| schema.published status | `schema.published` | ok | PASS | 30 |
| schema.published types | `schema.published` | ok | PASS | 31 |
| schema.published person | `schema.published` | ok | PASS | 32 |
| schema.published etag-ish | `schema.published` | ok | PASS | 30 |
| schema.drafts has data | `schema.drafts` | ok | PASS | 22 |
| schema.drafts revision id | `schema.drafts` | ok | PASS | 22 |
| schema.drafts shows qa | `schema.drafts` | ok | PASS | 23 |
| schema.drafts repeat | `schema.drafts` | ok | PASS | 22 |
| schema.drafts analyst deny2 | `schema.drafts` | deny | PASS | 4 |
| schema.drafts bogus arg | `schema.drafts` | tool_err | PASS | 3 |
| schema.drafts has status | `schema.drafts` | ok | PASS | 23 |
| draft_create ghost base | `schema.draft_create` | fail | PASS | 20 |
| draft_create empty name | `schema.draft_create` | fail | PASS | 15 |
| role_groups admins | `users.role_groups` | ok | PASS | 16 |
| role_groups analysts | `users.role_groups` | ok | PASS | 16 |
| role_groups all_data | `users.role_groups` | ok | PASS | 15 |
| role_groups class field | `users.role_groups` | ok | PASS | 16 |
| role_groups capability | `users.role_groups` | ok | PASS | 15 |
| role_groups parent | `users.role_groups` | ok | PASS | 16 |
| perm.groups admins | `permissions.groups` | ok | PASS | 17 |
| perm.groups rules | `permissions.groups` | ok | PASS | 17 |
| perm.groups class | `permissions.groups` | ok | PASS | 18 |
| perm.groups all_data | `permissions.groups` | ok | PASS | 21 |
| perm.groups repeat | `permissions.groups` | ok | PASS | 18 |
| checkpoint_get data | `audit.checkpoint_get` | ok | PASS | 16 |
| checkpoint_get again | `audit.checkpoint_get` | ok | PASS | 15 |
| checkpoint_get id field | `audit.checkpoint_get` | ok | PASS | 16 |
| checkpoint_get 4th | `audit.checkpoint_get` | ok | PASS | 16 |
| checkpoint_get 5th | `audit.checkpoint_get` | ok | PASS | 15 |
| checkpoint_get bogus | `audit.checkpoint_get` | tool_err | PASS | 4 |
| checkpoint_create #2 | `audit.checkpoint_create` | ok | PASS | 18 |
| checkpoint_create #3 | `audit.checkpoint_create` | ok | PASS | 18 |
| checkpoint_create #4 | `audit.checkpoint_create` | ok | PASS | 17 |
| checkpoint_create idempotent-shape | `audit.checkpoint_create` | ok | PASS | 17 |
| checkpoint visible in get | `audit.checkpoint_get` | ok | PASS | 15 |
| checkpoint_create bogus | `audit.checkpoint_create` | tool_err | PASS | 3 |
| checkpoint_create #5 | `audit.checkpoint_create` | ok | PASS | 17 |
| verify after checkpoints | `audit.verify` | ok | PASS | 27 |
| verify range args | `audit.verify` | ok | PASS | 26 |
| verify true | `audit.verify` | ok | PASS | 26 |
| verify analyst deny2 | `audit.verify` | deny | PASS | 3 |
| verify repeat | `audit.verify` | ok | PASS | 25 |
| verify bad range | `audit.verify` | ok | PASS | 26 |
| retention_get days | `audit.retention_get` | ok | PASS | 16 |
| retention_get again | `audit.retention_get` | ok | PASS | 16 |
| retention_get fields | `audit.retention_get` | ok | PASS | 15 |
| retention_get 4th | `audit.retention_get` | ok | PASS | 15 |
| retention_get bogus | `audit.retention_get` | tool_err | PASS | 3 |
| retention_get analyst deny | `audit.retention_get` | deny | PASS | 4 |
| retention_set negative | `audit.retention_set` | fail | PASS | 13 |
| retention_set huge | `audit.retention_set` | fail | PASS | 13 |
| retention_set badtype | `audit.retention_set` | fail | PASS | 14 |
| context caps | `context.get` | ok | PASS | 18 |
| context principal | `context.get` | ok | PASS | 15 |
| context tenant | `context.get` | ok | PASS | 14 |
| context entity.read | `context.get` | ok | PASS | 14 |
| context admin cap | `context.get` | ok | PASS | 14 |
| dashboard my_tasks | `dashboard.stats` | ok | PASS | 18 |
| dashboard shape | `dashboard.stats` | ok | PASS | 18 |
| dashboard repeat | `dashboard.stats` | ok | PASS | 18 |
| dashboard analyst | `dashboard.stats` | ok | PASS | 18 |
| dashboard bogus arg | `dashboard.stats` | tool_err | PASS | 3 |
| monitors has data | `monitors.list` | ok | PASS | 16 |
| monitors seeded | `monitors.list` | ok | PASS | 15 |
| monitors frequency | `monitors.list` | ok | PASS | 17 |
| monitors repeat | `monitors.list` | ok | PASS | 15 |
| monitors analyst deny2 | `monitors.list` | deny | PASS | 3 |
| monitors bogus arg | `monitors.list` | tool_err | PASS | 3 |
| query typed person | `entities.query` | ok | PASS | 18 |
| query search | `entities.query` | ok | PASS | 20 |
| query page size 1 | `entities.query` | ok | PASS | 20 |
| query empty obj | `entities.query` | ok | PASS | 21 |
| query null→err | `entities.query` | tool_err | PASS | 3 |
| query non-object→422 | `entities.query` | fail | PASS | 17 |

### X.context.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| context.get ∅args | `context.get` | ok | PASS | 14 |
| context.get unknown arg | `context.get` | tool_err | PASS | 3 |
| context.get analyst ok | `context.get` | ok | PASS | 14 |

### X.dashboard.stats

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| dashboard.stats ∅args | `dashboard.stats` | ok | PASS | 17 |
| dashboard.stats unknown arg | `dashboard.stats` | tool_err | PASS | 3 |
| dashboard.stats analyst ok | `dashboard.stats` | ok | PASS | 17 |

### X.entities.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.list ∅args | `entities.list` | ok | PASS | 19 |
| entities.list unknown arg | `entities.list` | tool_err | PASS | 3 |
| entities.list filter:type | `entities.list` | ok | PASS | 17 |
| entities.list null-opt:type | `entities.list` | ok | PASS | 18 |
| entities.list filter:search | `entities.list` | ok | PASS | 20 |
| entities.list null-opt:search | `entities.list` | ok | PASS | 18 |
| entities.list num:limit | `entities.list` | ok | PASS | 16 |
| entities.list null-opt:limit | `entities.list` | ok | PASS | 18 |
| entities.list filter:cursor | `entities.list` | ok | PASS | 18 |
| entities.list null-opt:cursor | `entities.list` | ok | PASS | 18 |
| entities.list analyst ok | `entities.list` | ok | PASS | 20 |

### X.entities.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.get ∅args→err | `entities.get` | tool_err | PASS | 3 |
| entities.get null:id | `entities.get` | tool_err | PASS | 2 |
| entities.get unknown arg | `entities.get` | tool_err | PASS | 2 |
| entities.get badtype:id | `entities.get` | fail | PASS | 2 |
| entities.get path:id:slash | `entities.get` | tool_err | PASS | 2 |
| entities.get path:id:dotdot | `entities.get` | tool_err | PASS | 2 |
| entities.get path:id:empty | `entities.get` | tool_err | PASS | 3 |
| entities.get path:id:array | `entities.get` | tool_err | PASS | 2 |
| entities.get ghost:id | `entities.get` | fail | PASS | 16 |
| entities.get analyst reach | `entities.get` | reach | PASS | 16 |

### X.entities.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.create ∅args→err | `entities.create` | tool_err | PASS | 3 |
| entities.create null:type_id | `entities.create` | tool_err | PASS | 2 |
| entities.create unknown arg | `entities.create` | tool_err | PASS | 2 |
| entities.create badtype:type_id | `entities.create` | fail | PASS | 15 |
| entities.create emptystr:type_id | `entities.create` | fail | PASS | 15 |
| entities.create analyst deny | `entities.create` | deny | PASS | 4 |

### X.entities.update

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.update ∅args→err | `entities.update` | tool_err | PASS | 3 |
| entities.update null:id | `entities.update` | tool_err | PASS | 3 |
| entities.update unknown arg | `entities.update` | tool_err | PASS | 2 |
| entities.update badtype:id | `entities.update` | fail | PASS | 2 |
| entities.update path:id:slash | `entities.update` | tool_err | PASS | 2 |
| entities.update path:id:dotdot | `entities.update` | tool_err | PASS | 2 |
| entities.update path:id:empty | `entities.update` | tool_err | PASS | 3 |
| entities.update path:id:array | `entities.update` | tool_err | PASS | 3 |
| entities.update ghost:id | `entities.update` | fail | PASS | 16 |
| entities.update analyst deny | `entities.update` | deny | PASS | 3 |

### X.entities.retire

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.retire ∅args→err | `entities.retire` | tool_err | PASS | 3 |
| entities.retire null:id | `entities.retire` | tool_err | PASS | 2 |
| entities.retire unknown arg | `entities.retire` | tool_err | PASS | 3 |
| entities.retire badtype:id | `entities.retire` | fail | PASS | 3 |
| entities.retire path:id:slash | `entities.retire` | tool_err | PASS | 2 |
| entities.retire path:id:dotdot | `entities.retire` | tool_err | PASS | 2 |
| entities.retire path:id:empty | `entities.retire` | tool_err | PASS | 2 |
| entities.retire path:id:array | `entities.retire` | tool_err | PASS | 3 |
| entities.retire ghost:id | `entities.retire` | fail | PASS | 19 |
| entities.retire analyst deny | `entities.retire` | deny | PASS | 3 |

### X.entities.merge

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.merge ∅args→err | `entities.merge` | tool_err | PASS | 3 |
| entities.merge null:id | `entities.merge` | tool_err | PASS | 2 |
| entities.merge null:loser_public_id | `entities.merge` | tool_err | PASS | 2 |
| entities.merge unknown arg | `entities.merge` | tool_err | PASS | 5 |
| entities.merge badtype:id | `entities.merge` | fail | PASS | 3 |
| entities.merge badtype:loser_public_id | `entities.merge` | fail | PASS | 13 |
| entities.merge emptystr:loser_public_id | `entities.merge` | fail | PASS | 12 |
| entities.merge path:id:slash | `entities.merge` | tool_err | PASS | 3 |
| entities.merge path:id:dotdot | `entities.merge` | tool_err | PASS | 3 |
| entities.merge path:id:empty | `entities.merge` | tool_err | PASS | 2 |
| entities.merge path:id:array | `entities.merge` | tool_err | PASS | 2 |
| entities.merge ghost:id | `entities.merge` | fail | PASS | 14 |
| entities.merge analyst deny | `entities.merge` | deny | PASS | 3 |

### X.entities.query

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.query ∅args→err | `entities.query` | tool_err | PASS | 3 |
| entities.query null:query | `entities.query` | tool_err | PASS | 3 |
| entities.query unknown arg | `entities.query` | tool_err | PASS | 3 |
| entities.query badtype:query | `entities.query` | fail | PASS | 16 |
| entities.query analyst reach | `entities.query` | reach | PASS | 21 |

### X.entities.graph

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.graph ∅args→err | `entities.graph` | tool_err | PASS | 3 |
| entities.graph null:id | `entities.graph` | tool_err | PASS | 3 |
| entities.graph unknown arg | `entities.graph` | tool_err | PASS | 3 |
| entities.graph badtype:id | `entities.graph` | fail | PASS | 2 |
| entities.graph path:id:slash | `entities.graph` | tool_err | PASS | 2 |
| entities.graph path:id:dotdot | `entities.graph` | tool_err | PASS | 2 |
| entities.graph path:id:empty | `entities.graph` | tool_err | PASS | 3 |
| entities.graph path:id:array | `entities.graph` | tool_err | PASS | 3 |
| entities.graph ghost:id | `entities.graph` | fail | PASS | 17 |
| entities.graph analyst reach | `entities.graph` | reach | PASS | 16 |

### X.entities.history

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entities.history ∅args→err | `entities.history` | tool_err | PASS | 3 |
| entities.history null:id | `entities.history` | tool_err | PASS | 2 |
| entities.history unknown arg | `entities.history` | tool_err | PASS | 2 |
| entities.history badtype:id | `entities.history` | fail | PASS | 3 |
| entities.history path:id:slash | `entities.history` | tool_err | PASS | 3 |
| entities.history path:id:dotdot | `entities.history` | tool_err | PASS | 2 |
| entities.history path:id:empty | `entities.history` | tool_err | PASS | 2 |
| entities.history path:id:array | `entities.history` | tool_err | PASS | 2 |
| entities.history ghost:id | `entities.history` | fail | PASS | 16 |
| entities.history analyst reach | `entities.history` | reach | PASS | 17 |

### X.relations.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| relations.list ∅args→err | `relations.list` | tool_err | PASS | 4 |
| relations.list null:id | `relations.list` | tool_err | PASS | 3 |
| relations.list unknown arg | `relations.list` | tool_err | PASS | 3 |
| relations.list badtype:id | `relations.list` | fail | PASS | 2 |
| relations.list path:id:slash | `relations.list` | tool_err | PASS | 2 |
| relations.list path:id:dotdot | `relations.list` | tool_err | PASS | 2 |
| relations.list path:id:empty | `relations.list` | tool_err | PASS | 4 |
| relations.list path:id:array | `relations.list` | tool_err | PASS | 3 |
| relations.list ghost:id | `relations.list` | fail | PASS | 16 |
| relations.list analyst reach | `relations.list` | reach | PASS | 16 |

### X.relations.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| relations.create ∅args→err | `relations.create` | tool_err | PASS | 3 |
| relations.create null:id | `relations.create` | tool_err | PASS | 2 |
| relations.create null:predicate | `relations.create` | tool_err | PASS | 2 |
| relations.create unknown arg | `relations.create` | tool_err | PASS | 3 |
| relations.create badtype:id | `relations.create` | fail | PASS | 3 |
| relations.create badtype:predicate | `relations.create` | fail | PASS | 14 |
| relations.create emptystr:predicate | `relations.create` | fail | PASS | 13 |
| relations.create path:id:slash | `relations.create` | tool_err | PASS | 4 |
| relations.create path:id:dotdot | `relations.create` | tool_err | PASS | 3 |
| relations.create path:id:empty | `relations.create` | tool_err | PASS | 2 |
| relations.create path:id:array | `relations.create` | tool_err | PASS | 2 |
| relations.create ghost:id | `relations.create` | fail | PASS | 16 |
| relations.create analyst deny | `relations.create` | deny | PASS | 3 |

### X.relations.delete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| relations.delete ∅args→err | `relations.delete` | tool_err | PASS | 3 |
| relations.delete null:id | `relations.delete` | tool_err | PASS | 3 |
| relations.delete null:relation_id | `relations.delete` | tool_err | PASS | 2 |
| relations.delete unknown arg | `relations.delete` | tool_err | PASS | 2 |
| relations.delete badtype:id | `relations.delete` | fail | PASS | 2 |
| relations.delete badtype:relation_id | `relations.delete` | fail | PASS | 4 |
| relations.delete path:id:slash | `relations.delete` | tool_err | PASS | 3 |
| relations.delete path:id:dotdot | `relations.delete` | tool_err | PASS | 3 |
| relations.delete path:id:empty | `relations.delete` | tool_err | PASS | 2 |
| relations.delete path:id:array | `relations.delete` | tool_err | PASS | 2 |
| relations.delete ghost:id | `relations.delete` | fail | PASS | 14 |
| relations.delete path:relation_id:slash | `relations.delete` | tool_err | PASS | 3 |
| relations.delete path:relation_id:dotdot | `relations.delete` | tool_err | PASS | 4 |
| relations.delete path:relation_id:empty | `relations.delete` | tool_err | PASS | 3 |
| relations.delete path:relation_id:array | `relations.delete` | tool_err | PASS | 2 |
| relations.delete ghost:relation_id | `relations.delete` | fail | PASS | 13 |
| relations.delete analyst deny | `relations.delete` | deny | PASS | 3 |

### X.documents.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| documents.list ∅args→err | `documents.list` | tool_err | PASS | 2 |
| documents.list null:id | `documents.list` | tool_err | PASS | 3 |
| documents.list unknown arg | `documents.list` | tool_err | PASS | 4 |
| documents.list badtype:id | `documents.list` | fail | PASS | 3 |
| documents.list path:id:slash | `documents.list` | tool_err | PASS | 2 |
| documents.list path:id:dotdot | `documents.list` | tool_err | PASS | 2 |
| documents.list path:id:empty | `documents.list` | tool_err | PASS | 2 |
| documents.list path:id:array | `documents.list` | tool_err | PASS | 3 |
| documents.list ghost:id | `documents.list` | fail | PASS | 15 |
| documents.list analyst reach | `documents.list` | reach | PASS | 16 |

### X.documents.upload

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| documents.upload ∅args→err | `documents.upload` | tool_err | PASS | 3 |
| documents.upload null:id | `documents.upload` | tool_err | PASS | 3 |
| documents.upload null:kind | `documents.upload` | tool_err | PASS | 3 |
| documents.upload null:file_name | `documents.upload` | tool_err | PASS | 2 |
| documents.upload unknown arg | `documents.upload` | tool_err | PASS | 2 |
| documents.upload badtype:id | `documents.upload` | fail | PASS | 4 |
| documents.upload badtype:kind | `documents.upload` | fail | PASS | 13 |
| documents.upload emptystr:kind | `documents.upload` | fail | PASS | 14 |
| documents.upload path:id:slash | `documents.upload` | tool_err | PASS | 4 |
| documents.upload path:id:dotdot | `documents.upload` | tool_err | PASS | 3 |
| documents.upload path:id:empty | `documents.upload` | tool_err | PASS | 2 |
| documents.upload path:id:array | `documents.upload` | tool_err | PASS | 2 |
| documents.upload ghost:id | `documents.upload` | fail | PASS | 13 |
| documents.upload analyst deny | `documents.upload` | deny | PASS | 3 |

### X.documents.delete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| documents.delete ∅args→err | `documents.delete` | tool_err | PASS | 2 |
| documents.delete null:id | `documents.delete` | tool_err | PASS | 3 |
| documents.delete null:doc_id | `documents.delete` | tool_err | PASS | 3 |
| documents.delete unknown arg | `documents.delete` | tool_err | PASS | 2 |
| documents.delete badtype:id | `documents.delete` | fail | PASS | 2 |
| documents.delete badtype:doc_id | `documents.delete` | fail | PASS | 2 |
| documents.delete path:id:slash | `documents.delete` | tool_err | PASS | 2 |
| documents.delete path:id:dotdot | `documents.delete` | tool_err | PASS | 3 |
| documents.delete path:id:empty | `documents.delete` | tool_err | PASS | 2 |
| documents.delete path:id:array | `documents.delete` | tool_err | PASS | 2 |
| documents.delete ghost:id | `documents.delete` | fail | PASS | 14 |
| documents.delete path:doc_id:slash | `documents.delete` | tool_err | PASS | 3 |
| documents.delete path:doc_id:dotdot | `documents.delete` | tool_err | PASS | 2 |
| documents.delete path:doc_id:empty | `documents.delete` | tool_err | PASS | 2 |
| documents.delete path:doc_id:array | `documents.delete` | tool_err | PASS | 3 |
| documents.delete ghost:doc_id | `documents.delete` | fail | PASS | 13 |
| documents.delete analyst deny | `documents.delete` | deny | PASS | 3 |

### X.files.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.get ∅args→err | `files.get` | tool_err | PASS | 3 |
| files.get null:id | `files.get` | tool_err | PASS | 2 |
| files.get unknown arg | `files.get` | tool_err | PASS | 2 |
| files.get badtype:id | `files.get` | fail | PASS | 2 |
| files.get path:id:slash | `files.get` | tool_err | PASS | 2 |
| files.get path:id:dotdot | `files.get` | tool_err | PASS | 3 |
| files.get path:id:empty | `files.get` | tool_err | PASS | 3 |
| files.get path:id:array | `files.get` | tool_err | PASS | 3 |
| files.get ghost:id | `files.get` | fail | PASS | 15 |
| files.get analyst deny | `files.get` | deny | PASS | 3 |

### X.files.delete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.delete ∅args→err | `files.delete` | tool_err | PASS | 2 |
| files.delete null:id | `files.delete` | tool_err | PASS | 4 |
| files.delete unknown arg | `files.delete` | tool_err | PASS | 2 |
| files.delete badtype:id | `files.delete` | fail | PASS | 2 |
| files.delete path:id:slash | `files.delete` | tool_err | PASS | 2 |
| files.delete path:id:dotdot | `files.delete` | tool_err | PASS | 2 |
| files.delete path:id:empty | `files.delete` | tool_err | PASS | 2 |
| files.delete path:id:array | `files.delete` | tool_err | PASS | 4 |
| files.delete ghost:id | `files.delete` | fail | PASS | 16 |
| files.delete analyst deny | `files.delete` | deny | PASS | 3 |

### X.files.upload_init

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.upload_init ∅args→err | `files.upload_init` | tool_err | PASS | 3 |
| files.upload_init null:filename | `files.upload_init` | tool_err | PASS | 2 |
| files.upload_init null:expected_size | `files.upload_init` | tool_err | PASS | 2 |
| files.upload_init null:expected_hash | `files.upload_init` | tool_err | PASS | 3 |
| files.upload_init unknown arg | `files.upload_init` | tool_err | PASS | 2 |
| files.upload_init badtype:filename | `files.upload_init` | fail | PASS | 13 |
| files.upload_init badtype:expected_size | `files.upload_init` | fail | PASS | 13 |
| files.upload_init emptystr:filename | `files.upload_init` | fail | PASS | 13 |
| files.upload_init analyst deny | `files.upload_init` | deny | PASS | 3 |

### X.files.upload_status

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.upload_status ∅args→err | `files.upload_status` | tool_err | PASS | 3 |
| files.upload_status null:id | `files.upload_status` | tool_err | PASS | 2 |
| files.upload_status unknown arg | `files.upload_status` | tool_err | PASS | 2 |
| files.upload_status badtype:id | `files.upload_status` | fail | PASS | 2 |
| files.upload_status path:id:slash | `files.upload_status` | tool_err | PASS | 2 |
| files.upload_status path:id:dotdot | `files.upload_status` | tool_err | PASS | 2 |
| files.upload_status path:id:empty | `files.upload_status` | tool_err | PASS | 3 |
| files.upload_status path:id:array | `files.upload_status` | tool_err | PASS | 2 |
| files.upload_status ghost:id | `files.upload_status` | fail | PASS | 16 |
| files.upload_status analyst deny | `files.upload_status` | deny | PASS | 3 |

### X.files.upload_abort

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.upload_abort ∅args→err | `files.upload_abort` | tool_err | PASS | 2 |
| files.upload_abort null:id | `files.upload_abort` | tool_err | PASS | 2 |
| files.upload_abort unknown arg | `files.upload_abort` | tool_err | PASS | 3 |
| files.upload_abort badtype:id | `files.upload_abort` | fail | PASS | 3 |
| files.upload_abort path:id:slash | `files.upload_abort` | tool_err | PASS | 2 |
| files.upload_abort path:id:dotdot | `files.upload_abort` | tool_err | PASS | 2 |
| files.upload_abort path:id:empty | `files.upload_abort` | tool_err | PASS | 2 |
| files.upload_abort path:id:array | `files.upload_abort` | tool_err | PASS | 2 |
| files.upload_abort ghost:id | `files.upload_abort` | fail | PASS | 17 |
| files.upload_abort analyst deny | `files.upload_abort` | deny | PASS | 4 |

### X.files.upload_chunk

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.upload_chunk ∅args→err | `files.upload_chunk` | tool_err | PASS | 3 |
| files.upload_chunk null:id | `files.upload_chunk` | tool_err | PASS | 3 |
| files.upload_chunk null:index | `files.upload_chunk` | tool_err | PASS | 2 |
| files.upload_chunk null:data_base64 | `files.upload_chunk` | tool_err | PASS | 3 |
| files.upload_chunk unknown arg | `files.upload_chunk` | tool_err | PASS | 3 |
| files.upload_chunk badtype:id | `files.upload_chunk` | fail | PASS | 3 |
| files.upload_chunk badtype:index | `files.upload_chunk` | fail | PASS | 2 |
| files.upload_chunk path:id:slash | `files.upload_chunk` | tool_err | PASS | 2 |
| files.upload_chunk path:id:dotdot | `files.upload_chunk` | tool_err | PASS | 2 |
| files.upload_chunk path:id:empty | `files.upload_chunk` | tool_err | PASS | 3 |
| files.upload_chunk path:id:array | `files.upload_chunk` | tool_err | PASS | 3 |
| files.upload_chunk ghost:id | `files.upload_chunk` | fail | PASS | 2 |
| files.upload_chunk path:index:slash | `files.upload_chunk` | tool_err | PASS | 2 |
| files.upload_chunk path:index:dotdot | `files.upload_chunk` | tool_err | PASS | 2 |
| files.upload_chunk path:index:empty | `files.upload_chunk` | tool_err | PASS | 4 |
| files.upload_chunk path:index:array | `files.upload_chunk` | tool_err | PASS | 3 |
| files.upload_chunk ghost:index | `files.upload_chunk` | fail | PASS | 3 |
| files.upload_chunk analyst deny | `files.upload_chunk` | deny | PASS | 2 |

### X.files.upload_complete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.upload_complete ∅args→err | `files.upload_complete` | tool_err | PASS | 2 |
| files.upload_complete null:id | `files.upload_complete` | tool_err | PASS | 4 |
| files.upload_complete null:final_hash | `files.upload_complete` | tool_err | PASS | 3 |
| files.upload_complete null:total_size | `files.upload_complete` | tool_err | PASS | 2 |
| files.upload_complete unknown arg | `files.upload_complete` | tool_err | PASS | 2 |
| files.upload_complete badtype:id | `files.upload_complete` | fail | PASS | 2 |
| files.upload_complete badtype:final_hash | `files.upload_complete` | fail | PASS | 14 |
| files.upload_complete emptystr:final_hash | `files.upload_complete` | fail | PASS | 13 |
| files.upload_complete path:id:slash | `files.upload_complete` | tool_err | PASS | 3 |
| files.upload_complete path:id:dotdot | `files.upload_complete` | tool_err | PASS | 3 |
| files.upload_complete path:id:empty | `files.upload_complete` | tool_err | PASS | 3 |
| files.upload_complete path:id:array | `files.upload_complete` | tool_err | PASS | 3 |
| files.upload_complete ghost:id | `files.upload_complete` | fail | PASS | 15 |
| files.upload_complete analyst deny | `files.upload_complete` | deny | PASS | 3 |

### X.files.download

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| files.download ∅args→err | `files.download` | tool_err | PASS | 2 |
| files.download null:id | `files.download` | tool_err | PASS | 2 |
| files.download unknown arg | `files.download` | tool_err | PASS | 2 |
| files.download badtype:id | `files.download` | fail | PASS | 3 |
| files.download path:id:slash | `files.download` | tool_err | PASS | 3 |
| files.download path:id:dotdot | `files.download` | tool_err | PASS | 2 |
| files.download path:id:empty | `files.download` | tool_err | PASS | 2 |
| files.download path:id:array | `files.download` | tool_err | PASS | 2 |
| files.download ghost:id | `files.download` | fail | PASS | 15 |
| files.download analyst deny | `files.download` | deny | PASS | 3 |

### X.documents.download

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| documents.download ∅args→err | `documents.download` | tool_err | PASS | 3 |
| documents.download null:id | `documents.download` | tool_err | PASS | 2 |
| documents.download null:doc_id | `documents.download` | tool_err | PASS | 2 |
| documents.download unknown arg | `documents.download` | tool_err | PASS | 2 |
| documents.download badtype:id | `documents.download` | fail | PASS | 2 |
| documents.download badtype:doc_id | `documents.download` | fail | PASS | 3 |
| documents.download path:id:slash | `documents.download` | tool_err | PASS | 3 |
| documents.download path:id:dotdot | `documents.download` | tool_err | PASS | 2 |
| documents.download path:id:empty | `documents.download` | tool_err | PASS | 2 |
| documents.download path:id:array | `documents.download` | tool_err | PASS | 2 |
| documents.download ghost:id | `documents.download` | fail | PASS | 14 |
| documents.download path:doc_id:slash | `documents.download` | tool_err | PASS | 3 |
| documents.download path:doc_id:dotdot | `documents.download` | tool_err | PASS | 3 |
| documents.download path:doc_id:empty | `documents.download` | tool_err | PASS | 3 |
| documents.download path:doc_id:array | `documents.download` | tool_err | PASS | 2 |
| documents.download ghost:doc_id | `documents.download` | fail | PASS | 14 |
| documents.download analyst reach | `documents.download` | reach | PASS | 16 |

### X.tasks.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| tasks.list ∅args→err | `tasks.list` | tool_err | PASS | 3 |
| tasks.list null:id | `tasks.list` | tool_err | PASS | 2 |
| tasks.list unknown arg | `tasks.list` | tool_err | PASS | 2 |
| tasks.list badtype:id | `tasks.list` | fail | PASS | 2 |
| tasks.list path:id:slash | `tasks.list` | tool_err | PASS | 3 |
| tasks.list path:id:dotdot | `tasks.list` | tool_err | PASS | 2 |
| tasks.list path:id:empty | `tasks.list` | tool_err | PASS | 2 |
| tasks.list path:id:array | `tasks.list` | tool_err | PASS | 2 |
| tasks.list ghost:id | `tasks.list` | fail | PASS | 16 |
| tasks.list analyst reach | `tasks.list` | reach | PASS | 15 |

### X.tasks.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| tasks.create ∅args→err | `tasks.create` | tool_err | PASS | 3 |
| tasks.create null:id | `tasks.create` | tool_err | PASS | 2 |
| tasks.create null:kind | `tasks.create` | tool_err | PASS | 3 |
| tasks.create unknown arg | `tasks.create` | tool_err | PASS | 2 |
| tasks.create badtype:id | `tasks.create` | fail | PASS | 2 |
| tasks.create badtype:kind | `tasks.create` | fail | PASS | 13 |
| tasks.create emptystr:kind | `tasks.create` | fail | PASS | 13 |
| tasks.create path:id:slash | `tasks.create` | tool_err | PASS | 3 |
| tasks.create path:id:dotdot | `tasks.create` | tool_err | PASS | 2 |
| tasks.create path:id:empty | `tasks.create` | tool_err | PASS | 2 |
| tasks.create path:id:array | `tasks.create` | tool_err | PASS | 2 |
| tasks.create ghost:id | `tasks.create` | fail | PASS | 14 |
| tasks.create analyst deny | `tasks.create` | deny | PASS | 3 |

### X.tasks.start

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| tasks.start ∅args→err | `tasks.start` | tool_err | PASS | 4 |
| tasks.start null:id | `tasks.start` | tool_err | PASS | 3 |
| tasks.start unknown arg | `tasks.start` | tool_err | PASS | 2 |
| tasks.start badtype:id | `tasks.start` | fail | PASS | 2 |
| tasks.start path:id:slash | `tasks.start` | tool_err | PASS | 2 |
| tasks.start path:id:dotdot | `tasks.start` | tool_err | PASS | 3 |
| tasks.start path:id:empty | `tasks.start` | tool_err | PASS | 3 |
| tasks.start path:id:array | `tasks.start` | tool_err | PASS | 3 |
| tasks.start ghost:id | `tasks.start` | fail | PASS | 16 |
| tasks.start analyst deny | `tasks.start` | deny | PASS | 3 |

### X.tasks.complete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| tasks.complete ∅args→err | `tasks.complete` | tool_err | PASS | 2 |
| tasks.complete null:id | `tasks.complete` | tool_err | PASS | 3 |
| tasks.complete unknown arg | `tasks.complete` | tool_err | PASS | 3 |
| tasks.complete badtype:id | `tasks.complete` | fail | PASS | 2 |
| tasks.complete path:id:slash | `tasks.complete` | tool_err | PASS | 2 |
| tasks.complete path:id:dotdot | `tasks.complete` | tool_err | PASS | 2 |
| tasks.complete path:id:empty | `tasks.complete` | tool_err | PASS | 2 |
| tasks.complete path:id:array | `tasks.complete` | tool_err | PASS | 4 |
| tasks.complete ghost:id | `tasks.complete` | fail | PASS | 16 |
| tasks.complete analyst deny | `tasks.complete` | deny | PASS | 3 |

### X.tasks.cancel

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| tasks.cancel ∅args→err | `tasks.cancel` | tool_err | PASS | 3 |
| tasks.cancel null:id | `tasks.cancel` | tool_err | PASS | 2 |
| tasks.cancel unknown arg | `tasks.cancel` | tool_err | PASS | 3 |
| tasks.cancel badtype:id | `tasks.cancel` | fail | PASS | 3 |
| tasks.cancel path:id:slash | `tasks.cancel` | tool_err | PASS | 3 |
| tasks.cancel path:id:dotdot | `tasks.cancel` | tool_err | PASS | 2 |
| tasks.cancel path:id:empty | `tasks.cancel` | tool_err | PASS | 2 |
| tasks.cancel path:id:array | `tasks.cancel` | tool_err | PASS | 2 |
| tasks.cancel ghost:id | `tasks.cancel` | fail | PASS | 16 |
| tasks.cancel analyst deny | `tasks.cancel` | deny | PASS | 4 |

### X.factors.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| factors.list ∅args→err | `factors.list` | tool_err | PASS | 3 |
| factors.list null:id | `factors.list` | tool_err | PASS | 2 |
| factors.list unknown arg | `factors.list` | tool_err | PASS | 2 |
| factors.list badtype:id | `factors.list` | fail | PASS | 2 |
| factors.list path:id:slash | `factors.list` | tool_err | PASS | 2 |
| factors.list path:id:dotdot | `factors.list` | tool_err | PASS | 3 |
| factors.list path:id:empty | `factors.list` | tool_err | PASS | 2 |
| factors.list path:id:array | `factors.list` | tool_err | PASS | 2 |
| factors.list ghost:id | `factors.list` | fail | PASS | 15 |
| factors.list analyst reach | `factors.list` | reach | PASS | 15 |

### X.factors.resolve

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| factors.resolve ∅args→err | `factors.resolve` | tool_err | PASS | 3 |
| factors.resolve null:id | `factors.resolve` | tool_err | PASS | 2 |
| factors.resolve null:factor_id | `factors.resolve` | tool_err | PASS | 2 |
| factors.resolve unknown arg | `factors.resolve` | tool_err | PASS | 3 |
| factors.resolve badtype:id | `factors.resolve` | fail | PASS | 3 |
| factors.resolve badtype:factor_id | `factors.resolve` | fail | PASS | 2 |
| factors.resolve path:id:slash | `factors.resolve` | tool_err | PASS | 2 |
| factors.resolve path:id:dotdot | `factors.resolve` | tool_err | PASS | 2 |
| factors.resolve path:id:empty | `factors.resolve` | tool_err | PASS | 2 |
| factors.resolve path:id:array | `factors.resolve` | tool_err | PASS | 3 |
| factors.resolve ghost:id | `factors.resolve` | fail | PASS | 13 |
| factors.resolve path:factor_id:slash | `factors.resolve` | tool_err | PASS | 3 |
| factors.resolve path:factor_id:dotdot | `factors.resolve` | tool_err | PASS | 3 |
| factors.resolve path:factor_id:empty | `factors.resolve` | tool_err | PASS | 2 |
| factors.resolve path:factor_id:array | `factors.resolve` | tool_err | PASS | 2 |
| factors.resolve ghost:factor_id | `factors.resolve` | fail | PASS | 16 |
| factors.resolve analyst deny | `factors.resolve` | deny | PASS | 3 |

### X.assessment.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| assessment.get ∅args→err | `assessment.get` | tool_err | PASS | 5 |
| assessment.get null:id | `assessment.get` | tool_err | PASS | 3 |
| assessment.get unknown arg | `assessment.get` | tool_err | PASS | 3 |
| assessment.get badtype:id | `assessment.get` | fail | PASS | 2 |
| assessment.get path:id:slash | `assessment.get` | tool_err | PASS | 3 |
| assessment.get path:id:dotdot | `assessment.get` | tool_err | PASS | 4 |
| assessment.get path:id:empty | `assessment.get` | tool_err | PASS | 4 |
| assessment.get path:id:array | `assessment.get` | tool_err | PASS | 3 |
| assessment.get ghost:id | `assessment.get` | fail | PASS | 16 |
| assessment.get analyst reach | `assessment.get` | reach | PASS | 15 |

### X.assessment.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| assessment.create ∅args→err | `assessment.create` | tool_err | PASS | 3 |
| assessment.create null:id | `assessment.create` | tool_err | PASS | 3 |
| assessment.create unknown arg | `assessment.create` | tool_err | PASS | 3 |
| assessment.create badtype:id | `assessment.create` | fail | PASS | 2 |
| assessment.create path:id:slash | `assessment.create` | tool_err | PASS | 2 |
| assessment.create path:id:dotdot | `assessment.create` | tool_err | PASS | 2 |
| assessment.create path:id:empty | `assessment.create` | tool_err | PASS | 2 |
| assessment.create path:id:array | `assessment.create` | tool_err | PASS | 3 |
| assessment.create ghost:id | `assessment.create` | fail | PASS | 17 |
| assessment.create analyst reach | `assessment.create` | reach | PASS | 16 |

### X.assessments.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| assessments.list ∅args | `assessments.list` | ok | PASS | 27 |
| assessments.list unknown arg | `assessments.list` | tool_err | PASS | 3 |
| assessments.list filter:status | `assessments.list` | ok | PASS | 14 |
| assessments.list null-opt:status | `assessments.list` | ok | PASS | 27 |
| assessments.list num:limit | `assessments.list` | ok | PASS | 16 |
| assessments.list null-opt:limit | `assessments.list` | ok | PASS | 26 |
| assessments.list filter:cursor | `assessments.list` | ok | PASS | 25 |
| assessments.list null-opt:cursor | `assessments.list` | ok | PASS | 27 |
| assessments.list analyst ok | `assessments.list` | ok | PASS | 24 |

### X.checks.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| checks.list ∅args | `checks.list` | ok | PASS | 27 |
| checks.list unknown arg | `checks.list` | tool_err | PASS | 3 |
| checks.list filter:target_entity_id | `checks.list` | ok | PASS | 27 |
| checks.list null-opt:target_entity_id | `checks.list` | ok | PASS | 29 |
| checks.list filter:status | `checks.list` | ok | PASS | 17 |
| checks.list null-opt:status | `checks.list` | ok | PASS | 27 |
| checks.list filter:kind | `checks.list` | ok | PASS | 16 |
| checks.list null-opt:kind | `checks.list` | ok | PASS | 27 |
| checks.list num:limit | `checks.list` | ok | PASS | 16 |
| checks.list null-opt:limit | `checks.list` | ok | PASS | 26 |
| checks.list filter:cursor | `checks.list` | ok | PASS | 27 |
| checks.list null-opt:cursor | `checks.list` | ok | PASS | 27 |
| checks.list analyst ok | `checks.list` | ok | PASS | 26 |

### X.checks.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| checks.get ∅args→err | `checks.get` | tool_err | PASS | 4 |
| checks.get null:id | `checks.get` | tool_err | PASS | 3 |
| checks.get unknown arg | `checks.get` | tool_err | PASS | 2 |
| checks.get badtype:id | `checks.get` | fail | PASS | 2 |
| checks.get path:id:slash | `checks.get` | tool_err | PASS | 2 |
| checks.get path:id:dotdot | `checks.get` | tool_err | PASS | 2 |
| checks.get path:id:empty | `checks.get` | tool_err | PASS | 3 |
| checks.get path:id:array | `checks.get` | tool_err | PASS | 2 |
| checks.get ghost:id | `checks.get` | fail | PASS | 15 |
| checks.get analyst reach | `checks.get` | reach | PASS | 16 |

### X.checks.evidence

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| checks.evidence ∅args→err | `checks.evidence` | tool_err | PASS | 3 |
| checks.evidence null:id | `checks.evidence` | tool_err | PASS | 2 |
| checks.evidence unknown arg | `checks.evidence` | tool_err | PASS | 2 |
| checks.evidence badtype:id | `checks.evidence` | fail | PASS | 2 |
| checks.evidence path:id:slash | `checks.evidence` | tool_err | PASS | 3 |
| checks.evidence path:id:dotdot | `checks.evidence` | tool_err | PASS | 3 |
| checks.evidence path:id:empty | `checks.evidence` | tool_err | PASS | 2 |
| checks.evidence path:id:array | `checks.evidence` | tool_err | PASS | 2 |
| checks.evidence ghost:id | `checks.evidence` | fail | PASS | 15 |
| checks.evidence analyst reach | `checks.evidence` | reach | PASS | 16 |

### X.checks.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| checks.create ∅args→err | `checks.create` | tool_err | PASS | 3 |
| checks.create null:target_entity_id | `checks.create` | tool_err | PASS | 4 |
| checks.create null:check_kind | `checks.create` | tool_err | PASS | 3 |
| checks.create unknown arg | `checks.create` | tool_err | PASS | 2 |
| checks.create badtype:target_entity_id | `checks.create` | fail | PASS | 14 |
| checks.create badtype:check_kind | `checks.create` | fail | PASS | 15 |
| checks.create emptystr:target_entity_id | `checks.create` | fail | PASS | 15 |
| checks.create emptystr:check_kind | `checks.create` | fail | PASS | 15 |
| checks.create analyst deny | `checks.create` | deny | PASS | 3 |

### X.entity_checks.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| entity_checks.list ∅args→err | `entity_checks.list` | tool_err | PASS | 2 |
| entity_checks.list null:id | `entity_checks.list` | tool_err | PASS | 2 |
| entity_checks.list unknown arg | `entity_checks.list` | tool_err | PASS | 2 |
| entity_checks.list badtype:id | `entity_checks.list` | fail | PASS | 3 |
| entity_checks.list path:id:slash | `entity_checks.list` | tool_err | PASS | 3 |
| entity_checks.list path:id:dotdot | `entity_checks.list` | tool_err | PASS | 3 |
| entity_checks.list path:id:empty | `entity_checks.list` | tool_err | PASS | 2 |
| entity_checks.list path:id:array | `entity_checks.list` | tool_err | PASS | 2 |
| entity_checks.list ghost:id | `entity_checks.list` | fail | PASS | 17 |
| entity_checks.list analyst reach | `entity_checks.list` | reach | PASS | 16 |

### X.schema.types

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.types ∅args | `schema.types` | ok | PASS | 27 |
| schema.types unknown arg | `schema.types` | tool_err | PASS | 3 |
| schema.types analyst ok | `schema.types` | ok | PASS | 27 |

### X.schema.type_get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.type_get ∅args→err | `schema.type_get` | tool_err | PASS | 3 |
| schema.type_get null:alias | `schema.type_get` | tool_err | PASS | 2 |
| schema.type_get unknown arg | `schema.type_get` | tool_err | PASS | 2 |
| schema.type_get badtype:alias | `schema.type_get` | fail | PASS | 3 |
| schema.type_get path:alias:slash | `schema.type_get` | tool_err | PASS | 3 |
| schema.type_get path:alias:dotdot | `schema.type_get` | tool_err | PASS | 2 |
| schema.type_get path:alias:empty | `schema.type_get` | tool_err | PASS | 2 |
| schema.type_get path:alias:array | `schema.type_get` | tool_err | PASS | 2 |
| schema.type_get ghost:alias | `schema.type_get` | fail | PASS | 20 |
| schema.type_get analyst reach | `schema.type_get` | reach | PASS | 22 |

### X.schema.type_view

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.type_view ∅args→err | `schema.type_view` | tool_err | PASS | 3 |
| schema.type_view null:alias | `schema.type_view` | tool_err | PASS | 2 |
| schema.type_view null:view_type | `schema.type_view` | tool_err | PASS | 2 |
| schema.type_view unknown arg | `schema.type_view` | tool_err | PASS | 2 |
| schema.type_view badtype:alias | `schema.type_view` | fail | PASS | 3 |
| schema.type_view badtype:view_type | `schema.type_view` | fail | PASS | 21 |
| schema.type_view emptystr:view_type | `schema.type_view` | fail | PASS | 19 |
| schema.type_view path:alias:slash | `schema.type_view` | tool_err | PASS | 3 |
| schema.type_view path:alias:dotdot | `schema.type_view` | tool_err | PASS | 2 |
| schema.type_view path:alias:empty | `schema.type_view` | tool_err | PASS | 3 |
| schema.type_view path:alias:array | `schema.type_view` | tool_err | PASS | 2 |
| schema.type_view ghost:alias | `schema.type_view` | fail | PASS | 20 |
| schema.type_view analyst reach | `schema.type_view` | reach | PASS | 20 |

### X.schema.predicates

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.predicates ∅args | `schema.predicates` | ok | PASS | 22 |
| schema.predicates unknown arg | `schema.predicates` | tool_err | PASS | 3 |
| schema.predicates analyst ok | `schema.predicates` | ok | PASS | 20 |

### X.schema.published

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.published ∅args | `schema.published` | ok | PASS | 47 |
| schema.published unknown arg | `schema.published` | tool_err | PASS | 4 |
| schema.published analyst ok | `schema.published` | ok | PASS | 30 |

### X.schema.drafts

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.drafts ∅args | `schema.drafts` | ok | PASS | 24 |
| schema.drafts unknown arg | `schema.drafts` | tool_err | PASS | 4 |
| schema.drafts analyst deny | `schema.drafts` | deny | PASS | 2 |

### X.schema.draft_get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.draft_get ∅args→err | `schema.draft_get` | tool_err | PASS | 2 |
| schema.draft_get null:id | `schema.draft_get` | tool_err | PASS | 2 |
| schema.draft_get unknown arg | `schema.draft_get` | tool_err | PASS | 2 |
| schema.draft_get badtype:id | `schema.draft_get` | fail | PASS | 4 |
| schema.draft_get path:id:slash | `schema.draft_get` | tool_err | PASS | 2 |
| schema.draft_get path:id:dotdot | `schema.draft_get` | tool_err | PASS | 2 |
| schema.draft_get path:id:empty | `schema.draft_get` | tool_err | PASS | 2 |
| schema.draft_get path:id:array | `schema.draft_get` | tool_err | PASS | 2 |
| schema.draft_get ghost:id | `schema.draft_get` | fail | PASS | 18 |
| schema.draft_get analyst deny | `schema.draft_get` | deny | PASS | 4 |

### X.schema.draft_create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.draft_create ∅args→err | `schema.draft_create` | tool_err | PASS | 3 |
| schema.draft_create null:name | `schema.draft_create` | tool_err | PASS | 2 |
| schema.draft_create unknown arg | `schema.draft_create` | tool_err | PASS | 2 |
| schema.draft_create badtype:name | `schema.draft_create` | fail | PASS | 14 |
| schema.draft_create emptystr:name | `schema.draft_create` | fail | PASS | 13 |
| schema.draft_create analyst deny | `schema.draft_create` | deny | PASS | 3 |

### X.schema.draft_update

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.draft_update ∅args→err | `schema.draft_update` | tool_err | PASS | 2 |
| schema.draft_update null:id | `schema.draft_update` | tool_err | PASS | 3 |
| schema.draft_update null:display_name | `schema.draft_update` | tool_err | PASS | 4 |
| schema.draft_update null:semantic_version | `schema.draft_update` | tool_err | PASS | 3 |
| schema.draft_update unknown arg | `schema.draft_update` | tool_err | PASS | 2 |
| schema.draft_update badtype:id | `schema.draft_update` | fail | PASS | 2 |
| schema.draft_update badtype:display_name | `schema.draft_update` | fail | PASS | 14 |
| schema.draft_update emptystr:display_name | `schema.draft_update` | fail | PASS | 19 |
| schema.draft_update path:id:slash | `schema.draft_update` | tool_err | PASS | 3 |
| schema.draft_update path:id:dotdot | `schema.draft_update` | tool_err | PASS | 3 |
| schema.draft_update path:id:empty | `schema.draft_update` | tool_err | PASS | 2 |
| schema.draft_update path:id:array | `schema.draft_update` | tool_err | PASS | 2 |
| schema.draft_update ghost:id | `schema.draft_update` | fail | PASS | 18 |
| schema.draft_update analyst deny | `schema.draft_update` | deny | PASS | 3 |

### X.schema.draft_delete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.draft_delete ∅args→err | `schema.draft_delete` | tool_err | PASS | 3 |
| schema.draft_delete null:id | `schema.draft_delete` | tool_err | PASS | 3 |
| schema.draft_delete unknown arg | `schema.draft_delete` | tool_err | PASS | 2 |
| schema.draft_delete badtype:id | `schema.draft_delete` | fail | PASS | 2 |
| schema.draft_delete path:id:slash | `schema.draft_delete` | tool_err | PASS | 2 |
| schema.draft_delete path:id:dotdot | `schema.draft_delete` | tool_err | PASS | 2 |
| schema.draft_delete path:id:empty | `schema.draft_delete` | tool_err | PASS | 2 |
| schema.draft_delete path:id:array | `schema.draft_delete` | tool_err | PASS | 3 |
| schema.draft_delete ghost:id | `schema.draft_delete` | fail | PASS | 19 |
| schema.draft_delete analyst deny | `schema.draft_delete` | deny | PASS | 3 |

### X.schema.draft_validate

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.draft_validate ∅args→err | `schema.draft_validate` | tool_err | PASS | 2 |
| schema.draft_validate null:id | `schema.draft_validate` | tool_err | PASS | 3 |
| schema.draft_validate unknown arg | `schema.draft_validate` | tool_err | PASS | 3 |
| schema.draft_validate badtype:id | `schema.draft_validate` | fail | PASS | 2 |
| schema.draft_validate path:id:slash | `schema.draft_validate` | tool_err | PASS | 2 |
| schema.draft_validate path:id:dotdot | `schema.draft_validate` | tool_err | PASS | 2 |
| schema.draft_validate path:id:empty | `schema.draft_validate` | tool_err | PASS | 2 |
| schema.draft_validate path:id:array | `schema.draft_validate` | tool_err | PASS | 3 |
| schema.draft_validate ghost:id | `schema.draft_validate` | fail | PASS | 20 |
| schema.draft_validate analyst deny | `schema.draft_validate` | deny | PASS | 3 |

### X.schema.draft_impact

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.draft_impact ∅args→err | `schema.draft_impact` | tool_err | PASS | 2 |
| schema.draft_impact null:id | `schema.draft_impact` | tool_err | PASS | 2 |
| schema.draft_impact unknown arg | `schema.draft_impact` | tool_err | PASS | 4 |
| schema.draft_impact badtype:id | `schema.draft_impact` | fail | PASS | 3 |
| schema.draft_impact path:id:slash | `schema.draft_impact` | tool_err | PASS | 2 |
| schema.draft_impact path:id:dotdot | `schema.draft_impact` | tool_err | PASS | 2 |
| schema.draft_impact path:id:empty | `schema.draft_impact` | tool_err | PASS | 2 |
| schema.draft_impact path:id:array | `schema.draft_impact` | tool_err | PASS | 3 |
| schema.draft_impact ghost:id | `schema.draft_impact` | fail | PASS | 19 |
| schema.draft_impact analyst deny | `schema.draft_impact` | deny | PASS | 3 |

### X.schema.draft_publish

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.draft_publish ∅args→err | `schema.draft_publish` | tool_err | PASS | 2 |
| schema.draft_publish null:id | `schema.draft_publish` | tool_err | PASS | 2 |
| schema.draft_publish null:reason | `schema.draft_publish` | tool_err | PASS | 2 |
| schema.draft_publish unknown arg | `schema.draft_publish` | tool_err | PASS | 3 |
| schema.draft_publish badtype:id | `schema.draft_publish` | fail | PASS | 3 |
| schema.draft_publish badtype:reason | `schema.draft_publish` | fail | PASS | 13 |
| schema.draft_publish emptystr:reason | `schema.draft_publish` | fail | PASS | 12 |
| schema.draft_publish path:id:slash | `schema.draft_publish` | tool_err | PASS | 4 |
| schema.draft_publish path:id:dotdot | `schema.draft_publish` | tool_err | PASS | 3 |
| schema.draft_publish path:id:empty | `schema.draft_publish` | tool_err | PASS | 2 |
| schema.draft_publish path:id:array | `schema.draft_publish` | tool_err | PASS | 2 |
| schema.draft_publish ghost:id | `schema.draft_publish` | fail | PASS | 13 |
| schema.draft_publish analyst deny | `schema.draft_publish` | deny | PASS | 3 |

### X.schema.revision_get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.revision_get ∅args→err | `schema.revision_get` | tool_err | PASS | 3 |
| schema.revision_get null:id | `schema.revision_get` | tool_err | PASS | 3 |
| schema.revision_get unknown arg | `schema.revision_get` | tool_err | PASS | 3 |
| schema.revision_get badtype:id | `schema.revision_get` | fail | PASS | 2 |
| schema.revision_get path:id:slash | `schema.revision_get` | tool_err | PASS | 2 |
| schema.revision_get path:id:dotdot | `schema.revision_get` | tool_err | PASS | 2 |
| schema.revision_get path:id:empty | `schema.revision_get` | tool_err | PASS | 2 |
| schema.revision_get path:id:array | `schema.revision_get` | tool_err | PASS | 3 |
| schema.revision_get ghost:id | `schema.revision_get` | fail | PASS | 20 |
| schema.revision_get analyst reach | `schema.revision_get` | reach | PASS | 20 |

### X.schema.revision_retire

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.revision_retire ∅args→err | `schema.revision_retire` | tool_err | PASS | 3 |
| schema.revision_retire null:id | `schema.revision_retire` | tool_err | PASS | 3 |
| schema.revision_retire unknown arg | `schema.revision_retire` | tool_err | PASS | 3 |
| schema.revision_retire badtype:id | `schema.revision_retire` | fail | PASS | 2 |
| schema.revision_retire path:id:slash | `schema.revision_retire` | tool_err | PASS | 2 |
| schema.revision_retire path:id:dotdot | `schema.revision_retire` | tool_err | PASS | 2 |
| schema.revision_retire path:id:empty | `schema.revision_retire` | tool_err | PASS | 2 |
| schema.revision_retire path:id:array | `schema.revision_retire` | tool_err | PASS | 3 |
| schema.revision_retire ghost:id | `schema.revision_retire` | fail | PASS | 13 |
| schema.revision_retire analyst deny | `schema.revision_retire` | deny | PASS | 4 |

### X.schema.revision_migration

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| schema.revision_migration ∅args→err | `schema.revision_migration` | tool_err | PASS | 3 |
| schema.revision_migration null:id | `schema.revision_migration` | tool_err | PASS | 3 |
| schema.revision_migration unknown arg | `schema.revision_migration` | tool_err | PASS | 2 |
| schema.revision_migration badtype:id | `schema.revision_migration` | fail | PASS | 2 |
| schema.revision_migration path:id:slash | `schema.revision_migration` | tool_err | PASS | 3 |
| schema.revision_migration path:id:dotdot | `schema.revision_migration` | tool_err | PASS | 3 |
| schema.revision_migration path:id:empty | `schema.revision_migration` | tool_err | PASS | 3 |
| schema.revision_migration path:id:array | `schema.revision_migration` | tool_err | PASS | 2 |
| schema.revision_migration ghost:id | `schema.revision_migration` | fail | PASS | 21 |
| schema.revision_migration analyst reach | `schema.revision_migration` | reach | PASS | 21 |

### X.requests.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| requests.list ∅args | `requests.list` | ok | PASS | 28 |
| requests.list unknown arg | `requests.list` | tool_err | PASS | 4 |
| requests.list filter:status | `requests.list` | ok | PASS | 16 |
| requests.list null-opt:status | `requests.list` | ok | PASS | 26 |
| requests.list num:limit | `requests.list` | ok | PASS | 16 |
| requests.list null-opt:limit | `requests.list` | ok | PASS | 27 |
| requests.list filter:cursor | `requests.list` | ok | PASS | 25 |
| requests.list null-opt:cursor | `requests.list` | ok | PASS | 27 |
| requests.list analyst ok | `requests.list` | ok | PASS | 26 |

### X.requests.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| requests.create ∅args→err | `requests.create` | tool_err | PASS | 3 |
| requests.create null:target_entity_id | `requests.create` | tool_err | PASS | 2 |
| requests.create null:requested_product | `requests.create` | tool_err | PASS | 2 |
| requests.create unknown arg | `requests.create` | tool_err | PASS | 2 |
| requests.create badtype:target_entity_id | `requests.create` | fail | PASS | 13 |
| requests.create badtype:requested_product | `requests.create` | fail | PASS | 13 |
| requests.create emptystr:target_entity_id | `requests.create` | fail | PASS | 13 |
| requests.create emptystr:requested_product | `requests.create` | fail | PASS | 12 |
| requests.create analyst deny | `requests.create` | deny | PASS | 4 |

### X.workflows.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| workflows.get ∅args→err | `workflows.get` | tool_err | PASS | 3 |
| workflows.get null:id | `workflows.get` | tool_err | PASS | 2 |
| workflows.get unknown arg | `workflows.get` | tool_err | PASS | 2 |
| workflows.get badtype:id | `workflows.get` | fail | PASS | 2 |
| workflows.get path:id:slash | `workflows.get` | tool_err | PASS | 4 |
| workflows.get path:id:dotdot | `workflows.get` | tool_err | PASS | 3 |
| workflows.get path:id:empty | `workflows.get` | tool_err | PASS | 2 |
| workflows.get path:id:array | `workflows.get` | tool_err | PASS | 2 |
| workflows.get ghost:id | `workflows.get` | fail | PASS | 16 |
| workflows.get analyst reach | `workflows.get` | reach | PASS | 16 |

### X.workflows.tasks

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| workflows.tasks ∅args→err | `workflows.tasks` | tool_err | PASS | 3 |
| workflows.tasks null:id | `workflows.tasks` | tool_err | PASS | 4 |
| workflows.tasks unknown arg | `workflows.tasks` | tool_err | PASS | 3 |
| workflows.tasks badtype:id | `workflows.tasks` | fail | PASS | 3 |
| workflows.tasks path:id:slash | `workflows.tasks` | tool_err | PASS | 2 |
| workflows.tasks path:id:dotdot | `workflows.tasks` | tool_err | PASS | 2 |
| workflows.tasks path:id:empty | `workflows.tasks` | tool_err | PASS | 3 |
| workflows.tasks path:id:array | `workflows.tasks` | tool_err | PASS | 3 |
| workflows.tasks ghost:id | `workflows.tasks` | fail | PASS | 16 |
| workflows.tasks analyst reach | `workflows.tasks` | reach | PASS | 15 |

### X.workflows.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| workflows.create ∅args→err | `workflows.create` | tool_err | PASS | 3 |
| workflows.create null:definition_id | `workflows.create` | tool_err | PASS | 2 |
| workflows.create null:target_entity_id | `workflows.create` | tool_err | PASS | 2 |
| workflows.create unknown arg | `workflows.create` | tool_err | PASS | 2 |
| workflows.create badtype:definition_id | `workflows.create` | fail | PASS | 13 |
| workflows.create badtype:target_entity_id | `workflows.create` | fail | PASS | 15 |
| workflows.create emptystr:definition_id | `workflows.create` | fail | PASS | 14 |
| workflows.create emptystr:target_entity_id | `workflows.create` | fail | PASS | 15 |
| workflows.create analyst deny | `workflows.create` | deny | PASS | 3 |

### X.workflows.transition

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| workflows.transition ∅args→err | `workflows.transition` | tool_err | PASS | 2 |
| workflows.transition null:id | `workflows.transition` | tool_err | PASS | 2 |
| workflows.transition null:trigger | `workflows.transition` | tool_err | PASS | 3 |
| workflows.transition null:to | `workflows.transition` | tool_err | PASS | 2 |
| workflows.transition unknown arg | `workflows.transition` | tool_err | PASS | 2 |
| workflows.transition badtype:id | `workflows.transition` | fail | PASS | 2 |
| workflows.transition badtype:trigger | `workflows.transition` | fail | PASS | 13 |
| workflows.transition emptystr:trigger | `workflows.transition` | fail | PASS | 14 |
| workflows.transition path:id:slash | `workflows.transition` | tool_err | PASS | 3 |
| workflows.transition path:id:dotdot | `workflows.transition` | tool_err | PASS | 2 |
| workflows.transition path:id:empty | `workflows.transition` | tool_err | PASS | 2 |
| workflows.transition path:id:array | `workflows.transition` | tool_err | PASS | 3 |
| workflows.transition ghost:id | `workflows.transition` | fail | PASS | 16 |
| workflows.transition analyst deny | `workflows.transition` | deny | PASS | 3 |

### X.monitors.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| monitors.list ∅args | `monitors.list` | ok | PASS | 15 |
| monitors.list unknown arg | `monitors.list` | tool_err | PASS | 3 |
| monitors.list analyst deny | `monitors.list` | deny | PASS | 2 |

### X.monitors.run

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| monitors.run ∅args→err | `monitors.run` | tool_err | PASS | 2 |
| monitors.run null:id | `monitors.run` | tool_err | PASS | 2 |
| monitors.run unknown arg | `monitors.run` | tool_err | PASS | 5 |
| monitors.run badtype:id | `monitors.run` | fail | PASS | 3 |
| monitors.run path:id:slash | `monitors.run` | tool_err | PASS | 2 |
| monitors.run path:id:dotdot | `monitors.run` | tool_err | PASS | 2 |
| monitors.run path:id:empty | `monitors.run` | tool_err | PASS | 2 |
| monitors.run path:id:array | `monitors.run` | tool_err | PASS | 2 |
| monitors.run ghost:id | `monitors.run` | fail | PASS | 15 |
| monitors.run analyst deny | `monitors.run` | deny | PASS | 4 |

### X.reports.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| reports.list ∅args | `reports.list` | ok | PASS | 16 |
| reports.list unknown arg | `reports.list` | tool_err | PASS | 3 |
| reports.list analyst ok | `reports.list` | ok | PASS | 17 |

### X.reports.generate

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| reports.generate ∅args→err | `reports.generate` | tool_err | PASS | 3 |
| reports.generate null:entity_id | `reports.generate` | tool_err | PASS | 2 |
| reports.generate unknown arg | `reports.generate` | tool_err | PASS | 3 |
| reports.generate badtype:entity_id | `reports.generate` | fail | PASS | 13 |
| reports.generate emptystr:entity_id | `reports.generate` | fail | PASS | 13 |
| reports.generate analyst deny | `reports.generate` | deny | PASS | 3 |

### X.connectors.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.list ∅args | `connectors.list` | ok | PASS | 15 |
| connectors.list unknown arg | `connectors.list` | tool_err | PASS | 4 |
| connectors.list analyst deny | `connectors.list` | deny | PASS | 3 |

### X.connectors.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.get ∅args→err | `connectors.get` | tool_err | PASS | 2 |
| connectors.get null:id | `connectors.get` | tool_err | PASS | 2 |
| connectors.get unknown arg | `connectors.get` | tool_err | PASS | 3 |
| connectors.get badtype:id | `connectors.get` | fail | PASS | 2 |
| connectors.get path:id:slash | `connectors.get` | tool_err | PASS | 3 |
| connectors.get path:id:dotdot | `connectors.get` | tool_err | PASS | 3 |
| connectors.get path:id:empty | `connectors.get` | tool_err | PASS | 2 |
| connectors.get path:id:array | `connectors.get` | tool_err | PASS | 2 |
| connectors.get ghost:id | `connectors.get` | fail | PASS | 16 |
| connectors.get analyst deny | `connectors.get` | deny | PASS | 3 |

### X.connectors.update

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.update ∅args→err | `connectors.update` | tool_err | PASS | 4 |
| connectors.update null:id | `connectors.update` | tool_err | PASS | 3 |
| connectors.update unknown arg | `connectors.update` | tool_err | PASS | 2 |
| connectors.update badtype:id | `connectors.update` | fail | PASS | 2 |
| connectors.update path:id:slash | `connectors.update` | tool_err | PASS | 2 |
| connectors.update path:id:dotdot | `connectors.update` | tool_err | PASS | 2 |
| connectors.update path:id:empty | `connectors.update` | tool_err | PASS | 3 |
| connectors.update path:id:array | `connectors.update` | tool_err | PASS | 3 |
| connectors.update ghost:id | `connectors.update` | fail | PASS | 16 |
| connectors.update analyst deny | `connectors.update` | deny | PASS | 3 |

### X.connectors.delete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.delete ∅args→err | `connectors.delete` | tool_err | PASS | 2 |
| connectors.delete null:id | `connectors.delete` | tool_err | PASS | 2 |
| connectors.delete unknown arg | `connectors.delete` | tool_err | PASS | 2 |
| connectors.delete badtype:id | `connectors.delete` | fail | PASS | 3 |
| connectors.delete path:id:slash | `connectors.delete` | tool_err | PASS | 3 |
| connectors.delete path:id:dotdot | `connectors.delete` | tool_err | PASS | 2 |
| connectors.delete path:id:empty | `connectors.delete` | tool_err | PASS | 2 |
| connectors.delete path:id:array | `connectors.delete` | tool_err | PASS | 2 |
| connectors.delete ghost:id | `connectors.delete` | fail | PASS | 16 |
| connectors.delete analyst deny | `connectors.delete` | deny | PASS | 3 |

### X.connectors.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.create ∅args→err | `connectors.create` | tool_err | PASS | 3 |
| connectors.create null:connector_type_id | `connectors.create` | tool_err | PASS | 3 |
| connectors.create unknown arg | `connectors.create` | tool_err | PASS | 2 |
| connectors.create badtype:connector_type_id | `connectors.create` | fail | PASS | 13 |
| connectors.create emptystr:connector_type_id | `connectors.create` | fail | PASS | 14 |
| connectors.create analyst deny | `connectors.create` | deny | PASS | 3 |

### X.connectors.enable

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.enable ∅args→err | `connectors.enable` | tool_err | PASS | 2 |
| connectors.enable null:id | `connectors.enable` | tool_err | PASS | 2 |
| connectors.enable unknown arg | `connectors.enable` | tool_err | PASS | 3 |
| connectors.enable badtype:id | `connectors.enable` | fail | PASS | 3 |
| connectors.enable path:id:slash | `connectors.enable` | tool_err | PASS | 3 |
| connectors.enable path:id:dotdot | `connectors.enable` | tool_err | PASS | 2 |
| connectors.enable path:id:empty | `connectors.enable` | tool_err | PASS | 2 |
| connectors.enable path:id:array | `connectors.enable` | tool_err | PASS | 2 |
| connectors.enable ghost:id | `connectors.enable` | fail | PASS | 16 |
| connectors.enable analyst deny | `connectors.enable` | deny | PASS | 4 |

### X.connectors.disable

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.disable ∅args→err | `connectors.disable` | tool_err | PASS | 3 |
| connectors.disable null:id | `connectors.disable` | tool_err | PASS | 2 |
| connectors.disable unknown arg | `connectors.disable` | tool_err | PASS | 2 |
| connectors.disable badtype:id | `connectors.disable` | fail | PASS | 2 |
| connectors.disable path:id:slash | `connectors.disable` | tool_err | PASS | 3 |
| connectors.disable path:id:dotdot | `connectors.disable` | tool_err | PASS | 3 |
| connectors.disable path:id:empty | `connectors.disable` | tool_err | PASS | 2 |
| connectors.disable path:id:array | `connectors.disable` | tool_err | PASS | 2 |
| connectors.disable ghost:id | `connectors.disable` | fail | PASS | 15 |
| connectors.disable analyst deny | `connectors.disable` | deny | PASS | 3 |

### X.connectors.health

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| connectors.health ∅args→err | `connectors.health` | tool_err | PASS | 2 |
| connectors.health null:id | `connectors.health` | tool_err | PASS | 3 |
| connectors.health unknown arg | `connectors.health` | tool_err | PASS | 3 |
| connectors.health badtype:id | `connectors.health` | fail | PASS | 3 |
| connectors.health path:id:slash | `connectors.health` | tool_err | PASS | 2 |
| connectors.health path:id:dotdot | `connectors.health` | tool_err | PASS | 2 |
| connectors.health path:id:empty | `connectors.health` | tool_err | PASS | 3 |
| connectors.health path:id:array | `connectors.health` | tool_err | PASS | 3 |
| connectors.health ghost:id | `connectors.health` | fail | PASS | 16 |
| connectors.health analyst deny | `connectors.health` | deny | PASS | 3 |

### X.audit.query

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| audit.query ∅args | `audit.query` | ok | PASS | 22 |
| audit.query unknown arg | `audit.query` | tool_err | PASS | 3 |
| audit.query filter:kind | `audit.query` | ok | PASS | 15 |
| audit.query null-opt:kind | `audit.query` | ok | PASS | 22 |
| audit.query filter:actor | `audit.query` | ok | PASS | 15 |
| audit.query null-opt:actor | `audit.query` | ok | PASS | 21 |
| audit.query filter:action | `audit.query` | ok | PASS | 16 |
| audit.query null-opt:action | `audit.query` | ok | PASS | 23 |
| audit.query filter:resource_kind | `audit.query` | ok | PASS | 15 |
| audit.query null-opt:resource_kind | `audit.query` | ok | PASS | 22 |
| audit.query filter:resource_id | `audit.query` | ok | PASS | 23 |
| audit.query null-opt:resource_id | `audit.query` | ok | PASS | 22 |
| audit.query filter:effect | `audit.query` | ok | PASS | 15 |
| audit.query null-opt:effect | `audit.query` | ok | PASS | 23 |
| audit.query filter:from | `audit.query` | ok | PASS | 21 |
| audit.query null-opt:from | `audit.query` | ok | PASS | 21 |
| audit.query filter:to | `audit.query` | ok | PASS | 21 |
| audit.query null-opt:to | `audit.query` | ok | PASS | 21 |
| audit.query num:limit | `audit.query` | ok | PASS | 19 |
| audit.query null-opt:limit | `audit.query` | ok | PASS | 21 |
| audit.query filter:cursor | `audit.query` | ok | PASS | 22 |
| audit.query null-opt:cursor | `audit.query` | ok | PASS | 22 |
| audit.query analyst deny | `audit.query` | deny | PASS | 3 |

### X.audit.checkpoint_get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| audit.checkpoint_get ∅args | `audit.checkpoint_get` | ok | PASS | 15 |
| audit.checkpoint_get unknown arg | `audit.checkpoint_get` | tool_err | PASS | 3 |
| audit.checkpoint_get analyst deny | `audit.checkpoint_get` | deny | PASS | 3 |

### X.audit.checkpoint_create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| audit.checkpoint_create ∅args | `audit.checkpoint_create` | ok | PASS | 17 |
| audit.checkpoint_create unknown arg | `audit.checkpoint_create` | tool_err | PASS | 3 |
| audit.checkpoint_create analyst deny | `audit.checkpoint_create` | deny | PASS | 2 |

### X.audit.verify

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| audit.verify ∅args | `audit.verify` | ok | PASS | 26 |
| audit.verify unknown arg | `audit.verify` | tool_err | PASS | 3 |
| audit.verify analyst deny | `audit.verify` | deny | PASS | 2 |

### X.audit.retention_get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| audit.retention_get ∅args | `audit.retention_get` | ok | PASS | 15 |
| audit.retention_get unknown arg | `audit.retention_get` | tool_err | PASS | 3 |
| audit.retention_get analyst deny | `audit.retention_get` | deny | PASS | 2 |

### X.audit.retention_set

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| audit.retention_set ∅args→err | `audit.retention_set` | tool_err | PASS | 3 |
| audit.retention_set null:retention_days | `audit.retention_set` | tool_err | PASS | 3 |
| audit.retention_set unknown arg | `audit.retention_set` | tool_err | PASS | 3 |
| audit.retention_set badtype:retention_days | `audit.retention_set` | fail | PASS | 14 |
| audit.retention_set analyst deny | `audit.retention_set` | deny | PASS | 3 |

### X.permissions.groups

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| permissions.groups ∅args | `permissions.groups` | ok | PASS | 17 |
| permissions.groups unknown arg | `permissions.groups` | tool_err | PASS | 3 |
| permissions.groups analyst deny | `permissions.groups` | deny | PASS | 2 |

### X.permissions.group_create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| permissions.group_create ∅args→err | `permissions.group_create` | tool_err | PASS | 4 |
| permissions.group_create null:name | `permissions.group_create` | tool_err | PASS | 3 |
| permissions.group_create null:class | `permissions.group_create` | tool_err | PASS | 2 |
| permissions.group_create unknown arg | `permissions.group_create` | tool_err | PASS | 2 |
| permissions.group_create badtype:name | `permissions.group_create` | fail | PASS | 13 |
| permissions.group_create badtype:class | `permissions.group_create` | fail | PASS | 13 |
| permissions.group_create emptystr:name | `permissions.group_create` | fail | PASS | 13 |
| permissions.group_create emptystr:class | `permissions.group_create` | fail | PASS | 13 |
| permissions.group_create analyst deny | `permissions.group_create` | deny | PASS | 4 |

### X.permissions.group_update

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| permissions.group_update ∅args→err | `permissions.group_update` | tool_err | PASS | 3 |
| permissions.group_update null:id | `permissions.group_update` | tool_err | PASS | 2 |
| permissions.group_update null:name | `permissions.group_update` | tool_err | PASS | 2 |
| permissions.group_update unknown arg | `permissions.group_update` | tool_err | PASS | 2 |
| permissions.group_update badtype:id | `permissions.group_update` | fail | PASS | 3 |
| permissions.group_update badtype:name | `permissions.group_update` | fail | PASS | 13 |
| permissions.group_update emptystr:name | `permissions.group_update` | fail | PASS | 13 |
| permissions.group_update path:id:slash | `permissions.group_update` | tool_err | PASS | 3 |
| permissions.group_update path:id:dotdot | `permissions.group_update` | tool_err | PASS | 4 |
| permissions.group_update path:id:empty | `permissions.group_update` | tool_err | PASS | 3 |
| permissions.group_update path:id:array | `permissions.group_update` | tool_err | PASS | 3 |
| permissions.group_update ghost:id | `permissions.group_update` | fail | PASS | 16 |
| permissions.group_update analyst deny | `permissions.group_update` | deny | PASS | 3 |

### X.permissions.group_delete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| permissions.group_delete ∅args→err | `permissions.group_delete` | tool_err | PASS | 2 |
| permissions.group_delete null:id | `permissions.group_delete` | tool_err | PASS | 4 |
| permissions.group_delete unknown arg | `permissions.group_delete` | tool_err | PASS | 2 |
| permissions.group_delete badtype:id | `permissions.group_delete` | fail | PASS | 2 |
| permissions.group_delete path:id:slash | `permissions.group_delete` | tool_err | PASS | 3 |
| permissions.group_delete path:id:dotdot | `permissions.group_delete` | tool_err | PASS | 2 |
| permissions.group_delete path:id:empty | `permissions.group_delete` | tool_err | PASS | 3 |
| permissions.group_delete path:id:array | `permissions.group_delete` | tool_err | PASS | 3 |
| permissions.group_delete ghost:id | `permissions.group_delete` | fail | PASS | 16 |
| permissions.group_delete analyst deny | `permissions.group_delete` | deny | PASS | 3 |

### X.users.role_groups

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.role_groups ∅args | `users.role_groups` | ok | PASS | 15 |
| users.role_groups unknown arg | `users.role_groups` | tool_err | PASS | 3 |
| users.role_groups analyst deny | `users.role_groups` | deny | PASS | 2 |

### X.users.list

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.list ∅args | `users.list` | ok | PASS | 22 |
| users.list unknown arg | `users.list` | tool_err | PASS | 3 |
| users.list filter:q | `users.list` | ok | PASS | 16 |
| users.list null-opt:q | `users.list` | ok | PASS | 20 |
| users.list filter:group_id | `users.list` | ok | PASS | 20 |
| users.list null-opt:group_id | `users.list` | ok | PASS | 21 |
| users.list filter:inherited | `users.list` | ok | PASS | 19 |
| users.list null-opt:inherited | `users.list` | ok | PASS | 20 |
| users.list filter:ip | `users.list` | ok | PASS | 17 |
| users.list null-opt:ip | `users.list` | ok | PASS | 20 |
| users.list filter:status | `users.list` | ok | PASS | 16 |
| users.list null-opt:status | `users.list` | ok | PASS | 22 |
| users.list num:limit | `users.list` | ok | PASS | 16 |
| users.list null-opt:limit | `users.list` | ok | PASS | 21 |
| users.list analyst deny | `users.list` | deny | PASS | 3 |

### X.users.get

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.get ∅args→err | `users.get` | tool_err | PASS | 2 |
| users.get null:id | `users.get` | tool_err | PASS | 2 |
| users.get unknown arg | `users.get` | tool_err | PASS | 3 |
| users.get badtype:id | `users.get` | fail | PASS | 3 |
| users.get path:id:slash | `users.get` | tool_err | PASS | 2 |
| users.get path:id:dotdot | `users.get` | tool_err | PASS | 2 |
| users.get path:id:empty | `users.get` | tool_err | PASS | 2 |
| users.get path:id:array | `users.get` | tool_err | PASS | 3 |
| users.get ghost:id | `users.get` | fail | PASS | 17 |
| users.get analyst deny | `users.get` | deny | PASS | 4 |

### X.users.create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.create ∅args→err | `users.create` | tool_err | PASS | 2 |
| users.create null:login | `users.create` | tool_err | PASS | 2 |
| users.create null:display_name | `users.create` | tool_err | PASS | 2 |
| users.create unknown arg | `users.create` | tool_err | PASS | 2 |
| users.create badtype:login | `users.create` | fail | PASS | 13 |
| users.create badtype:display_name | `users.create` | fail | PASS | 13 |
| users.create emptystr:login | `users.create` | fail | PASS | 13 |
| users.create emptystr:display_name | `users.create` | fail | PASS | 13 |
| users.create analyst deny | `users.create` | deny | PASS | 3 |

### X.users.update

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.update ∅args→err | `users.update` | tool_err | PASS | 3 |
| users.update null:id | `users.update` | tool_err | PASS | 2 |
| users.update unknown arg | `users.update` | tool_err | PASS | 2 |
| users.update badtype:id | `users.update` | fail | PASS | 3 |
| users.update path:id:slash | `users.update` | tool_err | PASS | 3 |
| users.update path:id:dotdot | `users.update` | tool_err | PASS | 2 |
| users.update path:id:empty | `users.update` | tool_err | PASS | 3 |
| users.update path:id:array | `users.update` | tool_err | PASS | 3 |
| users.update ghost:id | `users.update` | fail | PASS | 17 |
| users.update analyst deny | `users.update` | deny | PASS | 4 |

### X.users.delete

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.delete ∅args→err | `users.delete` | tool_err | PASS | 3 |
| users.delete null:id | `users.delete` | tool_err | PASS | 3 |
| users.delete unknown arg | `users.delete` | tool_err | PASS | 3 |
| users.delete badtype:id | `users.delete` | fail | PASS | 4 |
| users.delete path:id:slash | `users.delete` | tool_err | PASS | 3 |
| users.delete path:id:dotdot | `users.delete` | tool_err | PASS | 3 |
| users.delete path:id:empty | `users.delete` | tool_err | PASS | 3 |
| users.delete path:id:array | `users.delete` | tool_err | PASS | 4 |
| users.delete ghost:id | `users.delete` | fail | PASS | 19 |
| users.delete analyst deny | `users.delete` | deny | PASS | 5 |

### X.users.activity

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.activity ∅args→err | `users.activity` | tool_err | PASS | 3 |
| users.activity null:id | `users.activity` | tool_err | PASS | 4 |
| users.activity unknown arg | `users.activity` | tool_err | PASS | 4 |
| users.activity badtype:id | `users.activity` | fail | PASS | 4 |
| users.activity path:id:slash | `users.activity` | tool_err | PASS | 4 |
| users.activity path:id:dotdot | `users.activity` | tool_err | PASS | 5 |
| users.activity path:id:empty | `users.activity` | tool_err | PASS | 4 |
| users.activity path:id:array | `users.activity` | tool_err | PASS | 4 |
| users.activity ghost:id | `users.activity` | fail | PASS | 26 |
| users.activity analyst deny | `users.activity` | deny | PASS | 5 |

### X.users.tokens

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.tokens ∅args→err | `users.tokens` | tool_err | PASS | 7 |
| users.tokens null:id | `users.tokens` | tool_err | PASS | 5 |
| users.tokens unknown arg | `users.tokens` | tool_err | PASS | 4 |
| users.tokens badtype:id | `users.tokens` | fail | PASS | 4 |
| users.tokens path:id:slash | `users.tokens` | tool_err | PASS | 4 |
| users.tokens path:id:dotdot | `users.tokens` | tool_err | PASS | 5 |
| users.tokens path:id:empty | `users.tokens` | tool_err | PASS | 4 |
| users.tokens path:id:array | `users.tokens` | tool_err | PASS | 5 |
| users.tokens ghost:id | `users.tokens` | fail | PASS | 24 |
| users.tokens analyst deny | `users.tokens` | deny | PASS | 5 |

### X.users.token_create

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.token_create ∅args→err | `users.token_create` | tool_err | PASS | 4 |
| users.token_create null:id | `users.token_create` | tool_err | PASS | 4 |
| users.token_create null:name | `users.token_create` | tool_err | PASS | 5 |
| users.token_create unknown arg | `users.token_create` | tool_err | PASS | 5 |
| users.token_create badtype:id | `users.token_create` | fail | PASS | 5 |
| users.token_create badtype:name | `users.token_create` | fail | PASS | 20 |
| users.token_create emptystr:name | `users.token_create` | fail | PASS | 22 |
| users.token_create path:id:slash | `users.token_create` | tool_err | PASS | 6 |
| users.token_create path:id:dotdot | `users.token_create` | tool_err | PASS | 15 |
| users.token_create path:id:empty | `users.token_create` | tool_err | PASS | 6 |
| users.token_create path:id:array | `users.token_create` | tool_err | PASS | 5 |
| users.token_create ghost:id | `users.token_create` | fail | PASS | 26 |
| users.token_create analyst deny | `users.token_create` | deny | PASS | 5 |

### X.users.token_revoke

| Кейс | Tool | Ожидание | Итог | мс |
|---|---|---|---|---|
| users.token_revoke ∅args→err | `users.token_revoke` | tool_err | PASS | 4 |
| users.token_revoke null:id | `users.token_revoke` | tool_err | PASS | 3 |
| users.token_revoke null:token_id | `users.token_revoke` | tool_err | PASS | 6 |
| users.token_revoke unknown arg | `users.token_revoke` | tool_err | PASS | 5 |
| users.token_revoke badtype:id | `users.token_revoke` | fail | PASS | 4 |
| users.token_revoke badtype:token_id | `users.token_revoke` | fail | PASS | 4 |
| users.token_revoke path:id:slash | `users.token_revoke` | tool_err | PASS | 5 |
| users.token_revoke path:id:dotdot | `users.token_revoke` | tool_err | PASS | 5 |
| users.token_revoke path:id:empty | `users.token_revoke` | tool_err | PASS | 4 |
| users.token_revoke path:id:array | `users.token_revoke` | tool_err | PASS | 4 |
| users.token_revoke ghost:id | `users.token_revoke` | fail | PASS | 16 |
| users.token_revoke path:token_id:slash | `users.token_revoke` | tool_err | PASS | 3 |
| users.token_revoke path:token_id:dotdot | `users.token_revoke` | tool_err | PASS | 3 |
| users.token_revoke path:token_id:empty | `users.token_revoke` | tool_err | PASS | 2 |
| users.token_revoke path:token_id:array | `users.token_revoke` | tool_err | PASS | 2 |
| users.token_revoke ghost:token_id | `users.token_revoke` | fail | PASS | 13 |
| users.token_revoke analyst deny | `users.token_revoke` | deny | PASS | 4 |
