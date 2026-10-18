# EngineR MCP — каталог тест-кейсов

Автогенерация из `build_cases()` в `mcp_test_runner.py` (`python docs/testing/gen_case_catalog.py`).
Всего кейсов: **1241** в 115 областях. Все выполняются через MCP endpoint `POST /mcp` (JSON-RPC `tools/call`, `tools/list`, `initialize`).

## Способы выполнения

```bash
# Весь каталог:
python docs/testing/mcp_test_runner.py

# Одна область (префикс кода области):
python docs/testing/mcp_test_runner.py --area J.requests

# Другой стенд / учётная запись:
python docs/testing/mcp_test_runner.py \
    --mcp http://host:8020 --api http://host:8010 \
    --login admin --password enginer-demo

# Прямой JSON-RPC вызов:
curl -X POST http://localhost:8020/mcp \
  -H "Authorization: Bearer egr_<token>" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call",
       "params":{"name":"entities.list","arguments":{"limit":5}}}'
```

Раннер перед прогоном сам заводит две сервисные учётки (`qa-mcp-admin`: admins+all_data, `qa-mcp-analyst`: analysts+all_data), выпускает каждой bearer-токен и гоняет все кейсы через `tools/call`. Столбец «Токен» — какой принципал выполняет вызов.

## Семантика ожиданий (expect)

| expect | Смысл |
|---|---|
| `ok` | вызов успешен; `match` — подстрока, обязательная в ответе |
| `tool_err` | MCP-валидация отклонила вызов до обращения к API (isError) |
| `fail` | API вернул бизнес-ошибку 4xx |
| `deny` | нет capability у принципала (-32602 / not granted) |
| `http401` | транспорт отклонил bearer-токен |

`fn:` — аргументы вычисляются из env (id, созданные ранними кейсами); `save:` — ответ сохраняется в env для цепочек.

## A.entities.list/search

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| A1 list all | `entities.list` | ok | admin |
| A2 type filter person | `entities.list` | ok | admin |
| A3 free-text search | `entities.list` | ok | admin |
| A4 search no hit | `entities.list` | ok | admin |
| A5 limit=1 pagination | `entities.list` | ok | admin |
| A6 cursor page 2 | `entities.list` | ok | admin |
| A7 limit wrong type | `entities.list` | ok | admin |
| A8 unknown arg rejected | `entities.list` | tool_err | admin |
| A9 type nonexistent alias | `entities.list` | ok | admin |
| A10 analyst read allowed | `entities.list` | ok | analyst |
| A11 filter legal_entity | `entities.list` | ok | admin |

## B.entities.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| B1 get seeded person | `entities.get` | ok | admin |
| B2 get legal_entity | `entities.get` | ok | admin |
| B3 nonexistent uuid → 404 | `entities.get` | fail | admin |
| B4 malformed id | `entities.get` | fail | admin |
| B5 path traversal rejected | `entities.get` | tool_err | admin |
| B6 numeric id accepted→404 | `entities.get` | fail | admin |
| B7 structured id rejected | `entities.get` | tool_err | admin |
| B8 retired entity readable, status=retired | `entities.get` | ok | admin |
| B9 empty id rejected | `entities.get` | tool_err | admin |
| B10 analyst get | `entities.get` | ok | analyst |

## C.entities.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| C1 create person | `entities.create` | ok | admin |
| C2 create legal_entity | `entities.create` | ok | admin |
| C3 missing type_id | `entities.create` | tool_err | admin |
| C4 unknown type alias | `entities.create` | fail | admin |
| C5 duplicate INN → return_existing | `entities.create` | ok | admin |
| C6 identity_mode=reject on dup | `entities.create` | fail | admin |
| C7 create document | `entities.create` | ok | admin |
| C8 create check entity | `entities.create` | ok | admin |
| C9 analyst denied | `entities.create` | deny | analyst |
| C10 unknown arg | `entities.create` | tool_err | admin |
| C11 create request entity | `entities.create` | ok | admin |
| C12 create_candidate creates new | `entities.create` | ok | admin |

## D.entities.update

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| D1 update field | `entities.update` | ok | admin |
| D2 stale revision | `entities.update` | fail | admin |
| D3 nonexistent id | `entities.update` | fail | admin |
| D4 update retired | `entities.update` | fail | admin |
| D5 single predicate form | `entities.update` | ok | admin |
| D6 unknown arg | `entities.update` | tool_err | admin |
| D7 analyst denied | `entities.update` | deny | analyst |
| D8 verify persisted | `entities.get` | ok | admin |
| D9 update le field | `entities.update` | ok | admin |
| D10 no fields no-op | `entities.update` | ok | admin |

## E.entities.retire/merge

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| E1 create loser | `entities.create` | ok | admin |
| E2 merge into canonical | `entities.merge` | ok | admin |
| E3 merge idempotent repeat | `entities.merge` | ok | admin |
| E4 loser marked (not 404) | `entities.get` | ok | admin |
| E5 merge nonexistent loser | `entities.merge` | fail | admin |
| E6 self merge | `entities.merge` | fail | admin |
| E7 retire with reason | `entities.retire` | ok | admin |
| E8 retire again → 404 | `entities.retire` | fail | admin |
| E9 analyst retire denied | `entities.retire` | deny | analyst |
| E10 merge analyst denied | `entities.merge` | deny | analyst |
| E11 retire nonexistent | `entities.retire` | fail | admin |

## F.relations

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| F1 works_for | `relations.create` | ok | admin |
| F2 owns | `relations.create` | ok | admin |
| F3 for_entity request→le | `relations.create` | ok | admin |
| F4 related_to wildcard | `relations.create` | ok | admin |
| F5 missing predicate | `relations.create` | tool_err | admin |
| F6 unknown predicate | `relations.create` | fail | admin |
| F7 missing target | `relations.create` | fail | admin |
| F8 list outgoing | `relations.list` | ok | admin |
| F9 list incoming on le | `relations.list` | ok | admin |
| F10 delete relation | `relations.delete` | ok | admin |
| F11 delete nonexistent | `relations.delete` | fail | admin |
| F12 analyst create denied | `relations.create` | deny | analyst |

## G.documents/files

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| G1 documents.upload | `documents.upload` | ok | admin |
| G2 documents.list | `documents.list` | ok | admin |
| G3 documents.download | `documents.download` | ok | admin |
| G4 upload_init | `files.upload_init` | ok | admin |
| G5 upload_chunk base64 | `files.upload_chunk` | ok | admin |
| G6 upload_status | `files.upload_status` | ok | admin |
| G7 upload_complete | `files.upload_complete` | ok | admin |
| G8 files.get | `files.get` | ok | admin |
| G9 files.download | `files.download` | ok | admin |
| G10 upload_init bad hash len | `files.upload_init` | fail | admin |
| G11 upload_chunk bad base64 | `files.upload_chunk` | tool_err | admin |
| G12 upload_abort | `files.upload_abort` | fail | admin |
| G13 files.delete | `files.delete` | ok | admin |
| G14 documents.delete | `documents.delete` | ok | admin |
| G15 analyst upload denied | `documents.upload` | deny | analyst |

## H.tasks

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| H1 tasks.create | `tasks.create` | ok | admin |
| H2 tasks.list | `tasks.list` | ok | admin |
| H3 tasks.start | `tasks.start` | ok | admin |
| H4 tasks.complete | `tasks.complete` | ok | admin |
| H5 create+cancel | `tasks.create` | ok | admin |
| H6 tasks.cancel | `tasks.cancel` | ok | admin |
| H7 missing kind → 422 | `tasks.create` | fail | admin |
| H8 complete completed → error | `tasks.complete` | fail | admin |
| H9 task on ghost entity → 404 | `tasks.create` | fail | admin |
| H10 analyst create denied | `tasks.create` | deny | analyst |
| H11 tasks.list analyst | `tasks.list` | ok | analyst |

## I.checks

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| I1 checks.create kyc | `checks.create` | ok | admin |
| I2 checks.create sanctions | `checks.create` | ok | admin |
| I3 checks.list | `checks.list` | ok | admin |
| I4 checks.list kind filter | `checks.list` | ok | admin |
| I5 checks.list status filter | `checks.list` | ok | admin |
| I6 checks.get | `checks.get` | ok | admin |
| I7 checks.evidence | `checks.evidence` | ok | admin |
| I8 entity_checks.list | `entity_checks.list` | ok | admin |
| I9 missing check_kind | `checks.create` | tool_err | admin |
| I10 ghost target → 404 | `checks.create` | fail | admin |
| I11 analyst denied | `checks.create` | deny | analyst |

## J.requests

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| J1 requests.create | `requests.create` | ok | admin |
| J2 requests.list | `requests.list` | ok | admin |
| J3 status filter | `requests.list` | ok | admin |
| J4 missing target | `requests.create` | tool_err | admin |
| J5 ghost target → 404 | `requests.create` | fail | admin |
| J6 limit param | `requests.list` | ok | admin |
| J7 unknown arg | `requests.create` | tool_err | admin |
| J8 create minimal | `requests.create` | ok | admin |
| J9 analyst list | `requests.list` | ok | analyst |
| J10 analyst create denied | `requests.create` | deny | analyst |

## K.workflows

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| K1 workflows.create | `workflows.create` | ok | admin |
| K2 workflows.get | `workflows.get` | ok | admin |
| K3 workflows.tasks | `workflows.tasks` | ok | admin |
| K4 transition valid | `workflows.transition` | ok | admin |
| K5 transition invalid state | `workflows.transition` | fail | admin |
| K6 missing definition_id | `workflows.create` | tool_err | admin |
| K7 unknown definition | `workflows.create` | fail | admin |
| K8 get ghost → 404 | `workflows.get` | fail | admin |
| K9 analyst create denied | `workflows.create` | deny | analyst |
| K10 create without target → 404 | `workflows.create` | fail | admin |
| K11 transition ghost | `workflows.transition` | fail | admin |

## L.reports

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| L1 generate json | `reports.generate` | ok | admin |
| L2 reports.list | `reports.list` | ok | admin |
| L3 generate csv | `reports.generate` | ok | admin |
| L4 missing entity_id | `reports.generate` | tool_err | admin |
| L5 ghost entity → 404 | `reports.generate` | fail | admin |
| L6 generate pdf | `reports.generate` | ok | admin |
| L7 generate no format | `reports.generate` | ok | admin |
| L8 second report new id | `reports.generate` | ok | admin |
| L9 analyst denied | `reports.generate` | deny | analyst |
| L10 reports.list analyst | `reports.list` | ok | analyst |

## M.connectors

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| M1 create | `connectors.create` | ok | admin |
| M2 list | `connectors.list` | ok | admin |
| M3 get | `connectors.get` | ok | admin |
| M4 update disable | `connectors.update` | ok | admin |
| M5 enable | `connectors.enable` | ok | admin |
| M6 disable | `connectors.disable` | ok | admin |
| M7 health | `connectors.health` | ok | admin |
| M8 missing type → tool_err | `connectors.create` | tool_err | admin |
| M9 get ghost → 404 | `connectors.get` | fail | admin |
| M10 update ghost → 404 | `connectors.update` | fail | admin |
| M11 analyst denied | `connectors.create` | deny | analyst |
| M12 delete | `connectors.delete` | ok | admin |

## N.schema

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| N1 schema.types | `schema.types` | ok | admin |
| N2 type_get alias | `schema.type_get` | ok | admin |
| N3 type_view form | `schema.type_view` | ok | admin |
| N4 schema.predicates | `schema.predicates` | ok | admin |
| N5 schema.published | `schema.published` | ok | admin |
| N6 draft_create | `schema.draft_create` | ok | admin |
| N7 draft_get | `schema.draft_get` | ok | admin |
| N8 draft_update no etag → 412 | `schema.draft_update` | fail | admin |
| N9 draft_update +etag | `schema.draft_update` | ok | admin |
| N10 draft_validate | `schema.draft_validate` | ok | admin |
| N11 draft_impact | `schema.draft_impact` | ok | admin |
| N11b refresh etag | `schema.draft_get` | ok | admin |
| N12 draft_publish +etag | `schema.draft_publish` | ok | admin |
| N13 revision_get | `schema.revision_get` | ok | admin |
| N14 type_get ghost → 404 | `schema.type_get` | fail | admin |
| N15 analyst schema.types | `schema.types` | ok | analyst |
| N16 analyst draft denied | `schema.draft_create` | deny | analyst |

## O.users/tokens

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| O1 users.list | `users.list` | ok | admin |
| O2 q filter | `users.list` | ok | admin |
| O3 status filter | `users.list` | ok | admin |
| O4 users.get | `users.get` | ok | admin |
| O5 create user | `users.create` | ok | admin |
| O6 create service acct | `users.create` | ok | admin |
| O7 dup login rejected | `users.create` | fail | admin |
| O8 token_create | `users.token_create` | ok | admin |
| O9 tokens list no secret | `users.tokens` | ok | admin |
| O10 token_revoke | `users.token_revoke` | ok | admin |
| O11 users.update groups | `users.update` | ok | admin |
| O12 users.get roles | `users.get` | ok | admin |
| O13 users.activity | `users.activity` | ok | admin |
| O14 users.delete | `users.delete` | ok | admin |
| O15 analyst denied users.list | `users.list` | deny | analyst |
| O15b role catalog | `users.role_groups` | ok | admin |
| O16 short password rejected | `users.create` | fail | admin |

## P.permission groups

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| P1 groups list | `permissions.groups` | ok | admin |
| P2 group_create capability | `permissions.group_create` | ok | admin |
| P3 group_create data | `permissions.group_create` | ok | admin |
| P4 invalid class → 422 | `permissions.group_create` | fail | admin |
| P5 missing name → tool_err | `permissions.group_create` | tool_err | admin |
| P6 group_update principals | `permissions.group_update` | ok | admin |
| P7 group_update parent | `permissions.group_update` | ok | admin |
| P8 delete special role | `permissions.group_delete` | fail | admin |
| P9 delete qa role | `permissions.group_delete` | ok | admin |
| P10 analyst denied | `permissions.groups` | deny | analyst |
| P11 delete ghost → 404 | `permissions.group_delete` | fail | admin |
| P12 delete data role | `permissions.group_delete` | ok | admin |

## Q.audit

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| Q1 audit.query | `audit.query` | ok | admin |
| Q2 filter action | `audit.query` | ok | admin |
| Q3 filter effect | `audit.query` | ok | admin |
| Q4 checkpoint_get | `audit.checkpoint_get` | ok | admin |
| Q5 checkpoint_create | `audit.checkpoint_create` | ok | admin |
| Q6 audit.verify | `audit.verify` | ok | admin |
| Q7 retention_get | `audit.retention_get` | ok | admin |
| Q8 retention_set | `audit.retention_set` | ok | admin |
| Q9 retention bad days → 422 | `audit.retention_set` | fail | admin |
| Q10 analyst denied | `audit.query` | deny | analyst |
| Q11 restore retention | `audit.retention_set` | ok | admin |

## R.monitoring/misc

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| R1 context.get | `context.get` | ok | admin |
| R2 dashboard.stats | `dashboard.stats` | ok | admin |
| R3 monitors.list | `monitors.list` | ok | admin |
| R4 monitors.run | `monitors.run` | ok | admin |
| R5 factors.list | `factors.list` | ok | admin |
| R6 assessments.list | `assessments.list` | ok | admin |
| R7 assessment.create | `assessment.create` | ok | admin |
| R7b invalid level → 422 | `assessment.create` | fail | admin |
| R8 entities.history | `entities.history` | ok | admin |
| R9 entities.query AST | `entities.query` | ok | admin |
| R10 analyst context | `context.get` | ok | analyst |
| R11 analyst dashboard | `dashboard.stats` | ok | analyst |

## S.protocol/security

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| S1 no token → 401 | `__raw__` | http401 | admin |
| S2 garbage token → 401 | `__raw__` | http401 | admin |
| S3 unknown method → -32601 | `__rpc__` | fail | admin |
| S4 unknown tool → -32602 | `no.such.tool` | fail | admin |
| S5 analyst tools/list smaller | `__tools_list__` | ok | analyst |
| S6 analyst admin tool denied | `audit.retention_set` | deny | analyst |
| S7 path arg '?x' rejected | `entities.get` | tool_err | admin |
| S8 path arg 'a/b' rejected | `entities.get` | tool_err | admin |
| S9 initialize | `__rpc__` | ok | admin |
| S10 missing required arg | `entities.get` | tool_err | admin |
| S11 tools/list admin 90+ | `__tools_list__` | ok | admin |
| S12 token revoked mid-flight → 401 | `__raw__` | http401 | admin |

## T.targeted

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| reports.list has data | `reports.list` | ok | admin |
| reports.list has ids | `reports.list` | ok | admin |
| reports.list status field | `reports.list` | ok | admin |
| reports.list repeat | `reports.list` | ok | admin |
| reports.list limit probe | `reports.list` | tool_err | admin |
| connectors.create trust>100 | `connectors.create` | fail | admin |
| connectors.create negative rps | `connectors.create` | fail | admin |
| connectors.list has data | `connectors.list` | ok | admin |
| connectors.list has conn | `connectors.list` | ok | admin |
| connectors.list repeat | `connectors.list` | ok | admin |
| connectors.list analyst deny2 | `connectors.list` | deny | analyst |
| connectors.create t-conn | `connectors.create` | ok | admin |
| connectors.list sees t-conn | `connectors.list` | ok | admin |
| connectors.list bogus | `connectors.list` | tool_err | admin |
| schema.types person | `schema.types` | ok | admin |
| schema.types legal_entity | `schema.types` | ok | admin |
| schema.types request | `schema.types` | ok | admin |
| schema.types document | `schema.types` | ok | admin |
| schema.types contract | `schema.types` | ok | admin |
| schema.predicates works_for | `schema.predicates` | ok | admin |
| schema.predicates owns | `schema.predicates` | ok | admin |
| schema.predicates inn | `schema.predicates` | ok | admin |
| schema.predicates full_name | `schema.predicates` | ok | admin |
| schema.predicates has_contract | `schema.predicates` | ok | admin |
| schema.predicates related_to | `schema.predicates` | ok | admin |
| schema.published revision_id | `schema.published` | ok | admin |
| schema.published semver | `schema.published` | ok | admin |
| schema.published status | `schema.published` | ok | admin |
| schema.published types | `schema.published` | ok | admin |
| schema.published person | `schema.published` | ok | admin |
| schema.published etag-ish | `schema.published` | ok | admin |
| schema.drafts has data | `schema.drafts` | ok | admin |
| schema.drafts revision id | `schema.drafts` | ok | admin |
| schema.drafts shows qa | `schema.drafts` | ok | admin |
| schema.drafts repeat | `schema.drafts` | ok | admin |
| schema.drafts analyst deny2 | `schema.drafts` | deny | analyst |
| schema.drafts bogus arg | `schema.drafts` | tool_err | admin |
| schema.drafts has status | `schema.drafts` | ok | admin |
| draft_create ghost base | `schema.draft_create` | fail | admin |
| draft_create empty name | `schema.draft_create` | fail | admin |
| role_groups admins | `users.role_groups` | ok | admin |
| role_groups analysts | `users.role_groups` | ok | analyst |
| role_groups all_data | `users.role_groups` | ok | admin |
| role_groups class field | `users.role_groups` | ok | admin |
| role_groups capability | `users.role_groups` | ok | admin |
| role_groups parent | `users.role_groups` | ok | admin |
| perm.groups admins | `permissions.groups` | ok | admin |
| perm.groups rules | `permissions.groups` | ok | admin |
| perm.groups class | `permissions.groups` | ok | admin |
| perm.groups all_data | `permissions.groups` | ok | admin |
| perm.groups repeat | `permissions.groups` | ok | admin |
| checkpoint_get data | `audit.checkpoint_get` | ok | admin |
| checkpoint_get again | `audit.checkpoint_get` | ok | admin |
| checkpoint_get id field | `audit.checkpoint_get` | ok | admin |
| checkpoint_get 4th | `audit.checkpoint_get` | ok | admin |
| checkpoint_get 5th | `audit.checkpoint_get` | ok | admin |
| checkpoint_get bogus | `audit.checkpoint_get` | tool_err | admin |
| checkpoint_create #2 | `audit.checkpoint_create` | ok | admin |
| checkpoint_create #3 | `audit.checkpoint_create` | ok | admin |
| checkpoint_create #4 | `audit.checkpoint_create` | ok | admin |
| checkpoint_create idempotent-shape | `audit.checkpoint_create` | ok | admin |
| checkpoint visible in get | `audit.checkpoint_get` | ok | admin |
| checkpoint_create bogus | `audit.checkpoint_create` | tool_err | admin |
| checkpoint_create #5 | `audit.checkpoint_create` | ok | admin |
| verify after checkpoints | `audit.verify` | ok | admin |
| verify range args | `audit.verify` | ok | admin |
| verify true | `audit.verify` | ok | admin |
| verify analyst deny2 | `audit.verify` | deny | analyst |
| verify repeat | `audit.verify` | ok | admin |
| verify bad range | `audit.verify` | ok | admin |
| retention_get days | `audit.retention_get` | ok | admin |
| retention_get again | `audit.retention_get` | ok | admin |
| retention_get fields | `audit.retention_get` | ok | admin |
| retention_get 4th | `audit.retention_get` | ok | admin |
| retention_get bogus | `audit.retention_get` | tool_err | admin |
| retention_get analyst deny | `audit.retention_get` | deny | analyst |
| retention_set negative | `audit.retention_set` | fail | admin |
| retention_set huge | `audit.retention_set` | fail | admin |
| retention_set badtype | `audit.retention_set` | fail | admin |
| context caps | `context.get` | ok | admin |
| context principal | `context.get` | ok | admin |
| context tenant | `context.get` | ok | admin |
| context entity.read | `context.get` | ok | admin |
| context admin cap | `context.get` | ok | admin |
| dashboard my_tasks | `dashboard.stats` | ok | admin |
| dashboard shape | `dashboard.stats` | ok | admin |
| dashboard repeat | `dashboard.stats` | ok | admin |
| dashboard analyst | `dashboard.stats` | ok | analyst |
| dashboard bogus arg | `dashboard.stats` | tool_err | admin |
| monitors has data | `monitors.list` | ok | admin |
| monitors seeded | `monitors.list` | ok | admin |
| monitors frequency | `monitors.list` | ok | admin |
| monitors repeat | `monitors.list` | ok | admin |
| monitors analyst deny2 | `monitors.list` | deny | analyst |
| monitors bogus arg | `monitors.list` | tool_err | admin |
| query typed person | `entities.query` | ok | admin |
| query search | `entities.query` | ok | admin |
| query page size 1 | `entities.query` | ok | admin |
| query empty obj | `entities.query` | ok | admin |
| query null→err | `entities.query` | tool_err | admin |
| query non-object→422 | `entities.query` | fail | admin |

## X.context.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| context.get ∅args | `context.get` | ok | admin |
| context.get unknown arg | `context.get` | tool_err | admin |
| context.get analyst ok | `context.get` | ok | analyst |

## X.dashboard.stats

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| dashboard.stats ∅args | `dashboard.stats` | ok | admin |
| dashboard.stats unknown arg | `dashboard.stats` | tool_err | admin |
| dashboard.stats analyst ok | `dashboard.stats` | ok | analyst |

## X.entities.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.list ∅args | `entities.list` | ok | admin |
| entities.list unknown arg | `entities.list` | tool_err | admin |
| entities.list filter:type | `entities.list` | ok | admin |
| entities.list null-opt:type | `entities.list` | ok | admin |
| entities.list filter:search | `entities.list` | ok | admin |
| entities.list null-opt:search | `entities.list` | ok | admin |
| entities.list num:limit | `entities.list` | ok | admin |
| entities.list null-opt:limit | `entities.list` | ok | admin |
| entities.list filter:cursor | `entities.list` | ok | admin |
| entities.list null-opt:cursor | `entities.list` | ok | admin |
| entities.list analyst ok | `entities.list` | ok | analyst |

## X.entities.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.get ∅args→err | `entities.get` | tool_err | admin |
| entities.get null:id | `entities.get` | tool_err | admin |
| entities.get unknown arg | `entities.get` | tool_err | admin |
| entities.get badtype:id | `entities.get` | fail | admin |
| entities.get path:id:slash | `entities.get` | tool_err | admin |
| entities.get path:id:dotdot | `entities.get` | tool_err | admin |
| entities.get path:id:empty | `entities.get` | tool_err | admin |
| entities.get path:id:array | `entities.get` | tool_err | admin |
| entities.get ghost:id | `entities.get` | fail | admin |
| entities.get analyst reach | `entities.get` | reach | analyst |

## X.entities.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.create ∅args→err | `entities.create` | tool_err | admin |
| entities.create null:type_id | `entities.create` | tool_err | admin |
| entities.create unknown arg | `entities.create` | tool_err | admin |
| entities.create badtype:type_id | `entities.create` | fail | admin |
| entities.create emptystr:type_id | `entities.create` | fail | admin |
| entities.create analyst deny | `entities.create` | deny | analyst |

## X.entities.update

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.update ∅args→err | `entities.update` | tool_err | admin |
| entities.update null:id | `entities.update` | tool_err | admin |
| entities.update unknown arg | `entities.update` | tool_err | admin |
| entities.update badtype:id | `entities.update` | fail | admin |
| entities.update path:id:slash | `entities.update` | tool_err | admin |
| entities.update path:id:dotdot | `entities.update` | tool_err | admin |
| entities.update path:id:empty | `entities.update` | tool_err | admin |
| entities.update path:id:array | `entities.update` | tool_err | admin |
| entities.update ghost:id | `entities.update` | fail | admin |
| entities.update analyst deny | `entities.update` | deny | analyst |

## X.entities.retire

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.retire ∅args→err | `entities.retire` | tool_err | admin |
| entities.retire null:id | `entities.retire` | tool_err | admin |
| entities.retire unknown arg | `entities.retire` | tool_err | admin |
| entities.retire badtype:id | `entities.retire` | fail | admin |
| entities.retire path:id:slash | `entities.retire` | tool_err | admin |
| entities.retire path:id:dotdot | `entities.retire` | tool_err | admin |
| entities.retire path:id:empty | `entities.retire` | tool_err | admin |
| entities.retire path:id:array | `entities.retire` | tool_err | admin |
| entities.retire ghost:id | `entities.retire` | fail | admin |
| entities.retire analyst deny | `entities.retire` | deny | analyst |

## X.entities.merge

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.merge ∅args→err | `entities.merge` | tool_err | admin |
| entities.merge null:id | `entities.merge` | tool_err | admin |
| entities.merge null:loser_public_id | `entities.merge` | tool_err | admin |
| entities.merge unknown arg | `entities.merge` | tool_err | admin |
| entities.merge badtype:id | `entities.merge` | fail | admin |
| entities.merge badtype:loser_public_id | `entities.merge` | fail | admin |
| entities.merge emptystr:loser_public_id | `entities.merge` | fail | admin |
| entities.merge path:id:slash | `entities.merge` | tool_err | admin |
| entities.merge path:id:dotdot | `entities.merge` | tool_err | admin |
| entities.merge path:id:empty | `entities.merge` | tool_err | admin |
| entities.merge path:id:array | `entities.merge` | tool_err | admin |
| entities.merge ghost:id | `entities.merge` | fail | admin |
| entities.merge analyst deny | `entities.merge` | deny | analyst |

## X.entities.query

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.query ∅args→err | `entities.query` | tool_err | admin |
| entities.query null:query | `entities.query` | tool_err | admin |
| entities.query unknown arg | `entities.query` | tool_err | admin |
| entities.query badtype:query | `entities.query` | fail | admin |
| entities.query analyst reach | `entities.query` | reach | analyst |

## X.entities.graph

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.graph ∅args→err | `entities.graph` | tool_err | admin |
| entities.graph null:id | `entities.graph` | tool_err | admin |
| entities.graph unknown arg | `entities.graph` | tool_err | admin |
| entities.graph badtype:id | `entities.graph` | fail | admin |
| entities.graph path:id:slash | `entities.graph` | tool_err | admin |
| entities.graph path:id:dotdot | `entities.graph` | tool_err | admin |
| entities.graph path:id:empty | `entities.graph` | tool_err | admin |
| entities.graph path:id:array | `entities.graph` | tool_err | admin |
| entities.graph ghost:id | `entities.graph` | fail | admin |
| entities.graph analyst reach | `entities.graph` | reach | analyst |

## X.entities.history

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entities.history ∅args→err | `entities.history` | tool_err | admin |
| entities.history null:id | `entities.history` | tool_err | admin |
| entities.history unknown arg | `entities.history` | tool_err | admin |
| entities.history badtype:id | `entities.history` | fail | admin |
| entities.history path:id:slash | `entities.history` | tool_err | admin |
| entities.history path:id:dotdot | `entities.history` | tool_err | admin |
| entities.history path:id:empty | `entities.history` | tool_err | admin |
| entities.history path:id:array | `entities.history` | tool_err | admin |
| entities.history ghost:id | `entities.history` | fail | admin |
| entities.history analyst reach | `entities.history` | reach | analyst |

## X.relations.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| relations.list ∅args→err | `relations.list` | tool_err | admin |
| relations.list null:id | `relations.list` | tool_err | admin |
| relations.list unknown arg | `relations.list` | tool_err | admin |
| relations.list badtype:id | `relations.list` | fail | admin |
| relations.list path:id:slash | `relations.list` | tool_err | admin |
| relations.list path:id:dotdot | `relations.list` | tool_err | admin |
| relations.list path:id:empty | `relations.list` | tool_err | admin |
| relations.list path:id:array | `relations.list` | tool_err | admin |
| relations.list ghost:id | `relations.list` | fail | admin |
| relations.list analyst reach | `relations.list` | reach | analyst |

## X.relations.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| relations.create ∅args→err | `relations.create` | tool_err | admin |
| relations.create null:id | `relations.create` | tool_err | admin |
| relations.create null:predicate | `relations.create` | tool_err | admin |
| relations.create unknown arg | `relations.create` | tool_err | admin |
| relations.create badtype:id | `relations.create` | fail | admin |
| relations.create badtype:predicate | `relations.create` | fail | admin |
| relations.create emptystr:predicate | `relations.create` | fail | admin |
| relations.create path:id:slash | `relations.create` | tool_err | admin |
| relations.create path:id:dotdot | `relations.create` | tool_err | admin |
| relations.create path:id:empty | `relations.create` | tool_err | admin |
| relations.create path:id:array | `relations.create` | tool_err | admin |
| relations.create ghost:id | `relations.create` | fail | admin |
| relations.create analyst deny | `relations.create` | deny | analyst |

## X.relations.delete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| relations.delete ∅args→err | `relations.delete` | tool_err | admin |
| relations.delete null:id | `relations.delete` | tool_err | admin |
| relations.delete null:relation_id | `relations.delete` | tool_err | admin |
| relations.delete unknown arg | `relations.delete` | tool_err | admin |
| relations.delete badtype:id | `relations.delete` | fail | admin |
| relations.delete badtype:relation_id | `relations.delete` | fail | admin |
| relations.delete path:id:slash | `relations.delete` | tool_err | admin |
| relations.delete path:id:dotdot | `relations.delete` | tool_err | admin |
| relations.delete path:id:empty | `relations.delete` | tool_err | admin |
| relations.delete path:id:array | `relations.delete` | tool_err | admin |
| relations.delete ghost:id | `relations.delete` | fail | admin |
| relations.delete path:relation_id:slash | `relations.delete` | tool_err | admin |
| relations.delete path:relation_id:dotdot | `relations.delete` | tool_err | admin |
| relations.delete path:relation_id:empty | `relations.delete` | tool_err | admin |
| relations.delete path:relation_id:array | `relations.delete` | tool_err | admin |
| relations.delete ghost:relation_id | `relations.delete` | fail | admin |
| relations.delete analyst deny | `relations.delete` | deny | analyst |

## X.documents.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| documents.list ∅args→err | `documents.list` | tool_err | admin |
| documents.list null:id | `documents.list` | tool_err | admin |
| documents.list unknown arg | `documents.list` | tool_err | admin |
| documents.list badtype:id | `documents.list` | fail | admin |
| documents.list path:id:slash | `documents.list` | tool_err | admin |
| documents.list path:id:dotdot | `documents.list` | tool_err | admin |
| documents.list path:id:empty | `documents.list` | tool_err | admin |
| documents.list path:id:array | `documents.list` | tool_err | admin |
| documents.list ghost:id | `documents.list` | fail | admin |
| documents.list analyst reach | `documents.list` | reach | analyst |

## X.documents.upload

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| documents.upload ∅args→err | `documents.upload` | tool_err | admin |
| documents.upload null:id | `documents.upload` | tool_err | admin |
| documents.upload null:kind | `documents.upload` | tool_err | admin |
| documents.upload null:file_name | `documents.upload` | tool_err | admin |
| documents.upload unknown arg | `documents.upload` | tool_err | admin |
| documents.upload badtype:id | `documents.upload` | fail | admin |
| documents.upload badtype:kind | `documents.upload` | fail | admin |
| documents.upload emptystr:kind | `documents.upload` | fail | admin |
| documents.upload path:id:slash | `documents.upload` | tool_err | admin |
| documents.upload path:id:dotdot | `documents.upload` | tool_err | admin |
| documents.upload path:id:empty | `documents.upload` | tool_err | admin |
| documents.upload path:id:array | `documents.upload` | tool_err | admin |
| documents.upload ghost:id | `documents.upload` | fail | admin |
| documents.upload analyst deny | `documents.upload` | deny | analyst |

## X.documents.delete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| documents.delete ∅args→err | `documents.delete` | tool_err | admin |
| documents.delete null:id | `documents.delete` | tool_err | admin |
| documents.delete null:doc_id | `documents.delete` | tool_err | admin |
| documents.delete unknown arg | `documents.delete` | tool_err | admin |
| documents.delete badtype:id | `documents.delete` | fail | admin |
| documents.delete badtype:doc_id | `documents.delete` | fail | admin |
| documents.delete path:id:slash | `documents.delete` | tool_err | admin |
| documents.delete path:id:dotdot | `documents.delete` | tool_err | admin |
| documents.delete path:id:empty | `documents.delete` | tool_err | admin |
| documents.delete path:id:array | `documents.delete` | tool_err | admin |
| documents.delete ghost:id | `documents.delete` | fail | admin |
| documents.delete path:doc_id:slash | `documents.delete` | tool_err | admin |
| documents.delete path:doc_id:dotdot | `documents.delete` | tool_err | admin |
| documents.delete path:doc_id:empty | `documents.delete` | tool_err | admin |
| documents.delete path:doc_id:array | `documents.delete` | tool_err | admin |
| documents.delete ghost:doc_id | `documents.delete` | fail | admin |
| documents.delete analyst deny | `documents.delete` | deny | analyst |

## X.files.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.get ∅args→err | `files.get` | tool_err | admin |
| files.get null:id | `files.get` | tool_err | admin |
| files.get unknown arg | `files.get` | tool_err | admin |
| files.get badtype:id | `files.get` | fail | admin |
| files.get path:id:slash | `files.get` | tool_err | admin |
| files.get path:id:dotdot | `files.get` | tool_err | admin |
| files.get path:id:empty | `files.get` | tool_err | admin |
| files.get path:id:array | `files.get` | tool_err | admin |
| files.get ghost:id | `files.get` | fail | admin |
| files.get analyst deny | `files.get` | deny | analyst |

## X.files.delete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.delete ∅args→err | `files.delete` | tool_err | admin |
| files.delete null:id | `files.delete` | tool_err | admin |
| files.delete unknown arg | `files.delete` | tool_err | admin |
| files.delete badtype:id | `files.delete` | fail | admin |
| files.delete path:id:slash | `files.delete` | tool_err | admin |
| files.delete path:id:dotdot | `files.delete` | tool_err | admin |
| files.delete path:id:empty | `files.delete` | tool_err | admin |
| files.delete path:id:array | `files.delete` | tool_err | admin |
| files.delete ghost:id | `files.delete` | fail | admin |
| files.delete analyst deny | `files.delete` | deny | analyst |

## X.files.upload_init

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.upload_init ∅args→err | `files.upload_init` | tool_err | admin |
| files.upload_init null:filename | `files.upload_init` | tool_err | admin |
| files.upload_init null:expected_size | `files.upload_init` | tool_err | admin |
| files.upload_init null:expected_hash | `files.upload_init` | tool_err | admin |
| files.upload_init unknown arg | `files.upload_init` | tool_err | admin |
| files.upload_init badtype:filename | `files.upload_init` | fail | admin |
| files.upload_init badtype:expected_size | `files.upload_init` | fail | admin |
| files.upload_init emptystr:filename | `files.upload_init` | fail | admin |
| files.upload_init analyst deny | `files.upload_init` | deny | analyst |

## X.files.upload_status

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.upload_status ∅args→err | `files.upload_status` | tool_err | admin |
| files.upload_status null:id | `files.upload_status` | tool_err | admin |
| files.upload_status unknown arg | `files.upload_status` | tool_err | admin |
| files.upload_status badtype:id | `files.upload_status` | fail | admin |
| files.upload_status path:id:slash | `files.upload_status` | tool_err | admin |
| files.upload_status path:id:dotdot | `files.upload_status` | tool_err | admin |
| files.upload_status path:id:empty | `files.upload_status` | tool_err | admin |
| files.upload_status path:id:array | `files.upload_status` | tool_err | admin |
| files.upload_status ghost:id | `files.upload_status` | fail | admin |
| files.upload_status analyst deny | `files.upload_status` | deny | analyst |

## X.files.upload_abort

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.upload_abort ∅args→err | `files.upload_abort` | tool_err | admin |
| files.upload_abort null:id | `files.upload_abort` | tool_err | admin |
| files.upload_abort unknown arg | `files.upload_abort` | tool_err | admin |
| files.upload_abort badtype:id | `files.upload_abort` | fail | admin |
| files.upload_abort path:id:slash | `files.upload_abort` | tool_err | admin |
| files.upload_abort path:id:dotdot | `files.upload_abort` | tool_err | admin |
| files.upload_abort path:id:empty | `files.upload_abort` | tool_err | admin |
| files.upload_abort path:id:array | `files.upload_abort` | tool_err | admin |
| files.upload_abort ghost:id | `files.upload_abort` | fail | admin |
| files.upload_abort analyst deny | `files.upload_abort` | deny | analyst |

## X.files.upload_chunk

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.upload_chunk ∅args→err | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk null:id | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk null:index | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk null:data_base64 | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk unknown arg | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk badtype:id | `files.upload_chunk` | fail | admin |
| files.upload_chunk badtype:index | `files.upload_chunk` | fail | admin |
| files.upload_chunk path:id:slash | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk path:id:dotdot | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk path:id:empty | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk path:id:array | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk ghost:id | `files.upload_chunk` | fail | admin |
| files.upload_chunk path:index:slash | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk path:index:dotdot | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk path:index:empty | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk path:index:array | `files.upload_chunk` | tool_err | admin |
| files.upload_chunk ghost:index | `files.upload_chunk` | fail | admin |
| files.upload_chunk analyst deny | `files.upload_chunk` | deny | analyst |

## X.files.upload_complete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.upload_complete ∅args→err | `files.upload_complete` | tool_err | admin |
| files.upload_complete null:id | `files.upload_complete` | tool_err | admin |
| files.upload_complete null:final_hash | `files.upload_complete` | tool_err | admin |
| files.upload_complete null:total_size | `files.upload_complete` | tool_err | admin |
| files.upload_complete unknown arg | `files.upload_complete` | tool_err | admin |
| files.upload_complete badtype:id | `files.upload_complete` | fail | admin |
| files.upload_complete badtype:final_hash | `files.upload_complete` | fail | admin |
| files.upload_complete emptystr:final_hash | `files.upload_complete` | fail | admin |
| files.upload_complete path:id:slash | `files.upload_complete` | tool_err | admin |
| files.upload_complete path:id:dotdot | `files.upload_complete` | tool_err | admin |
| files.upload_complete path:id:empty | `files.upload_complete` | tool_err | admin |
| files.upload_complete path:id:array | `files.upload_complete` | tool_err | admin |
| files.upload_complete ghost:id | `files.upload_complete` | fail | admin |
| files.upload_complete analyst deny | `files.upload_complete` | deny | analyst |

## X.files.download

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| files.download ∅args→err | `files.download` | tool_err | admin |
| files.download null:id | `files.download` | tool_err | admin |
| files.download unknown arg | `files.download` | tool_err | admin |
| files.download badtype:id | `files.download` | fail | admin |
| files.download path:id:slash | `files.download` | tool_err | admin |
| files.download path:id:dotdot | `files.download` | tool_err | admin |
| files.download path:id:empty | `files.download` | tool_err | admin |
| files.download path:id:array | `files.download` | tool_err | admin |
| files.download ghost:id | `files.download` | fail | admin |
| files.download analyst deny | `files.download` | deny | analyst |

## X.documents.download

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| documents.download ∅args→err | `documents.download` | tool_err | admin |
| documents.download null:id | `documents.download` | tool_err | admin |
| documents.download null:doc_id | `documents.download` | tool_err | admin |
| documents.download unknown arg | `documents.download` | tool_err | admin |
| documents.download badtype:id | `documents.download` | fail | admin |
| documents.download badtype:doc_id | `documents.download` | fail | admin |
| documents.download path:id:slash | `documents.download` | tool_err | admin |
| documents.download path:id:dotdot | `documents.download` | tool_err | admin |
| documents.download path:id:empty | `documents.download` | tool_err | admin |
| documents.download path:id:array | `documents.download` | tool_err | admin |
| documents.download ghost:id | `documents.download` | fail | admin |
| documents.download path:doc_id:slash | `documents.download` | tool_err | admin |
| documents.download path:doc_id:dotdot | `documents.download` | tool_err | admin |
| documents.download path:doc_id:empty | `documents.download` | tool_err | admin |
| documents.download path:doc_id:array | `documents.download` | tool_err | admin |
| documents.download ghost:doc_id | `documents.download` | fail | admin |
| documents.download analyst reach | `documents.download` | reach | analyst |

## X.tasks.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| tasks.list ∅args→err | `tasks.list` | tool_err | admin |
| tasks.list null:id | `tasks.list` | tool_err | admin |
| tasks.list unknown arg | `tasks.list` | tool_err | admin |
| tasks.list badtype:id | `tasks.list` | fail | admin |
| tasks.list path:id:slash | `tasks.list` | tool_err | admin |
| tasks.list path:id:dotdot | `tasks.list` | tool_err | admin |
| tasks.list path:id:empty | `tasks.list` | tool_err | admin |
| tasks.list path:id:array | `tasks.list` | tool_err | admin |
| tasks.list ghost:id | `tasks.list` | fail | admin |
| tasks.list analyst reach | `tasks.list` | reach | analyst |

## X.tasks.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| tasks.create ∅args→err | `tasks.create` | tool_err | admin |
| tasks.create null:id | `tasks.create` | tool_err | admin |
| tasks.create null:kind | `tasks.create` | tool_err | admin |
| tasks.create unknown arg | `tasks.create` | tool_err | admin |
| tasks.create badtype:id | `tasks.create` | fail | admin |
| tasks.create badtype:kind | `tasks.create` | fail | admin |
| tasks.create emptystr:kind | `tasks.create` | fail | admin |
| tasks.create path:id:slash | `tasks.create` | tool_err | admin |
| tasks.create path:id:dotdot | `tasks.create` | tool_err | admin |
| tasks.create path:id:empty | `tasks.create` | tool_err | admin |
| tasks.create path:id:array | `tasks.create` | tool_err | admin |
| tasks.create ghost:id | `tasks.create` | fail | admin |
| tasks.create analyst deny | `tasks.create` | deny | analyst |

## X.tasks.start

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| tasks.start ∅args→err | `tasks.start` | tool_err | admin |
| tasks.start null:id | `tasks.start` | tool_err | admin |
| tasks.start unknown arg | `tasks.start` | tool_err | admin |
| tasks.start badtype:id | `tasks.start` | fail | admin |
| tasks.start path:id:slash | `tasks.start` | tool_err | admin |
| tasks.start path:id:dotdot | `tasks.start` | tool_err | admin |
| tasks.start path:id:empty | `tasks.start` | tool_err | admin |
| tasks.start path:id:array | `tasks.start` | tool_err | admin |
| tasks.start ghost:id | `tasks.start` | fail | admin |
| tasks.start analyst deny | `tasks.start` | deny | analyst |

## X.tasks.complete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| tasks.complete ∅args→err | `tasks.complete` | tool_err | admin |
| tasks.complete null:id | `tasks.complete` | tool_err | admin |
| tasks.complete unknown arg | `tasks.complete` | tool_err | admin |
| tasks.complete badtype:id | `tasks.complete` | fail | admin |
| tasks.complete path:id:slash | `tasks.complete` | tool_err | admin |
| tasks.complete path:id:dotdot | `tasks.complete` | tool_err | admin |
| tasks.complete path:id:empty | `tasks.complete` | tool_err | admin |
| tasks.complete path:id:array | `tasks.complete` | tool_err | admin |
| tasks.complete ghost:id | `tasks.complete` | fail | admin |
| tasks.complete analyst deny | `tasks.complete` | deny | analyst |

## X.tasks.cancel

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| tasks.cancel ∅args→err | `tasks.cancel` | tool_err | admin |
| tasks.cancel null:id | `tasks.cancel` | tool_err | admin |
| tasks.cancel unknown arg | `tasks.cancel` | tool_err | admin |
| tasks.cancel badtype:id | `tasks.cancel` | fail | admin |
| tasks.cancel path:id:slash | `tasks.cancel` | tool_err | admin |
| tasks.cancel path:id:dotdot | `tasks.cancel` | tool_err | admin |
| tasks.cancel path:id:empty | `tasks.cancel` | tool_err | admin |
| tasks.cancel path:id:array | `tasks.cancel` | tool_err | admin |
| tasks.cancel ghost:id | `tasks.cancel` | fail | admin |
| tasks.cancel analyst deny | `tasks.cancel` | deny | analyst |

## X.factors.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| factors.list ∅args→err | `factors.list` | tool_err | admin |
| factors.list null:id | `factors.list` | tool_err | admin |
| factors.list unknown arg | `factors.list` | tool_err | admin |
| factors.list badtype:id | `factors.list` | fail | admin |
| factors.list path:id:slash | `factors.list` | tool_err | admin |
| factors.list path:id:dotdot | `factors.list` | tool_err | admin |
| factors.list path:id:empty | `factors.list` | tool_err | admin |
| factors.list path:id:array | `factors.list` | tool_err | admin |
| factors.list ghost:id | `factors.list` | fail | admin |
| factors.list analyst reach | `factors.list` | reach | analyst |

## X.factors.resolve

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| factors.resolve ∅args→err | `factors.resolve` | tool_err | admin |
| factors.resolve null:id | `factors.resolve` | tool_err | admin |
| factors.resolve null:factor_id | `factors.resolve` | tool_err | admin |
| factors.resolve unknown arg | `factors.resolve` | tool_err | admin |
| factors.resolve badtype:id | `factors.resolve` | fail | admin |
| factors.resolve badtype:factor_id | `factors.resolve` | fail | admin |
| factors.resolve path:id:slash | `factors.resolve` | tool_err | admin |
| factors.resolve path:id:dotdot | `factors.resolve` | tool_err | admin |
| factors.resolve path:id:empty | `factors.resolve` | tool_err | admin |
| factors.resolve path:id:array | `factors.resolve` | tool_err | admin |
| factors.resolve ghost:id | `factors.resolve` | fail | admin |
| factors.resolve path:factor_id:slash | `factors.resolve` | tool_err | admin |
| factors.resolve path:factor_id:dotdot | `factors.resolve` | tool_err | admin |
| factors.resolve path:factor_id:empty | `factors.resolve` | tool_err | admin |
| factors.resolve path:factor_id:array | `factors.resolve` | tool_err | admin |
| factors.resolve ghost:factor_id | `factors.resolve` | fail | admin |
| factors.resolve analyst deny | `factors.resolve` | deny | analyst |

## X.assessment.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| assessment.get ∅args→err | `assessment.get` | tool_err | admin |
| assessment.get null:id | `assessment.get` | tool_err | admin |
| assessment.get unknown arg | `assessment.get` | tool_err | admin |
| assessment.get badtype:id | `assessment.get` | fail | admin |
| assessment.get path:id:slash | `assessment.get` | tool_err | admin |
| assessment.get path:id:dotdot | `assessment.get` | tool_err | admin |
| assessment.get path:id:empty | `assessment.get` | tool_err | admin |
| assessment.get path:id:array | `assessment.get` | tool_err | admin |
| assessment.get ghost:id | `assessment.get` | fail | admin |
| assessment.get analyst reach | `assessment.get` | reach | analyst |

## X.assessment.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| assessment.create ∅args→err | `assessment.create` | tool_err | admin |
| assessment.create null:id | `assessment.create` | tool_err | admin |
| assessment.create unknown arg | `assessment.create` | tool_err | admin |
| assessment.create badtype:id | `assessment.create` | fail | admin |
| assessment.create path:id:slash | `assessment.create` | tool_err | admin |
| assessment.create path:id:dotdot | `assessment.create` | tool_err | admin |
| assessment.create path:id:empty | `assessment.create` | tool_err | admin |
| assessment.create path:id:array | `assessment.create` | tool_err | admin |
| assessment.create ghost:id | `assessment.create` | fail | admin |
| assessment.create analyst reach | `assessment.create` | reach | analyst |

## X.assessments.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| assessments.list ∅args | `assessments.list` | ok | admin |
| assessments.list unknown arg | `assessments.list` | tool_err | admin |
| assessments.list filter:status | `assessments.list` | ok | admin |
| assessments.list null-opt:status | `assessments.list` | ok | admin |
| assessments.list num:limit | `assessments.list` | ok | admin |
| assessments.list null-opt:limit | `assessments.list` | ok | admin |
| assessments.list filter:cursor | `assessments.list` | ok | admin |
| assessments.list null-opt:cursor | `assessments.list` | ok | admin |
| assessments.list analyst ok | `assessments.list` | ok | analyst |

## X.checks.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| checks.list ∅args | `checks.list` | ok | admin |
| checks.list unknown arg | `checks.list` | tool_err | admin |
| checks.list filter:target_entity_id | `checks.list` | ok | admin |
| checks.list null-opt:target_entity_id | `checks.list` | ok | admin |
| checks.list filter:status | `checks.list` | ok | admin |
| checks.list null-opt:status | `checks.list` | ok | admin |
| checks.list filter:kind | `checks.list` | ok | admin |
| checks.list null-opt:kind | `checks.list` | ok | admin |
| checks.list num:limit | `checks.list` | ok | admin |
| checks.list null-opt:limit | `checks.list` | ok | admin |
| checks.list filter:cursor | `checks.list` | ok | admin |
| checks.list null-opt:cursor | `checks.list` | ok | admin |
| checks.list analyst ok | `checks.list` | ok | analyst |

## X.checks.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| checks.get ∅args→err | `checks.get` | tool_err | admin |
| checks.get null:id | `checks.get` | tool_err | admin |
| checks.get unknown arg | `checks.get` | tool_err | admin |
| checks.get badtype:id | `checks.get` | fail | admin |
| checks.get path:id:slash | `checks.get` | tool_err | admin |
| checks.get path:id:dotdot | `checks.get` | tool_err | admin |
| checks.get path:id:empty | `checks.get` | tool_err | admin |
| checks.get path:id:array | `checks.get` | tool_err | admin |
| checks.get ghost:id | `checks.get` | fail | admin |
| checks.get analyst reach | `checks.get` | reach | analyst |

## X.checks.evidence

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| checks.evidence ∅args→err | `checks.evidence` | tool_err | admin |
| checks.evidence null:id | `checks.evidence` | tool_err | admin |
| checks.evidence unknown arg | `checks.evidence` | tool_err | admin |
| checks.evidence badtype:id | `checks.evidence` | fail | admin |
| checks.evidence path:id:slash | `checks.evidence` | tool_err | admin |
| checks.evidence path:id:dotdot | `checks.evidence` | tool_err | admin |
| checks.evidence path:id:empty | `checks.evidence` | tool_err | admin |
| checks.evidence path:id:array | `checks.evidence` | tool_err | admin |
| checks.evidence ghost:id | `checks.evidence` | fail | admin |
| checks.evidence analyst reach | `checks.evidence` | reach | analyst |

## X.checks.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| checks.create ∅args→err | `checks.create` | tool_err | admin |
| checks.create null:target_entity_id | `checks.create` | tool_err | admin |
| checks.create null:check_kind | `checks.create` | tool_err | admin |
| checks.create unknown arg | `checks.create` | tool_err | admin |
| checks.create badtype:target_entity_id | `checks.create` | fail | admin |
| checks.create badtype:check_kind | `checks.create` | fail | admin |
| checks.create emptystr:target_entity_id | `checks.create` | fail | admin |
| checks.create emptystr:check_kind | `checks.create` | fail | admin |
| checks.create analyst deny | `checks.create` | deny | analyst |

## X.entity_checks.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| entity_checks.list ∅args→err | `entity_checks.list` | tool_err | admin |
| entity_checks.list null:id | `entity_checks.list` | tool_err | admin |
| entity_checks.list unknown arg | `entity_checks.list` | tool_err | admin |
| entity_checks.list badtype:id | `entity_checks.list` | fail | admin |
| entity_checks.list path:id:slash | `entity_checks.list` | tool_err | admin |
| entity_checks.list path:id:dotdot | `entity_checks.list` | tool_err | admin |
| entity_checks.list path:id:empty | `entity_checks.list` | tool_err | admin |
| entity_checks.list path:id:array | `entity_checks.list` | tool_err | admin |
| entity_checks.list ghost:id | `entity_checks.list` | fail | admin |
| entity_checks.list analyst reach | `entity_checks.list` | reach | analyst |

## X.schema.types

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.types ∅args | `schema.types` | ok | admin |
| schema.types unknown arg | `schema.types` | tool_err | admin |
| schema.types analyst ok | `schema.types` | ok | analyst |

## X.schema.type_get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.type_get ∅args→err | `schema.type_get` | tool_err | admin |
| schema.type_get null:alias | `schema.type_get` | tool_err | admin |
| schema.type_get unknown arg | `schema.type_get` | tool_err | admin |
| schema.type_get badtype:alias | `schema.type_get` | fail | admin |
| schema.type_get path:alias:slash | `schema.type_get` | tool_err | admin |
| schema.type_get path:alias:dotdot | `schema.type_get` | tool_err | admin |
| schema.type_get path:alias:empty | `schema.type_get` | tool_err | admin |
| schema.type_get path:alias:array | `schema.type_get` | tool_err | admin |
| schema.type_get ghost:alias | `schema.type_get` | fail | admin |
| schema.type_get analyst reach | `schema.type_get` | reach | analyst |

## X.schema.type_view

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.type_view ∅args→err | `schema.type_view` | tool_err | admin |
| schema.type_view null:alias | `schema.type_view` | tool_err | admin |
| schema.type_view null:view_type | `schema.type_view` | tool_err | admin |
| schema.type_view unknown arg | `schema.type_view` | tool_err | admin |
| schema.type_view badtype:alias | `schema.type_view` | fail | admin |
| schema.type_view badtype:view_type | `schema.type_view` | fail | admin |
| schema.type_view emptystr:view_type | `schema.type_view` | fail | admin |
| schema.type_view path:alias:slash | `schema.type_view` | tool_err | admin |
| schema.type_view path:alias:dotdot | `schema.type_view` | tool_err | admin |
| schema.type_view path:alias:empty | `schema.type_view` | tool_err | admin |
| schema.type_view path:alias:array | `schema.type_view` | tool_err | admin |
| schema.type_view ghost:alias | `schema.type_view` | fail | admin |
| schema.type_view analyst reach | `schema.type_view` | reach | analyst |

## X.schema.predicates

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.predicates ∅args | `schema.predicates` | ok | admin |
| schema.predicates unknown arg | `schema.predicates` | tool_err | admin |
| schema.predicates analyst ok | `schema.predicates` | ok | analyst |

## X.schema.published

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.published ∅args | `schema.published` | ok | admin |
| schema.published unknown arg | `schema.published` | tool_err | admin |
| schema.published analyst ok | `schema.published` | ok | analyst |

## X.schema.drafts

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.drafts ∅args | `schema.drafts` | ok | admin |
| schema.drafts unknown arg | `schema.drafts` | tool_err | admin |
| schema.drafts analyst deny | `schema.drafts` | deny | analyst |

## X.schema.draft_get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.draft_get ∅args→err | `schema.draft_get` | tool_err | admin |
| schema.draft_get null:id | `schema.draft_get` | tool_err | admin |
| schema.draft_get unknown arg | `schema.draft_get` | tool_err | admin |
| schema.draft_get badtype:id | `schema.draft_get` | fail | admin |
| schema.draft_get path:id:slash | `schema.draft_get` | tool_err | admin |
| schema.draft_get path:id:dotdot | `schema.draft_get` | tool_err | admin |
| schema.draft_get path:id:empty | `schema.draft_get` | tool_err | admin |
| schema.draft_get path:id:array | `schema.draft_get` | tool_err | admin |
| schema.draft_get ghost:id | `schema.draft_get` | fail | admin |
| schema.draft_get analyst deny | `schema.draft_get` | deny | analyst |

## X.schema.draft_create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.draft_create ∅args→err | `schema.draft_create` | tool_err | admin |
| schema.draft_create null:name | `schema.draft_create` | tool_err | admin |
| schema.draft_create unknown arg | `schema.draft_create` | tool_err | admin |
| schema.draft_create badtype:name | `schema.draft_create` | fail | admin |
| schema.draft_create emptystr:name | `schema.draft_create` | fail | admin |
| schema.draft_create analyst deny | `schema.draft_create` | deny | analyst |

## X.schema.draft_update

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.draft_update ∅args→err | `schema.draft_update` | tool_err | admin |
| schema.draft_update null:id | `schema.draft_update` | tool_err | admin |
| schema.draft_update null:display_name | `schema.draft_update` | tool_err | admin |
| schema.draft_update null:semantic_version | `schema.draft_update` | tool_err | admin |
| schema.draft_update unknown arg | `schema.draft_update` | tool_err | admin |
| schema.draft_update badtype:id | `schema.draft_update` | fail | admin |
| schema.draft_update badtype:display_name | `schema.draft_update` | fail | admin |
| schema.draft_update emptystr:display_name | `schema.draft_update` | fail | admin |
| schema.draft_update path:id:slash | `schema.draft_update` | tool_err | admin |
| schema.draft_update path:id:dotdot | `schema.draft_update` | tool_err | admin |
| schema.draft_update path:id:empty | `schema.draft_update` | tool_err | admin |
| schema.draft_update path:id:array | `schema.draft_update` | tool_err | admin |
| schema.draft_update ghost:id | `schema.draft_update` | fail | admin |
| schema.draft_update analyst deny | `schema.draft_update` | deny | analyst |

## X.schema.draft_delete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.draft_delete ∅args→err | `schema.draft_delete` | tool_err | admin |
| schema.draft_delete null:id | `schema.draft_delete` | tool_err | admin |
| schema.draft_delete unknown arg | `schema.draft_delete` | tool_err | admin |
| schema.draft_delete badtype:id | `schema.draft_delete` | fail | admin |
| schema.draft_delete path:id:slash | `schema.draft_delete` | tool_err | admin |
| schema.draft_delete path:id:dotdot | `schema.draft_delete` | tool_err | admin |
| schema.draft_delete path:id:empty | `schema.draft_delete` | tool_err | admin |
| schema.draft_delete path:id:array | `schema.draft_delete` | tool_err | admin |
| schema.draft_delete ghost:id | `schema.draft_delete` | fail | admin |
| schema.draft_delete analyst deny | `schema.draft_delete` | deny | analyst |

## X.schema.draft_validate

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.draft_validate ∅args→err | `schema.draft_validate` | tool_err | admin |
| schema.draft_validate null:id | `schema.draft_validate` | tool_err | admin |
| schema.draft_validate unknown arg | `schema.draft_validate` | tool_err | admin |
| schema.draft_validate badtype:id | `schema.draft_validate` | fail | admin |
| schema.draft_validate path:id:slash | `schema.draft_validate` | tool_err | admin |
| schema.draft_validate path:id:dotdot | `schema.draft_validate` | tool_err | admin |
| schema.draft_validate path:id:empty | `schema.draft_validate` | tool_err | admin |
| schema.draft_validate path:id:array | `schema.draft_validate` | tool_err | admin |
| schema.draft_validate ghost:id | `schema.draft_validate` | fail | admin |
| schema.draft_validate analyst deny | `schema.draft_validate` | deny | analyst |

## X.schema.draft_impact

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.draft_impact ∅args→err | `schema.draft_impact` | tool_err | admin |
| schema.draft_impact null:id | `schema.draft_impact` | tool_err | admin |
| schema.draft_impact unknown arg | `schema.draft_impact` | tool_err | admin |
| schema.draft_impact badtype:id | `schema.draft_impact` | fail | admin |
| schema.draft_impact path:id:slash | `schema.draft_impact` | tool_err | admin |
| schema.draft_impact path:id:dotdot | `schema.draft_impact` | tool_err | admin |
| schema.draft_impact path:id:empty | `schema.draft_impact` | tool_err | admin |
| schema.draft_impact path:id:array | `schema.draft_impact` | tool_err | admin |
| schema.draft_impact ghost:id | `schema.draft_impact` | fail | admin |
| schema.draft_impact analyst deny | `schema.draft_impact` | deny | analyst |

## X.schema.draft_publish

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.draft_publish ∅args→err | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish null:id | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish null:reason | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish unknown arg | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish badtype:id | `schema.draft_publish` | fail | admin |
| schema.draft_publish badtype:reason | `schema.draft_publish` | fail | admin |
| schema.draft_publish emptystr:reason | `schema.draft_publish` | fail | admin |
| schema.draft_publish path:id:slash | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish path:id:dotdot | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish path:id:empty | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish path:id:array | `schema.draft_publish` | tool_err | admin |
| schema.draft_publish ghost:id | `schema.draft_publish` | fail | admin |
| schema.draft_publish analyst deny | `schema.draft_publish` | deny | analyst |

## X.schema.revision_get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.revision_get ∅args→err | `schema.revision_get` | tool_err | admin |
| schema.revision_get null:id | `schema.revision_get` | tool_err | admin |
| schema.revision_get unknown arg | `schema.revision_get` | tool_err | admin |
| schema.revision_get badtype:id | `schema.revision_get` | fail | admin |
| schema.revision_get path:id:slash | `schema.revision_get` | tool_err | admin |
| schema.revision_get path:id:dotdot | `schema.revision_get` | tool_err | admin |
| schema.revision_get path:id:empty | `schema.revision_get` | tool_err | admin |
| schema.revision_get path:id:array | `schema.revision_get` | tool_err | admin |
| schema.revision_get ghost:id | `schema.revision_get` | fail | admin |
| schema.revision_get analyst reach | `schema.revision_get` | reach | analyst |

## X.schema.revision_retire

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.revision_retire ∅args→err | `schema.revision_retire` | tool_err | admin |
| schema.revision_retire null:id | `schema.revision_retire` | tool_err | admin |
| schema.revision_retire unknown arg | `schema.revision_retire` | tool_err | admin |
| schema.revision_retire badtype:id | `schema.revision_retire` | fail | admin |
| schema.revision_retire path:id:slash | `schema.revision_retire` | tool_err | admin |
| schema.revision_retire path:id:dotdot | `schema.revision_retire` | tool_err | admin |
| schema.revision_retire path:id:empty | `schema.revision_retire` | tool_err | admin |
| schema.revision_retire path:id:array | `schema.revision_retire` | tool_err | admin |
| schema.revision_retire ghost:id | `schema.revision_retire` | fail | admin |
| schema.revision_retire analyst deny | `schema.revision_retire` | deny | analyst |

## X.schema.revision_migration

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| schema.revision_migration ∅args→err | `schema.revision_migration` | tool_err | admin |
| schema.revision_migration null:id | `schema.revision_migration` | tool_err | admin |
| schema.revision_migration unknown arg | `schema.revision_migration` | tool_err | admin |
| schema.revision_migration badtype:id | `schema.revision_migration` | fail | admin |
| schema.revision_migration path:id:slash | `schema.revision_migration` | tool_err | admin |
| schema.revision_migration path:id:dotdot | `schema.revision_migration` | tool_err | admin |
| schema.revision_migration path:id:empty | `schema.revision_migration` | tool_err | admin |
| schema.revision_migration path:id:array | `schema.revision_migration` | tool_err | admin |
| schema.revision_migration ghost:id | `schema.revision_migration` | fail | admin |
| schema.revision_migration analyst reach | `schema.revision_migration` | reach | analyst |

## X.requests.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| requests.list ∅args | `requests.list` | ok | admin |
| requests.list unknown arg | `requests.list` | tool_err | admin |
| requests.list filter:status | `requests.list` | ok | admin |
| requests.list null-opt:status | `requests.list` | ok | admin |
| requests.list num:limit | `requests.list` | ok | admin |
| requests.list null-opt:limit | `requests.list` | ok | admin |
| requests.list filter:cursor | `requests.list` | ok | admin |
| requests.list null-opt:cursor | `requests.list` | ok | admin |
| requests.list analyst ok | `requests.list` | ok | analyst |

## X.requests.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| requests.create ∅args→err | `requests.create` | tool_err | admin |
| requests.create null:target_entity_id | `requests.create` | tool_err | admin |
| requests.create null:requested_product | `requests.create` | tool_err | admin |
| requests.create unknown arg | `requests.create` | tool_err | admin |
| requests.create badtype:target_entity_id | `requests.create` | fail | admin |
| requests.create badtype:requested_product | `requests.create` | fail | admin |
| requests.create emptystr:target_entity_id | `requests.create` | fail | admin |
| requests.create emptystr:requested_product | `requests.create` | fail | admin |
| requests.create analyst deny | `requests.create` | deny | analyst |

## X.workflows.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| workflows.get ∅args→err | `workflows.get` | tool_err | admin |
| workflows.get null:id | `workflows.get` | tool_err | admin |
| workflows.get unknown arg | `workflows.get` | tool_err | admin |
| workflows.get badtype:id | `workflows.get` | fail | admin |
| workflows.get path:id:slash | `workflows.get` | tool_err | admin |
| workflows.get path:id:dotdot | `workflows.get` | tool_err | admin |
| workflows.get path:id:empty | `workflows.get` | tool_err | admin |
| workflows.get path:id:array | `workflows.get` | tool_err | admin |
| workflows.get ghost:id | `workflows.get` | fail | admin |
| workflows.get analyst reach | `workflows.get` | reach | analyst |

## X.workflows.tasks

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| workflows.tasks ∅args→err | `workflows.tasks` | tool_err | admin |
| workflows.tasks null:id | `workflows.tasks` | tool_err | admin |
| workflows.tasks unknown arg | `workflows.tasks` | tool_err | admin |
| workflows.tasks badtype:id | `workflows.tasks` | fail | admin |
| workflows.tasks path:id:slash | `workflows.tasks` | tool_err | admin |
| workflows.tasks path:id:dotdot | `workflows.tasks` | tool_err | admin |
| workflows.tasks path:id:empty | `workflows.tasks` | tool_err | admin |
| workflows.tasks path:id:array | `workflows.tasks` | tool_err | admin |
| workflows.tasks ghost:id | `workflows.tasks` | fail | admin |
| workflows.tasks analyst reach | `workflows.tasks` | reach | analyst |

## X.workflows.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| workflows.create ∅args→err | `workflows.create` | tool_err | admin |
| workflows.create null:definition_id | `workflows.create` | tool_err | admin |
| workflows.create null:target_entity_id | `workflows.create` | tool_err | admin |
| workflows.create unknown arg | `workflows.create` | tool_err | admin |
| workflows.create badtype:definition_id | `workflows.create` | fail | admin |
| workflows.create badtype:target_entity_id | `workflows.create` | fail | admin |
| workflows.create emptystr:definition_id | `workflows.create` | fail | admin |
| workflows.create emptystr:target_entity_id | `workflows.create` | fail | admin |
| workflows.create analyst deny | `workflows.create` | deny | analyst |

## X.workflows.transition

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| workflows.transition ∅args→err | `workflows.transition` | tool_err | admin |
| workflows.transition null:id | `workflows.transition` | tool_err | admin |
| workflows.transition null:trigger | `workflows.transition` | tool_err | admin |
| workflows.transition null:to | `workflows.transition` | tool_err | admin |
| workflows.transition unknown arg | `workflows.transition` | tool_err | admin |
| workflows.transition badtype:id | `workflows.transition` | fail | admin |
| workflows.transition badtype:trigger | `workflows.transition` | fail | admin |
| workflows.transition emptystr:trigger | `workflows.transition` | fail | admin |
| workflows.transition path:id:slash | `workflows.transition` | tool_err | admin |
| workflows.transition path:id:dotdot | `workflows.transition` | tool_err | admin |
| workflows.transition path:id:empty | `workflows.transition` | tool_err | admin |
| workflows.transition path:id:array | `workflows.transition` | tool_err | admin |
| workflows.transition ghost:id | `workflows.transition` | fail | admin |
| workflows.transition analyst deny | `workflows.transition` | deny | analyst |

## X.monitors.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| monitors.list ∅args | `monitors.list` | ok | admin |
| monitors.list unknown arg | `monitors.list` | tool_err | admin |
| monitors.list analyst deny | `monitors.list` | deny | analyst |

## X.monitors.run

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| monitors.run ∅args→err | `monitors.run` | tool_err | admin |
| monitors.run null:id | `monitors.run` | tool_err | admin |
| monitors.run unknown arg | `monitors.run` | tool_err | admin |
| monitors.run badtype:id | `monitors.run` | fail | admin |
| monitors.run path:id:slash | `monitors.run` | tool_err | admin |
| monitors.run path:id:dotdot | `monitors.run` | tool_err | admin |
| monitors.run path:id:empty | `monitors.run` | tool_err | admin |
| monitors.run path:id:array | `monitors.run` | tool_err | admin |
| monitors.run ghost:id | `monitors.run` | fail | admin |
| monitors.run analyst deny | `monitors.run` | deny | analyst |

## X.reports.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| reports.list ∅args | `reports.list` | ok | admin |
| reports.list unknown arg | `reports.list` | tool_err | admin |
| reports.list analyst ok | `reports.list` | ok | analyst |

## X.reports.generate

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| reports.generate ∅args→err | `reports.generate` | tool_err | admin |
| reports.generate null:entity_id | `reports.generate` | tool_err | admin |
| reports.generate unknown arg | `reports.generate` | tool_err | admin |
| reports.generate badtype:entity_id | `reports.generate` | fail | admin |
| reports.generate emptystr:entity_id | `reports.generate` | fail | admin |
| reports.generate analyst deny | `reports.generate` | deny | analyst |

## X.connectors.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.list ∅args | `connectors.list` | ok | admin |
| connectors.list unknown arg | `connectors.list` | tool_err | admin |
| connectors.list analyst deny | `connectors.list` | deny | analyst |

## X.connectors.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.get ∅args→err | `connectors.get` | tool_err | admin |
| connectors.get null:id | `connectors.get` | tool_err | admin |
| connectors.get unknown arg | `connectors.get` | tool_err | admin |
| connectors.get badtype:id | `connectors.get` | fail | admin |
| connectors.get path:id:slash | `connectors.get` | tool_err | admin |
| connectors.get path:id:dotdot | `connectors.get` | tool_err | admin |
| connectors.get path:id:empty | `connectors.get` | tool_err | admin |
| connectors.get path:id:array | `connectors.get` | tool_err | admin |
| connectors.get ghost:id | `connectors.get` | fail | admin |
| connectors.get analyst deny | `connectors.get` | deny | analyst |

## X.connectors.update

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.update ∅args→err | `connectors.update` | tool_err | admin |
| connectors.update null:id | `connectors.update` | tool_err | admin |
| connectors.update unknown arg | `connectors.update` | tool_err | admin |
| connectors.update badtype:id | `connectors.update` | fail | admin |
| connectors.update path:id:slash | `connectors.update` | tool_err | admin |
| connectors.update path:id:dotdot | `connectors.update` | tool_err | admin |
| connectors.update path:id:empty | `connectors.update` | tool_err | admin |
| connectors.update path:id:array | `connectors.update` | tool_err | admin |
| connectors.update ghost:id | `connectors.update` | fail | admin |
| connectors.update analyst deny | `connectors.update` | deny | analyst |

## X.connectors.delete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.delete ∅args→err | `connectors.delete` | tool_err | admin |
| connectors.delete null:id | `connectors.delete` | tool_err | admin |
| connectors.delete unknown arg | `connectors.delete` | tool_err | admin |
| connectors.delete badtype:id | `connectors.delete` | fail | admin |
| connectors.delete path:id:slash | `connectors.delete` | tool_err | admin |
| connectors.delete path:id:dotdot | `connectors.delete` | tool_err | admin |
| connectors.delete path:id:empty | `connectors.delete` | tool_err | admin |
| connectors.delete path:id:array | `connectors.delete` | tool_err | admin |
| connectors.delete ghost:id | `connectors.delete` | fail | admin |
| connectors.delete analyst deny | `connectors.delete` | deny | analyst |

## X.connectors.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.create ∅args→err | `connectors.create` | tool_err | admin |
| connectors.create null:connector_type_id | `connectors.create` | tool_err | admin |
| connectors.create unknown arg | `connectors.create` | tool_err | admin |
| connectors.create badtype:connector_type_id | `connectors.create` | fail | admin |
| connectors.create emptystr:connector_type_id | `connectors.create` | fail | admin |
| connectors.create analyst deny | `connectors.create` | deny | analyst |

## X.connectors.enable

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.enable ∅args→err | `connectors.enable` | tool_err | admin |
| connectors.enable null:id | `connectors.enable` | tool_err | admin |
| connectors.enable unknown arg | `connectors.enable` | tool_err | admin |
| connectors.enable badtype:id | `connectors.enable` | fail | admin |
| connectors.enable path:id:slash | `connectors.enable` | tool_err | admin |
| connectors.enable path:id:dotdot | `connectors.enable` | tool_err | admin |
| connectors.enable path:id:empty | `connectors.enable` | tool_err | admin |
| connectors.enable path:id:array | `connectors.enable` | tool_err | admin |
| connectors.enable ghost:id | `connectors.enable` | fail | admin |
| connectors.enable analyst deny | `connectors.enable` | deny | analyst |

## X.connectors.disable

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.disable ∅args→err | `connectors.disable` | tool_err | admin |
| connectors.disable null:id | `connectors.disable` | tool_err | admin |
| connectors.disable unknown arg | `connectors.disable` | tool_err | admin |
| connectors.disable badtype:id | `connectors.disable` | fail | admin |
| connectors.disable path:id:slash | `connectors.disable` | tool_err | admin |
| connectors.disable path:id:dotdot | `connectors.disable` | tool_err | admin |
| connectors.disable path:id:empty | `connectors.disable` | tool_err | admin |
| connectors.disable path:id:array | `connectors.disable` | tool_err | admin |
| connectors.disable ghost:id | `connectors.disable` | fail | admin |
| connectors.disable analyst deny | `connectors.disable` | deny | analyst |

## X.connectors.health

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| connectors.health ∅args→err | `connectors.health` | tool_err | admin |
| connectors.health null:id | `connectors.health` | tool_err | admin |
| connectors.health unknown arg | `connectors.health` | tool_err | admin |
| connectors.health badtype:id | `connectors.health` | fail | admin |
| connectors.health path:id:slash | `connectors.health` | tool_err | admin |
| connectors.health path:id:dotdot | `connectors.health` | tool_err | admin |
| connectors.health path:id:empty | `connectors.health` | tool_err | admin |
| connectors.health path:id:array | `connectors.health` | tool_err | admin |
| connectors.health ghost:id | `connectors.health` | fail | admin |
| connectors.health analyst deny | `connectors.health` | deny | analyst |

## X.audit.query

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| audit.query ∅args | `audit.query` | ok | admin |
| audit.query unknown arg | `audit.query` | tool_err | admin |
| audit.query filter:kind | `audit.query` | ok | admin |
| audit.query null-opt:kind | `audit.query` | ok | admin |
| audit.query filter:actor | `audit.query` | ok | admin |
| audit.query null-opt:actor | `audit.query` | ok | admin |
| audit.query filter:action | `audit.query` | ok | admin |
| audit.query null-opt:action | `audit.query` | ok | admin |
| audit.query filter:resource_kind | `audit.query` | ok | admin |
| audit.query null-opt:resource_kind | `audit.query` | ok | admin |
| audit.query filter:resource_id | `audit.query` | ok | admin |
| audit.query null-opt:resource_id | `audit.query` | ok | admin |
| audit.query filter:effect | `audit.query` | ok | admin |
| audit.query null-opt:effect | `audit.query` | ok | admin |
| audit.query filter:from | `audit.query` | ok | admin |
| audit.query null-opt:from | `audit.query` | ok | admin |
| audit.query filter:to | `audit.query` | ok | admin |
| audit.query null-opt:to | `audit.query` | ok | admin |
| audit.query num:limit | `audit.query` | ok | admin |
| audit.query null-opt:limit | `audit.query` | ok | admin |
| audit.query filter:cursor | `audit.query` | ok | admin |
| audit.query null-opt:cursor | `audit.query` | ok | admin |
| audit.query analyst deny | `audit.query` | deny | analyst |

## X.audit.checkpoint_get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| audit.checkpoint_get ∅args | `audit.checkpoint_get` | ok | admin |
| audit.checkpoint_get unknown arg | `audit.checkpoint_get` | tool_err | admin |
| audit.checkpoint_get analyst deny | `audit.checkpoint_get` | deny | analyst |

## X.audit.checkpoint_create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| audit.checkpoint_create ∅args | `audit.checkpoint_create` | ok | admin |
| audit.checkpoint_create unknown arg | `audit.checkpoint_create` | tool_err | admin |
| audit.checkpoint_create analyst deny | `audit.checkpoint_create` | deny | analyst |

## X.audit.verify

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| audit.verify ∅args | `audit.verify` | ok | admin |
| audit.verify unknown arg | `audit.verify` | tool_err | admin |
| audit.verify analyst deny | `audit.verify` | deny | analyst |

## X.audit.retention_get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| audit.retention_get ∅args | `audit.retention_get` | ok | admin |
| audit.retention_get unknown arg | `audit.retention_get` | tool_err | admin |
| audit.retention_get analyst deny | `audit.retention_get` | deny | analyst |

## X.audit.retention_set

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| audit.retention_set ∅args→err | `audit.retention_set` | tool_err | admin |
| audit.retention_set null:retention_days | `audit.retention_set` | tool_err | admin |
| audit.retention_set unknown arg | `audit.retention_set` | tool_err | admin |
| audit.retention_set badtype:retention_days | `audit.retention_set` | fail | admin |
| audit.retention_set analyst deny | `audit.retention_set` | deny | analyst |

## X.permissions.groups

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| permissions.groups ∅args | `permissions.groups` | ok | admin |
| permissions.groups unknown arg | `permissions.groups` | tool_err | admin |
| permissions.groups analyst deny | `permissions.groups` | deny | analyst |

## X.permissions.group_create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| permissions.group_create ∅args→err | `permissions.group_create` | tool_err | admin |
| permissions.group_create null:name | `permissions.group_create` | tool_err | admin |
| permissions.group_create null:class | `permissions.group_create` | tool_err | admin |
| permissions.group_create unknown arg | `permissions.group_create` | tool_err | admin |
| permissions.group_create badtype:name | `permissions.group_create` | fail | admin |
| permissions.group_create badtype:class | `permissions.group_create` | fail | admin |
| permissions.group_create emptystr:name | `permissions.group_create` | fail | admin |
| permissions.group_create emptystr:class | `permissions.group_create` | fail | admin |
| permissions.group_create analyst deny | `permissions.group_create` | deny | analyst |

## X.permissions.group_update

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| permissions.group_update ∅args→err | `permissions.group_update` | tool_err | admin |
| permissions.group_update null:id | `permissions.group_update` | tool_err | admin |
| permissions.group_update null:name | `permissions.group_update` | tool_err | admin |
| permissions.group_update unknown arg | `permissions.group_update` | tool_err | admin |
| permissions.group_update badtype:id | `permissions.group_update` | fail | admin |
| permissions.group_update badtype:name | `permissions.group_update` | fail | admin |
| permissions.group_update emptystr:name | `permissions.group_update` | fail | admin |
| permissions.group_update path:id:slash | `permissions.group_update` | tool_err | admin |
| permissions.group_update path:id:dotdot | `permissions.group_update` | tool_err | admin |
| permissions.group_update path:id:empty | `permissions.group_update` | tool_err | admin |
| permissions.group_update path:id:array | `permissions.group_update` | tool_err | admin |
| permissions.group_update ghost:id | `permissions.group_update` | fail | admin |
| permissions.group_update analyst deny | `permissions.group_update` | deny | analyst |

## X.permissions.group_delete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| permissions.group_delete ∅args→err | `permissions.group_delete` | tool_err | admin |
| permissions.group_delete null:id | `permissions.group_delete` | tool_err | admin |
| permissions.group_delete unknown arg | `permissions.group_delete` | tool_err | admin |
| permissions.group_delete badtype:id | `permissions.group_delete` | fail | admin |
| permissions.group_delete path:id:slash | `permissions.group_delete` | tool_err | admin |
| permissions.group_delete path:id:dotdot | `permissions.group_delete` | tool_err | admin |
| permissions.group_delete path:id:empty | `permissions.group_delete` | tool_err | admin |
| permissions.group_delete path:id:array | `permissions.group_delete` | tool_err | admin |
| permissions.group_delete ghost:id | `permissions.group_delete` | fail | admin |
| permissions.group_delete analyst deny | `permissions.group_delete` | deny | analyst |

## X.users.role_groups

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.role_groups ∅args | `users.role_groups` | ok | admin |
| users.role_groups unknown arg | `users.role_groups` | tool_err | admin |
| users.role_groups analyst deny | `users.role_groups` | deny | analyst |

## X.users.list

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.list ∅args | `users.list` | ok | admin |
| users.list unknown arg | `users.list` | tool_err | admin |
| users.list filter:q | `users.list` | ok | admin |
| users.list null-opt:q | `users.list` | ok | admin |
| users.list filter:group_id | `users.list` | ok | admin |
| users.list null-opt:group_id | `users.list` | ok | admin |
| users.list filter:inherited | `users.list` | ok | admin |
| users.list null-opt:inherited | `users.list` | ok | admin |
| users.list filter:ip | `users.list` | ok | admin |
| users.list null-opt:ip | `users.list` | ok | admin |
| users.list filter:status | `users.list` | ok | admin |
| users.list null-opt:status | `users.list` | ok | admin |
| users.list num:limit | `users.list` | ok | admin |
| users.list null-opt:limit | `users.list` | ok | admin |
| users.list analyst deny | `users.list` | deny | analyst |

## X.users.get

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.get ∅args→err | `users.get` | tool_err | admin |
| users.get null:id | `users.get` | tool_err | admin |
| users.get unknown arg | `users.get` | tool_err | admin |
| users.get badtype:id | `users.get` | fail | admin |
| users.get path:id:slash | `users.get` | tool_err | admin |
| users.get path:id:dotdot | `users.get` | tool_err | admin |
| users.get path:id:empty | `users.get` | tool_err | admin |
| users.get path:id:array | `users.get` | tool_err | admin |
| users.get ghost:id | `users.get` | fail | admin |
| users.get analyst deny | `users.get` | deny | analyst |

## X.users.create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.create ∅args→err | `users.create` | tool_err | admin |
| users.create null:login | `users.create` | tool_err | admin |
| users.create null:display_name | `users.create` | tool_err | admin |
| users.create unknown arg | `users.create` | tool_err | admin |
| users.create badtype:login | `users.create` | fail | admin |
| users.create badtype:display_name | `users.create` | fail | admin |
| users.create emptystr:login | `users.create` | fail | admin |
| users.create emptystr:display_name | `users.create` | fail | admin |
| users.create analyst deny | `users.create` | deny | analyst |

## X.users.update

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.update ∅args→err | `users.update` | tool_err | admin |
| users.update null:id | `users.update` | tool_err | admin |
| users.update unknown arg | `users.update` | tool_err | admin |
| users.update badtype:id | `users.update` | fail | admin |
| users.update path:id:slash | `users.update` | tool_err | admin |
| users.update path:id:dotdot | `users.update` | tool_err | admin |
| users.update path:id:empty | `users.update` | tool_err | admin |
| users.update path:id:array | `users.update` | tool_err | admin |
| users.update ghost:id | `users.update` | fail | admin |
| users.update analyst deny | `users.update` | deny | analyst |

## X.users.delete

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.delete ∅args→err | `users.delete` | tool_err | admin |
| users.delete null:id | `users.delete` | tool_err | admin |
| users.delete unknown arg | `users.delete` | tool_err | admin |
| users.delete badtype:id | `users.delete` | fail | admin |
| users.delete path:id:slash | `users.delete` | tool_err | admin |
| users.delete path:id:dotdot | `users.delete` | tool_err | admin |
| users.delete path:id:empty | `users.delete` | tool_err | admin |
| users.delete path:id:array | `users.delete` | tool_err | admin |
| users.delete ghost:id | `users.delete` | fail | admin |
| users.delete analyst deny | `users.delete` | deny | analyst |

## X.users.activity

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.activity ∅args→err | `users.activity` | tool_err | admin |
| users.activity null:id | `users.activity` | tool_err | admin |
| users.activity unknown arg | `users.activity` | tool_err | admin |
| users.activity badtype:id | `users.activity` | fail | admin |
| users.activity path:id:slash | `users.activity` | tool_err | admin |
| users.activity path:id:dotdot | `users.activity` | tool_err | admin |
| users.activity path:id:empty | `users.activity` | tool_err | admin |
| users.activity path:id:array | `users.activity` | tool_err | admin |
| users.activity ghost:id | `users.activity` | fail | admin |
| users.activity analyst deny | `users.activity` | deny | analyst |

## X.users.tokens

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.tokens ∅args→err | `users.tokens` | tool_err | admin |
| users.tokens null:id | `users.tokens` | tool_err | admin |
| users.tokens unknown arg | `users.tokens` | tool_err | admin |
| users.tokens badtype:id | `users.tokens` | fail | admin |
| users.tokens path:id:slash | `users.tokens` | tool_err | admin |
| users.tokens path:id:dotdot | `users.tokens` | tool_err | admin |
| users.tokens path:id:empty | `users.tokens` | tool_err | admin |
| users.tokens path:id:array | `users.tokens` | tool_err | admin |
| users.tokens ghost:id | `users.tokens` | fail | admin |
| users.tokens analyst deny | `users.tokens` | deny | analyst |

## X.users.token_create

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.token_create ∅args→err | `users.token_create` | tool_err | admin |
| users.token_create null:id | `users.token_create` | tool_err | admin |
| users.token_create null:name | `users.token_create` | tool_err | admin |
| users.token_create unknown arg | `users.token_create` | tool_err | admin |
| users.token_create badtype:id | `users.token_create` | fail | admin |
| users.token_create badtype:name | `users.token_create` | fail | admin |
| users.token_create emptystr:name | `users.token_create` | fail | admin |
| users.token_create path:id:slash | `users.token_create` | tool_err | admin |
| users.token_create path:id:dotdot | `users.token_create` | tool_err | admin |
| users.token_create path:id:empty | `users.token_create` | tool_err | admin |
| users.token_create path:id:array | `users.token_create` | tool_err | admin |
| users.token_create ghost:id | `users.token_create` | fail | admin |
| users.token_create analyst deny | `users.token_create` | deny | analyst |

## X.users.token_revoke

| Кейс | Tool | Ожидание | Токен |
|---|---|---|---|
| users.token_revoke ∅args→err | `users.token_revoke` | tool_err | admin |
| users.token_revoke null:id | `users.token_revoke` | tool_err | admin |
| users.token_revoke null:token_id | `users.token_revoke` | tool_err | admin |
| users.token_revoke unknown arg | `users.token_revoke` | tool_err | admin |
| users.token_revoke badtype:id | `users.token_revoke` | fail | admin |
| users.token_revoke badtype:token_id | `users.token_revoke` | fail | admin |
| users.token_revoke path:id:slash | `users.token_revoke` | tool_err | admin |
| users.token_revoke path:id:dotdot | `users.token_revoke` | tool_err | admin |
| users.token_revoke path:id:empty | `users.token_revoke` | tool_err | admin |
| users.token_revoke path:id:array | `users.token_revoke` | tool_err | admin |
| users.token_revoke ghost:id | `users.token_revoke` | fail | admin |
| users.token_revoke path:token_id:slash | `users.token_revoke` | tool_err | admin |
| users.token_revoke path:token_id:dotdot | `users.token_revoke` | tool_err | admin |
| users.token_revoke path:token_id:empty | `users.token_revoke` | tool_err | admin |
| users.token_revoke path:token_id:array | `users.token_revoke` | tool_err | admin |
| users.token_revoke ghost:token_id | `users.token_revoke` | fail | admin |
| users.token_revoke analyst deny | `users.token_revoke` | deny | analyst |
