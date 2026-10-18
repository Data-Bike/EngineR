# Warehouse «ТЕСТ» — каталог тест-кейсов MCP

Всего кейсов: **1257** (пройдено 1257, инструментов 95)

Запуск: `python docs/testing/warehouse/wh_test_runner.py` (после `bootstrap.py` и `provision.py`). Все кейсы выполняются только через MCP `tools/call`.

## WH1.schema — Схема: опубликованный каталог склада (types/predicates) (12 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH1-01 published rev contains wh types | `schema.published` | ok | pass |
| WH1-02 types list has wh set | `schema.types` | ok | pass |
| WH1-03 type_get wh_product | `schema.type_get` | ok | pass |
| WH1-04 type_get wh_warehouse | `schema.type_get` | ok | pass |
| WH1-05 predicates include wh edges | `schema.predicates` | ok | pass |
| WH1-06 predicates include wh scalars | `schema.predicates` | ok | pass |
| WH1-07 type_get wh_inventory | `schema.type_get` | ok | pass |
| WH1-08 type_view wh_warehouse form | `schema.type_view` | ok | pass |
| WH1-09 revision_get warehouse rev | `schema.revision_get` | ok | pass |
| WH1-10 type_get ghost alias → err | `schema.type_get` | fail | pass |
| WH1-11 drafts list reachable | `schema.drafts` | ok | pass |
| WH1-12 schema.types analyst readable | `schema.types` | ok | pass |

## WH10.negatives — Негативные кейсы: валидация, прав доступа, границы (18 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH10-01 get traversal | `entities.get` | tool_err | pass |
| WH10-02 get ghost | `entities.get` | fail | pass |
| WH10-03 get malformed | `entities.get` | fail | pass |
| WH10-04 create missing type | `entities.create` | tool_err | pass |
| WH10-05 update ghost | `entities.update` | fail | pass |
| WH10-06 retire ghost | `entities.retire` | fail | pass |
| WH10-07 merge ghost pair | `entities.merge` | fail | pass |
| WH10-08 relations missing pred | `relations.create` | tool_err | pass |
| WH10-09 query non-object → 422 | `entities.query` | fail | pass |
| WH10-10 query null → err | `entities.query` | tool_err | pass |
| WH10-11 draft_update no etag → fail | `schema.draft_update` | fail | pass |
| WH10-12 task complete ghost | `tasks.complete` | fail | pass |
| WH10-13 report ghost | `reports.generate` | fail | pass |
| WH10-14 doc upload ghost | `documents.upload` | fail | pass |
| WH10-15 no token → 401 | `__raw__` | http401 | pass |
| WH10-16 garbage token → 401 | `__raw__` | http401 | pass |
| WH10-17 unknown tool → -32602 | `__rpc__` | fail | pass |
| WH10-18 unknown method → -32601 | `__rpc__` | fail | pass |

## WH11.coverage — Добивка покрытия: list/read инструменты (95 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH11-E1 list wh_zone | `entities.list` | ok | pass |
| WH11-E2 list wh_stock | `entities.list` | ok | pass |
| WH11-E3 list wh_inbound | `entities.list` | ok | pass |
| WH11-E4 search Клей | `entities.list` | ok | pass |
| WH11-E5 limit 2 | `entities.list` | ok | pass |
| WH11-E6 analyst list wh | `entities.list` | ok | pass |
| WH11-E7 operator list | `entities.list` | ok | pass |
| WH11-S1 types analyst | `schema.types` | ok | pass |
| WH11-S2 types operator | `schema.types` | ok | pass |
| WH11-S3 predicates analyst | `schema.predicates` | ok | pass |
| WH11-S4 predicates operator | `schema.predicates` | ok | pass |
| WH11-S5 published analyst | `schema.published` | ok | pass |
| WH11-S6 published operator | `schema.published` | ok | pass |
| WH11-S7 published 2nd call | `schema.published` | ok | pass |
| WH11-S8 drafts analyst deny | `schema.drafts` | deny | pass |
| WH11-S9 drafts operator deny | `schema.drafts` | deny | pass |
| WH11-S10 drafts admin 2nd | `schema.drafts` | ok | pass |
| WH11-C1 checks all | `checks.list` | ok | pass |
| WH11-C2 status pending | `checks.list` | ok | pass |
| WH11-C3 kind kyc | `checks.list` | ok | pass |
| WH11-C4 limit 1 | `checks.list` | ok | pass |
| WH11-C5 target wh2 | `checks.list` | ok | pass |
| WH11-C6 target ghost → 404 | `checks.list` | fail | pass |
| WH11-C7 analyst checks | `checks.list` | ok | pass |
| WH11-Q1 requests all | `requests.list` | ok | pass |
| WH11-Q2 status filter | `requests.list` | ok | pass |
| WH11-Q3 limit 1 | `requests.list` | ok | pass |
| WH11-Q4 analyst | `requests.list` | ok | pass |
| WH11-Q5 operator | `requests.list` | ok | pass |
| WH11-Q6 cursor probe | `requests.list` | ok | pass |
| WH11-R1 reports.list | `reports.list` | ok | pass |
| WH11-R2 reports.list analyst | `reports.list` | ok | pass |
| WH11-R3 report wh2 json | `reports.generate` | ok | pass |
| WH11-R4 report product csv | `reports.generate` | ok | pass |
| WH11-R5 report inbound | `reports.generate` | ok | pass |
| WH11-R6 report outbound | `reports.generate` | ok | pass |
| WH11-R7 report inventory | `reports.generate` | ok | pass |
| WH11-R8 report bad format → err | `reports.generate` | fail | pass |
| WH11-R9 report analyst deny | `reports.generate` | deny | pass |
| WH11-A1 assess all | `assessments.list` | ok | pass |
| WH11-A2 status pending | `assessments.list` | ok | pass |
| WH11-A3 limit 1 | `assessments.list` | ok | pass |
| WH11-A4 analyst | `assessments.list` | ok | pass |
| WH11-A5 operator | `assessments.list` | ok | pass |
| WH11-A6 cursor | `assessments.list` | ok | pass |
| WH11-A7 status approved | `assessments.list` | ok | pass |
| WH11-M1 monitors admin | `monitors.list` | ok | pass |
| WH11-M2 monitors analyst deny | `monitors.list` | deny | pass |
| WH11-M3 monitors operator deny | `monitors.list` | deny | pass |
| WH11-M4 monitors 2nd | `monitors.list` | ok | pass |
| WH11-P1 groups admin | `permissions.groups` | ok | pass |
| WH11-P2 groups analyst deny | `permissions.groups` | deny | pass |
| WH11-P3 groups operator deny | `permissions.groups` | deny | pass |
| WH11-P4 groups 2nd | `permissions.groups` | ok | pass |
| WH11-U1 users q=wh | `users.list` | ok | pass |
| WH11-U2 users q=qa | `users.list` | ok | pass |
| WH11-U3 status active | `users.list` | ok | pass |
| WH11-U4 limit 5 | `users.list` | ok | pass |
| WH11-U5 analyst deny | `users.list` | deny | pass |
| WH11-U6 role_groups admin | `users.role_groups` | ok | pass |
| WH11-U7 role_groups 2nd | `users.role_groups` | ok | pass |
| WH11-U8 role_groups deny | `users.role_groups` | deny | pass |
| WH11-D1 audit limit 3 | `audit.query` | ok | pass |
| WH11-D2 audit action create | `audit.query` | ok | pass |
| WH11-D3 audit kind | `audit.query` | ok | pass |
| WH11-D4 audit effect allow | `audit.query` | ok | pass |
| WH11-D5 audit resource entity | `audit.query` | ok | pass |
| WH11-D6 audit analyst deny | `audit.query` | deny | pass |
| WH11-D7 audit from range | `audit.query` | ok | pass |
| WH11-D8 audit to range | `audit.query` | ok | pass |
| WH11-T1 monitors admin 3rd | `monitors.list` | ok | pass |
| WH11-T2 monitors bogus arg | `monitors.list` | tool_err | pass |
| WH11-T3 groups admin 3rd | `permissions.groups` | ok | pass |
| WH11-T4 groups bogus arg | `permissions.groups` | tool_err | pass |
| WH11-T5 reports.list analyst2 | `reports.list` | ok | pass |
| WH11-T6 reports.list operator | `reports.list` | ok | pass |
| WH11-T7 reports.list 3rd | `reports.list` | ok | pass |
| WH11-T8 reports.list bogus | `reports.list` | tool_err | pass |
| WH11-T9 reports.list 4th | `reports.list` | ok | pass |
| WH11-T10 drafts admin 3rd | `schema.drafts` | ok | pass |
| WH11-T11 drafts bogus | `schema.drafts` | tool_err | pass |
| WH11-T12 drafts 4th | `schema.drafts` | ok | pass |
| WH11-T13 predicates admin2 | `schema.predicates` | ok | pass |
| WH11-T14 predicates bogus | `schema.predicates` | tool_err | pass |
| WH11-T15 predicates 3rd | `schema.predicates` | ok | pass |
| WH11-T16 published admin2 | `schema.published` | ok | pass |
| WH11-T17 published bogus | `schema.published` | tool_err | pass |
| WH11-T18 published 3rd | `schema.published` | ok | pass |
| WH11-T19 types admin2 | `schema.types` | ok | pass |
| WH11-T20 types bogus | `schema.types` | tool_err | pass |
| WH11-T21 types 3rd | `schema.types` | ok | pass |
| WH11-T22 users.list q none | `users.list` | ok | pass |
| WH11-T23 role_groups operator deny | `users.role_groups` | deny | pass |
| WH11-T24 role_groups 3rd | `users.role_groups` | ok | pass |
| WH11-T25 role_groups bogus | `users.role_groups` | tool_err | pass |

## WH12.extended — Расширенные операции: аудит, файлы, коннекторы, жизненный цикл схемы (141 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH12-CTX1 context admin | `context.get` | ok | pass |
| WH12-CTX2 context caps | `context.get` | ok | pass |
| WH12-CTX3 context analyst | `context.get` | ok | pass |
| WH12-CTX4 context operator | `context.get` | ok | pass |
| WH12-CTX5 context groups | `context.get` | ok | pass |
| WH12-CTX6 context 2nd | `context.get` | ok | pass |
| WH12-CTX7 context 3rd | `context.get` | ok | pass |
| WH12-DS1 stats admin | `dashboard.stats` | ok | pass |
| WH12-DS2 stats reports | `dashboard.stats` | ok | pass |
| WH12-DS3 stats analyst | `dashboard.stats` | ok | pass |
| WH12-DS4 stats operator | `dashboard.stats` | ok | pass |
| WH12-DS5 stats 2nd | `dashboard.stats` | ok | pass |
| WH12-DS6 stats 3rd | `dashboard.stats` | ok | pass |
| WH12-DS7 stats 4th | `dashboard.stats` | ok | pass |
| WH12-AU1 checkpoint create | `audit.checkpoint_create` | ok | pass |
| WH12-AU2 checkpoint get | `audit.checkpoint_get` | ok | pass |
| WH12-AU3 checkpoint create 2nd | `audit.checkpoint_create` | ok | pass |
| WH12-AU4 checkpoint get 2nd | `audit.checkpoint_get` | ok | pass |
| WH12-AU5 verify full | `audit.verify` | ok | pass |
| WH12-AU6 verify range | `audit.verify` | ok | pass |
| WH12-AU7 verify narrow | `audit.verify` | ok | pass |
| WH12-AU8 verify analyst deny | `audit.verify` | deny | pass |
| WH12-AU9 checkpoint analyst deny | `audit.checkpoint_get` | deny | pass |
| WH12-AU10 retention get | `audit.retention_get` | ok | pass |
| WH12-AU11 retention get analyst deny | `audit.retention_get` | deny | pass |
| WH12-AU12 retention set 365 | `audit.retention_set` | ok | pass |
| WH12-AU13 retention set legal_hold | `audit.retention_set` | ok | pass |
| WH12-AU14 retention set restore | `audit.retention_set` | ok | pass |
| WH12-AU15 retention set analyst deny | `audit.retention_set` | deny | pass |
| WH12-AU16 retention set operator deny | `audit.retention_set` | deny | pass |
| WH12-AU17 verify operator deny | `audit.verify` | deny | pass |
| WH12-DOC1 upload for download | `documents.upload` | ok | pass |
| WH12-DOC2 download doc | `documents.download` | ok | pass |
| WH12-DOC3 download ghost doc | `documents.download` | fail | pass |
| WH12-DOC4 download ghost entity | `documents.download` | fail | pass |
| WH12-DOC5 delete doc | `documents.delete` | ok | pass |
| WH12-DOC6 delete again → err | `documents.delete` | fail | pass |
| WH12-DOC7 delete ghost | `documents.delete` | fail | pass |
| WH12-DOC8 download analyst | `documents.download` | reach | pass |
| WH12-DOC9 delete analyst deny | `documents.delete` | deny | pass |
| WH12-DOC10 upload 2nd doc | `documents.upload` | ok | pass |
| WH12-DOC11 download wh2 | `documents.download` | ok | pass |
| WH12-F1 upload init | `files.upload_init` | ok | pass |
| WH12-F2 upload status | `files.upload_status` | ok | pass |
| WH12-F3 upload chunk 0 | `files.upload_chunk` | ok | pass |
| WH12-F4 status after chunk | `files.upload_status` | ok | pass |
| WH12-F5 upload complete | `files.upload_complete` | ok | pass |
| WH12-F6 files.get | `files.get` | ok | pass |
| WH12-F7 files.download | `files.download` | ok | pass |
| WH12-F8 files.delete | `files.delete` | ok | pass |
| WH12-F9 get deleted → err | `files.get` | fail | pass |
| WH12-F10 init 2nd + abort | `files.upload_init` | ok | pass |
| WH12-F11 upload abort | `files.upload_abort` | ok | pass |
| WH12-F12 status aborted | `files.upload_status` | ok | pass |
| WH12-F13 chunk ghost session | `files.upload_chunk` | fail | pass |
| WH12-F14 complete ghost | `files.upload_complete` | fail | pass |
| WH12-F15 download ghost file | `files.download` | fail | pass |
| WH12-F16 delete ghost file | `files.delete` | fail | pass |
| WH12-F17 status ghost | `files.upload_status` | fail | pass |
| WH12-F18 abort ghost | `files.upload_abort` | fail | pass |
| WH12-F19 init zero size → err | `files.upload_init` | fail | pass |
| WH12-F20 files.get operator | `files.get` | deny | pass |
| WH12-CON1 create | `connectors.create` | ok | pass |
| WH12-CON2 create 2nd | `connectors.create` | ok | pass |
| WH12-CON3 create analyst deny | `connectors.create` | deny | pass |
| WH12-CON3b create operator deny | `connectors.create` | deny | pass |
| WH12-CON4 get | `connectors.get` | ok | pass |
| WH12-CON5 get ghost | `connectors.get` | fail | pass |
| WH12-CON6 list | `connectors.list` | ok | pass |
| WH12-CON7 list 2nd | `connectors.list` | ok | pass |
| WH12-CON8 list analyst | `connectors.list` | deny | pass |
| WH12-CON9 list operator deny | `connectors.list` | deny | pass |
| WH12-CON10 update name | `connectors.update` | ok | pass |
| WH12-CON11 update ghost | `connectors.update` | fail | pass |
| WH12-CON12 update analyst deny | `connectors.update` | deny | pass |
| WH12-CON13 disable | `connectors.disable` | ok | pass |
| WH12-CON14 enable | `connectors.enable` | ok | pass |
| WH12-CON15 health | `connectors.health` | ok | pass |
| WH12-CON16 health ghost | `connectors.health` | fail | pass |
| WH12-CON17 disable ghost | `connectors.disable` | fail | pass |
| WH12-CON18 enable ghost | `connectors.enable` | fail | pass |
| WH12-CON19 delete | `connectors.delete` | ok | pass |
| WH12-CON20 delete ghost | `connectors.delete` | fail | pass |
| WH12-CON21 get deleted → err | `connectors.get` | fail | pass |
| WH12-MON1 monitors list save | `monitors.list` | ok | pass |
| WH12-MON2 run monitor | `monitors.run` | ok | pass |
| WH12-MON3 run ghost | `monitors.run` | fail | pass |
| WH12-MON4 run analyst deny | `monitors.run` | deny | pass |
| WH12-MON5 run operator deny | `monitors.run` | deny | pass |
| WH12-FAC1 create check→factor | `checks.create` | ok | pass |
| WH12-FAC2 factors list | `factors.list` | ok | pass |
| WH12-FAC3 resolve factor | `factors.resolve` | reach | pass |
| WH12-FAC4 resolve ghost factor | `factors.resolve` | fail | pass |
| WH12-FAC5 resolve ghost entity | `factors.resolve` | fail | pass |
| WH12-ASM1 assessment.create | `assessment.create` | ok | pass |
| WH12-ASM2 assessment.get | `assessment.get` | ok | pass |
| WH12-ASM3 assessment.get ghost | `assessment.get` | fail | pass |
| WH12-ASM4 assessment.get analyst | `assessment.get` | reach | pass |
| WH12-SC1 published save | `schema.published` | ok | pass |
| WH12-SC2 draft create | `schema.draft_create` | ok | pass |
| WH12-SC3 draft get | `schema.draft_get` | ok | pass |
| WH12-SC4 draft get save etag | `schema.draft_get` | ok | pass |
| WH12-SC5 draft validate | `schema.draft_validate` | ok | pass |
| WH12-SC6 draft impact | `schema.draft_impact` | ok | pass |
| WH12-SC7 get etag after impact | `schema.draft_get` | ok | pass |
| WH12-SC8 draft delete | `schema.draft_delete` | ok | pass |
| WH12-SC9 draft get after delete | `schema.draft_get` | reach | pass |
| WH12-SC10 draft create analyst deny | `schema.draft_create` | deny | pass |
| WH12-SC11 draft create 2nd | `schema.draft_create` | ok | pass |
| WH12-SC11b draft get etag pre-update | `schema.draft_get` | ok | pass |
| WH12-SC12 draft update ver | `schema.draft_update` | ok | pass |
| WH12-SC13 draft get etag2 | `schema.draft_get` | ok | pass |
| WH12-SC14 draft validate b | `schema.draft_validate` | ok | pass |
| WH12-SC15 draft impact b | `schema.draft_impact` | ok | pass |
| WH12-SC16 draft get etag3 | `schema.draft_get` | ok | pass |
| WH12-SC17 draft publish | `schema.draft_publish` | ok | pass |
| WH12-SC18 revision_get new | `schema.revision_get` | ok | pass |
| WH12-SC19 revision_migration | `schema.revision_migration` | ok | pass |
| WH12-SC20 revision ghost migration | `schema.revision_migration` | fail | pass |
| WH12-SC20b revision_get rev12 etag | `schema.revision_get` | ok | pass |
| WH12-SC21 revision_retire current | `schema.revision_retire` | ok | pass |
| WH12-SC21b restore draft | `schema.draft_create` | ok | pass |
| WH12-SC21c restore etag | `schema.draft_get` | ok | pass |
| WH12-SC21d restore update ver | `schema.draft_update` | ok | pass |
| WH12-SC21e restore validate | `schema.draft_validate` | ok | pass |
| WH12-SC21e2 restore impact | `schema.draft_impact` | ok | pass |
| WH12-SC21e3 restore etag2 | `schema.draft_get` | ok | pass |
| WH12-SC21f restore publish | `schema.draft_publish` | ok | pass |
| WH12-SC22 revision_retire ghost | `schema.revision_retire` | fail | pass |
| WH12-SC23 revision_retire analyst deny | `schema.revision_retire` | deny | pass |
| WH12-PG1 group create temp | `permissions.group_create` | ok | pass |
| WH12-PG2 group delete | `permissions.group_delete` | ok | pass |
| WH12-PG3 delete again → err | `permissions.group_delete` | fail | pass |
| WH12-PG4 delete ghost | `permissions.group_delete` | fail | pass |
| WH12-PG5 delete analyst deny | `permissions.group_delete` | deny | pass |
| WH12-UA1 activity admin self | `users.activity` | ok | pass |
| WH12-UA2 activity op user | `users.activity` | reach | pass |
| WH12-UA3 activity ghost | `users.activity` | fail | pass |
| WH12-UA4 activity limit | `users.activity` | ok | pass |
| WH12-UA5 activity q filter | `users.activity` | ok | pass |
| WH12-UA6 activity analyst deny | `users.activity` | deny | pass |

## WH12.topup — Добивка расширенных инструментов (24 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH12-TA1 checkpoint create 3rd | `audit.checkpoint_create` | ok | pass |
| WH12-TA2 checkpoint create 4th | `audit.checkpoint_create` | ok | pass |
| WH12-TA3 checkpoint create 5th | `audit.checkpoint_create` | ok | pass |
| WH12-TA4 checkpoint create op deny | `audit.checkpoint_create` | deny | pass |
| WH12-TA5 checkpoint create analyst deny | `audit.checkpoint_create` | deny | pass |
| WH12-TB1 checkpoint get 3rd | `audit.checkpoint_get` | ok | pass |
| WH12-TB2 checkpoint get 4th | `audit.checkpoint_get` | ok | pass |
| WH12-TB3 checkpoint get 5th | `audit.checkpoint_get` | ok | pass |
| WH12-TB4 checkpoint get op deny | `audit.checkpoint_get` | deny | pass |
| WH12-TB5 checkpoint get match hash | `audit.checkpoint_get` | ok | pass |
| WH12-TC1 retention get 2nd | `audit.retention_get` | ok | pass |
| WH12-TC2 retention get 3rd | `audit.retention_get` | ok | pass |
| WH12-TC3 retention get legal_hold | `audit.retention_get` | ok | pass |
| WH12-TC4 retention get archive | `audit.retention_get` | ok | pass |
| WH12-TC5 retention get op deny | `audit.retention_get` | deny | pass |
| WH12-TD1 verify to-only | `audit.verify` | ok | pass |
| WH12-TD2 verify checkpoint deny analyst | `audit.verify` | deny | pass |
| WH12-TD3 checkpoint_create reach | `audit.checkpoint_create` | ok | pass |
| WH12-TD4 verify 3rd | `audit.verify` | ok | pass |
| WH12-TE1 connectors.list 3rd | `connectors.list` | ok | pass |
| WH12-TE2 connectors.list 4th | `connectors.list` | ok | pass |
| WH12-TE3 connectors.list match conn | `connectors.list` | ok | pass |
| WH12-TF1 draft_create no-base | `schema.draft_create` | ok | pass |
| WH12-TF2 draft_create operator deny | `schema.draft_create` | deny | pass |

## WH2.objects — Объекты склада: создание, дедупликация, правка, retire (15 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH2-01 create warehouse | `entities.create` | ok | pass |
| WH2-02 get wh1 by id | `entities.get` | ok | pass |
| WH2-03 get wh_zone | `entities.get` | ok | pass |
| WH2-04 create product | `entities.create` | ok | pass |
| WH2-05 dedup same SKU | `entities.create` | ok | pass |
| WH2-06 reject dup SKU | `entities.create` | fail | pass |
| WH2-07 update product name | `entities.update` | ok | pass |
| WH2-08 update stock qty | `entities.update` | ok | pass |
| WH2-09 history after updates | `entities.history` | ok | pass |
| WH2-10 list wh_warehouse | `entities.list` | ok | pass |
| WH2-11 create stock record | `entities.create` | ok | pass |
| WH2-12 retire + status | `entities.retire` | ok | pass |
| WH2-13 get retired → status retired | `entities.get` | ok | pass |
| WH2-14 create wh_zone ghost type → err | `entities.create` | fail | pass |
| WH2-15 analyst read wh | `entities.get` | ok | pass |

## WH3.relations — Связи: ребра склад-зона-остаток-товар-поставщик (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH3-00 new zone for edges | `entities.create` | ok | pass |
| WH3-01 zone→warehouse edge | `relations.create` | ok | pass |
| WH3-02 stock→product edge | `relations.create` | ok | pass |
| WH3-03 stock→zone edge | `relations.create` | ok | pass |
| WH3-04 product→supplier edge | `relations.create` | ok | pass |
| WH3-05 list outgoing stock | `relations.list` | ok | pass |
| WH3-06 list incoming zone | `relations.list` | ok | pass |
| WH3-07 list outgoing inb1 | `relations.list` | ok | pass |
| WH3-08 bad predicate → err | `relations.create` | fail | pass |
| WH3-09 ghost target → err | `relations.create` | fail | pass |
| WH3-10 ghost source → err | `relations.create` | fail | pass |
| WH3-11 delete relation | `relations.delete` | ok | pass |
| WH3-12 list after delete | `relations.list` | ok | pass |

## WH4.search — Поиск: по тексту, полям, типам, связям, AST-запросы (16 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH4-01 query wh_product type | `entities.query` | ok | pass |
| WH4-02 query wh_warehouse type | `entities.query` | ok | pass |
| WH4-03 search text Гвозди | `entities.query` | ok | pass |
| WH4-04 search text Склад ТЕСТ | `entities.query` | ok | pass |
| WH4-05 search miss | `entities.query` | ok | pass |
| WH4-06 filter field eq | `entities.query` | ok | pass |
| WH4-07 where eq sku | `entities.query` | ok | pass |
| WH4-08 where contains name | `entities.query` | ok | pass |
| WH4-09 where gt qty | `entities.query` | ok | pass |
| WH4-10 where and-combo | `entities.query` | ok | pass |
| WH4-11 page size 1 | `entities.query` | ok | pass |
| WH4-12 page after cursor | `entities.query` | ok | pass |
| WH4-13 search by sku text | `entities.query` | ok | pass |
| WH4-14 query ghost type | `entities.query` | ok | pass |
| WH4-15 graph from product | `entities.graph` | ok | pass |
| WH4-16 graph inbound | `entities.graph` | ok | pass |

## WH5.inbound — Сценарий: приёмка (inbound delivery) (14 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH5-01 create inbound doc | `entities.create` | ok | pass |
| WH5-02 link supplier | `relations.create` | ok | pass |
| WH5-03 link warehouse | `relations.create` | ok | pass |
| WH5-04 link item 1 | `relations.create` | ok | pass |
| WH5-05 link item 2 | `relations.create` | ok | pass |
| WH5-06 receive stock | `entities.create` | ok | pass |
| WH5-07 stock→product | `relations.create` | ok | pass |
| WH5-08 stock→rcv zone | `relations.create` | ok | pass |
| WH5-09 set status проведена | `entities.update` | ok | pass |
| WH5-10 workflow on inbound | `workflows.create` | ok | pass |
| WH5-11 transition start_review | `workflows.transition` | ok | pass |
| WH5-12 upload packing list | `documents.upload` | ok | pass |
| WH5-13 documents.list | `documents.list` | ok | pass |
| WH5-14 graph verify inbound | `entities.graph` | ok | pass |

## WH6.outbound — Сценарий: отгрузка (outbound order) (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH6-01 create outbound doc | `entities.create` | ok | pass |
| WH6-02 link customer | `relations.create` | ok | pass |
| WH6-03 link warehouse | `relations.create` | ok | pass |
| WH6-04 link item | `relations.create` | ok | pass |
| WH6-05 decrease stock | `entities.update` | ok | pass |
| WH6-06 picking task | `tasks.create` | ok | pass |
| WH6-07 tasks.list | `tasks.list` | ok | pass |
| WH6-08 task start | `tasks.start` | ok | pass |
| WH6-09 task complete | `tasks.complete` | ok | pass |
| WH6-10 set отгружена | `entities.update` | ok | pass |
| WH6-11 history outbound | `entities.history` | ok | pass |
| WH6-12 report on warehouse | `reports.generate` | ok | pass |
| WH6-13 graph outbound | `entities.graph` | ok | pass |

## WH7.inventory — Сценарий: инвентаризация (count) (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH7-01 create inv act | `entities.create` | ok | pass |
| WH7-02 link warehouse | `relations.create` | ok | pass |
| WH7-03 record diff | `entities.update` | ok | pass |
| WH7-04 check on warehouse | `checks.create` | ok | pass |
| WH7-05 checks.list filtered | `checks.list` | ok | pass |
| WH7-06 checks.get | `checks.get` | ok | pass |
| WH7-07 entity_checks.list | `entity_checks.list` | ok | pass |
| WH7-08 factors.list wh | `factors.list` | ok | pass |
| WH7-09 assessment.create | `assessment.create` | ok | pass |
| WH7-10 assessments.list | `assessments.list` | ok | pass |
| WH7-11 recount task | `tasks.create` | ok | pass |
| WH7-12 upload act scan | `documents.upload` | ok | pass |
| WH7-13 monitors.list | `monitors.list` | ok | pass |

## WH8.process — Процессы: workflow/задачи/отчёты/мониторы (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH8-01 wf create wh2 | `workflows.create` | ok | pass |
| WH8-02 wf get | `workflows.get` | ok | pass |
| WH8-03 wf tasks | `workflows.tasks` | ok | pass |
| WH8-04 wf transition | `workflows.transition` | ok | pass |
| WH8-05 wf bad transition | `workflows.transition` | fail | pass |
| WH8-06 wf unknown def | `workflows.create` | fail | pass |
| WH8-07 request on product | `requests.create` | ok | pass |
| WH8-08 requests.list | `requests.list` | ok | pass |
| WH8-09 task cancel | `tasks.create` | ok | pass |
| WH8-10 cancel it | `tasks.cancel` | ok | pass |
| WH8-11 task ghost start | `tasks.start` | fail | pass |
| WH8-12 wf ghost get | `workflows.get` | fail | pass |
| WH8-13 check evidence | `checks.evidence` | ok | pass |

## WH9.admin — Администрирование: пользователи, роли, токены, аудит (19 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| WH9-01 create wh operator | `users.create` | ok | pass |
| WH9-02 users.list | `users.list` | ok | pass |
| WH9-03 users.get | `users.get` | ok | pass |
| WH9-04 update display | `users.update` | ok | pass |
| WH9-05 users.role_groups | `users.role_groups` | ok | pass |
| WH9-06 group create readers | `permissions.group_create` | ok | pass |
| WH9-07 permissions.groups | `permissions.groups` | ok | pass |
| WH9-08 group update principals | `permissions.group_update` | ok | pass |
| WH9-09 token create | `users.token_create` | ok | pass |
| WH9-10 tokens list | `users.tokens` | ok | pass |
| WH9-11 operator context via MCP | `__call__` | ok | pass |
| WH9-12 operator reads wh1 | `__call__` | ok | pass |
| WH9-13 token revoke | `users.token_revoke` | ok | pass |
| WH9-14 revoked → 401 | `__call__` | ok | pass |
| WH9-15 audit query wh | `audit.query` | ok | pass |
| WH9-16 audit by action | `audit.query` | ok | pass |
| WH9-17 analyst deny users.create | `users.create` | deny | pass |
| WH9-18 users.delete temp | `users.delete` | ok | pass |
| WH9-19 delete ghost user | `users.delete` | fail | pass |

## WHX.assessment.create — Per-tool негативы: WHX.assessment.create (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| assessment.create ∅args→err | `assessment.create` | tool_err | pass |
| assessment.create null:id | `assessment.create` | tool_err | pass |
| assessment.create unknown arg | `assessment.create` | tool_err | pass |
| assessment.create badtype:id | `assessment.create` | fail | pass |
| assessment.create path:id:slash | `assessment.create` | tool_err | pass |
| assessment.create path:id:dotdot | `assessment.create` | tool_err | pass |
| assessment.create path:id:empty | `assessment.create` | tool_err | pass |
| assessment.create path:id:array | `assessment.create` | tool_err | pass |
| assessment.create ghost:id | `assessment.create` | fail | pass |
| assessment.create operator reach | `assessment.create` | reach | pass |

## WHX.assessment.get — Per-tool негативы: WHX.assessment.get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| assessment.get ∅args→err | `assessment.get` | tool_err | pass |
| assessment.get null:id | `assessment.get` | tool_err | pass |
| assessment.get unknown arg | `assessment.get` | tool_err | pass |
| assessment.get badtype:id | `assessment.get` | fail | pass |
| assessment.get path:id:slash | `assessment.get` | tool_err | pass |
| assessment.get path:id:dotdot | `assessment.get` | tool_err | pass |
| assessment.get path:id:empty | `assessment.get` | tool_err | pass |
| assessment.get path:id:array | `assessment.get` | tool_err | pass |
| assessment.get ghost:id | `assessment.get` | fail | pass |
| assessment.get operator reach | `assessment.get` | reach | pass |

## WHX.assessments.list — Per-tool негативы: WHX.assessments.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| assessments.list ∅args | `assessments.list` | ok | pass |
| assessments.list unknown arg | `assessments.list` | tool_err | pass |
| assessments.list operator ok | `assessments.list` | ok | pass |

## WHX.audit.checkpoint_create — Per-tool негативы: WHX.audit.checkpoint_create (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| audit.checkpoint_create ∅args | `audit.checkpoint_create` | ok | pass |
| audit.checkpoint_create unknown arg | `audit.checkpoint_create` | tool_err | pass |
| audit.checkpoint_create operator deny | `audit.checkpoint_create` | deny | pass |

## WHX.audit.checkpoint_get — Per-tool негативы: WHX.audit.checkpoint_get (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| audit.checkpoint_get ∅args | `audit.checkpoint_get` | ok | pass |
| audit.checkpoint_get unknown arg | `audit.checkpoint_get` | tool_err | pass |
| audit.checkpoint_get operator deny | `audit.checkpoint_get` | deny | pass |

## WHX.audit.query — Per-tool негативы: WHX.audit.query (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| audit.query ∅args | `audit.query` | ok | pass |
| audit.query unknown arg | `audit.query` | tool_err | pass |
| audit.query operator deny | `audit.query` | deny | pass |

## WHX.audit.retention_get — Per-tool негативы: WHX.audit.retention_get (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| audit.retention_get ∅args | `audit.retention_get` | ok | pass |
| audit.retention_get unknown arg | `audit.retention_get` | tool_err | pass |
| audit.retention_get operator deny | `audit.retention_get` | deny | pass |

## WHX.audit.retention_set — Per-tool негативы: WHX.audit.retention_set (5 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| audit.retention_set ∅args→err | `audit.retention_set` | tool_err | pass |
| audit.retention_set null:retention_days | `audit.retention_set` | tool_err | pass |
| audit.retention_set unknown arg | `audit.retention_set` | tool_err | pass |
| audit.retention_set badtype:retention_days | `audit.retention_set` | fail | pass |
| audit.retention_set operator deny | `audit.retention_set` | deny | pass |

## WHX.audit.verify — Per-tool негативы: WHX.audit.verify (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| audit.verify ∅args | `audit.verify` | ok | pass |
| audit.verify unknown arg | `audit.verify` | tool_err | pass |
| audit.verify operator deny | `audit.verify` | deny | pass |

## WHX.checks.create — Per-tool негативы: WHX.checks.create (9 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| checks.create ∅args→err | `checks.create` | tool_err | pass |
| checks.create null:target_entity_id | `checks.create` | tool_err | pass |
| checks.create null:check_kind | `checks.create` | tool_err | pass |
| checks.create unknown arg | `checks.create` | tool_err | pass |
| checks.create badtype:target_entity_id | `checks.create` | fail | pass |
| checks.create badtype:check_kind | `checks.create` | fail | pass |
| checks.create emptystr:target_entity_id | `checks.create` | fail | pass |
| checks.create emptystr:check_kind | `checks.create` | fail | pass |
| checks.create operator deny | `checks.create` | deny | pass |

## WHX.checks.evidence — Per-tool негативы: WHX.checks.evidence (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| checks.evidence ∅args→err | `checks.evidence` | tool_err | pass |
| checks.evidence null:id | `checks.evidence` | tool_err | pass |
| checks.evidence unknown arg | `checks.evidence` | tool_err | pass |
| checks.evidence badtype:id | `checks.evidence` | fail | pass |
| checks.evidence path:id:slash | `checks.evidence` | tool_err | pass |
| checks.evidence path:id:dotdot | `checks.evidence` | tool_err | pass |
| checks.evidence path:id:empty | `checks.evidence` | tool_err | pass |
| checks.evidence path:id:array | `checks.evidence` | tool_err | pass |
| checks.evidence ghost:id | `checks.evidence` | fail | pass |
| checks.evidence operator reach | `checks.evidence` | reach | pass |

## WHX.checks.get — Per-tool негативы: WHX.checks.get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| checks.get ∅args→err | `checks.get` | tool_err | pass |
| checks.get null:id | `checks.get` | tool_err | pass |
| checks.get unknown arg | `checks.get` | tool_err | pass |
| checks.get badtype:id | `checks.get` | fail | pass |
| checks.get path:id:slash | `checks.get` | tool_err | pass |
| checks.get path:id:dotdot | `checks.get` | tool_err | pass |
| checks.get path:id:empty | `checks.get` | tool_err | pass |
| checks.get path:id:array | `checks.get` | tool_err | pass |
| checks.get ghost:id | `checks.get` | fail | pass |
| checks.get operator reach | `checks.get` | reach | pass |

## WHX.checks.list — Per-tool негативы: WHX.checks.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| checks.list ∅args | `checks.list` | ok | pass |
| checks.list unknown arg | `checks.list` | tool_err | pass |
| checks.list operator ok | `checks.list` | ok | pass |

## WHX.connectors.create — Per-tool негативы: WHX.connectors.create (6 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.create ∅args→err | `connectors.create` | tool_err | pass |
| connectors.create null:connector_type_id | `connectors.create` | tool_err | pass |
| connectors.create unknown arg | `connectors.create` | tool_err | pass |
| connectors.create badtype:connector_type_id | `connectors.create` | fail | pass |
| connectors.create emptystr:connector_type_id | `connectors.create` | fail | pass |
| connectors.create operator deny | `connectors.create` | deny | pass |

## WHX.connectors.delete — Per-tool негативы: WHX.connectors.delete (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.delete ∅args→err | `connectors.delete` | tool_err | pass |
| connectors.delete null:id | `connectors.delete` | tool_err | pass |
| connectors.delete unknown arg | `connectors.delete` | tool_err | pass |
| connectors.delete badtype:id | `connectors.delete` | fail | pass |
| connectors.delete path:id:slash | `connectors.delete` | tool_err | pass |
| connectors.delete path:id:dotdot | `connectors.delete` | tool_err | pass |
| connectors.delete path:id:empty | `connectors.delete` | tool_err | pass |
| connectors.delete path:id:array | `connectors.delete` | tool_err | pass |
| connectors.delete ghost:id | `connectors.delete` | fail | pass |
| connectors.delete operator deny | `connectors.delete` | deny | pass |

## WHX.connectors.disable — Per-tool негативы: WHX.connectors.disable (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.disable ∅args→err | `connectors.disable` | tool_err | pass |
| connectors.disable null:id | `connectors.disable` | tool_err | pass |
| connectors.disable unknown arg | `connectors.disable` | tool_err | pass |
| connectors.disable badtype:id | `connectors.disable` | fail | pass |
| connectors.disable path:id:slash | `connectors.disable` | tool_err | pass |
| connectors.disable path:id:dotdot | `connectors.disable` | tool_err | pass |
| connectors.disable path:id:empty | `connectors.disable` | tool_err | pass |
| connectors.disable path:id:array | `connectors.disable` | tool_err | pass |
| connectors.disable ghost:id | `connectors.disable` | fail | pass |
| connectors.disable operator deny | `connectors.disable` | deny | pass |

## WHX.connectors.enable — Per-tool негативы: WHX.connectors.enable (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.enable ∅args→err | `connectors.enable` | tool_err | pass |
| connectors.enable null:id | `connectors.enable` | tool_err | pass |
| connectors.enable unknown arg | `connectors.enable` | tool_err | pass |
| connectors.enable badtype:id | `connectors.enable` | fail | pass |
| connectors.enable path:id:slash | `connectors.enable` | tool_err | pass |
| connectors.enable path:id:dotdot | `connectors.enable` | tool_err | pass |
| connectors.enable path:id:empty | `connectors.enable` | tool_err | pass |
| connectors.enable path:id:array | `connectors.enable` | tool_err | pass |
| connectors.enable ghost:id | `connectors.enable` | fail | pass |
| connectors.enable operator deny | `connectors.enable` | deny | pass |

## WHX.connectors.get — Per-tool негативы: WHX.connectors.get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.get ∅args→err | `connectors.get` | tool_err | pass |
| connectors.get null:id | `connectors.get` | tool_err | pass |
| connectors.get unknown arg | `connectors.get` | tool_err | pass |
| connectors.get badtype:id | `connectors.get` | fail | pass |
| connectors.get path:id:slash | `connectors.get` | tool_err | pass |
| connectors.get path:id:dotdot | `connectors.get` | tool_err | pass |
| connectors.get path:id:empty | `connectors.get` | tool_err | pass |
| connectors.get path:id:array | `connectors.get` | tool_err | pass |
| connectors.get ghost:id | `connectors.get` | fail | pass |
| connectors.get operator deny | `connectors.get` | deny | pass |

## WHX.connectors.health — Per-tool негативы: WHX.connectors.health (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.health ∅args→err | `connectors.health` | tool_err | pass |
| connectors.health null:id | `connectors.health` | tool_err | pass |
| connectors.health unknown arg | `connectors.health` | tool_err | pass |
| connectors.health badtype:id | `connectors.health` | fail | pass |
| connectors.health path:id:slash | `connectors.health` | tool_err | pass |
| connectors.health path:id:dotdot | `connectors.health` | tool_err | pass |
| connectors.health path:id:empty | `connectors.health` | tool_err | pass |
| connectors.health path:id:array | `connectors.health` | tool_err | pass |
| connectors.health ghost:id | `connectors.health` | fail | pass |
| connectors.health operator deny | `connectors.health` | deny | pass |

## WHX.connectors.list — Per-tool негативы: WHX.connectors.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.list ∅args | `connectors.list` | ok | pass |
| connectors.list unknown arg | `connectors.list` | tool_err | pass |
| connectors.list operator deny | `connectors.list` | deny | pass |

## WHX.connectors.update — Per-tool негативы: WHX.connectors.update (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| connectors.update ∅args→err | `connectors.update` | tool_err | pass |
| connectors.update null:id | `connectors.update` | tool_err | pass |
| connectors.update unknown arg | `connectors.update` | tool_err | pass |
| connectors.update badtype:id | `connectors.update` | fail | pass |
| connectors.update path:id:slash | `connectors.update` | tool_err | pass |
| connectors.update path:id:dotdot | `connectors.update` | tool_err | pass |
| connectors.update path:id:empty | `connectors.update` | tool_err | pass |
| connectors.update path:id:array | `connectors.update` | tool_err | pass |
| connectors.update ghost:id | `connectors.update` | fail | pass |
| connectors.update operator deny | `connectors.update` | deny | pass |

## WHX.context.get — Per-tool негативы: WHX.context.get (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| context.get ∅args | `context.get` | ok | pass |
| context.get unknown arg | `context.get` | tool_err | pass |
| context.get operator ok | `context.get` | ok | pass |

## WHX.dashboard.stats — Per-tool негативы: WHX.dashboard.stats (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| dashboard.stats ∅args | `dashboard.stats` | ok | pass |
| dashboard.stats unknown arg | `dashboard.stats` | tool_err | pass |
| dashboard.stats operator ok | `dashboard.stats` | ok | pass |

## WHX.documents.delete — Per-tool негативы: WHX.documents.delete (17 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| documents.delete ∅args→err | `documents.delete` | tool_err | pass |
| documents.delete null:id | `documents.delete` | tool_err | pass |
| documents.delete null:doc_id | `documents.delete` | tool_err | pass |
| documents.delete unknown arg | `documents.delete` | tool_err | pass |
| documents.delete badtype:id | `documents.delete` | fail | pass |
| documents.delete badtype:doc_id | `documents.delete` | fail | pass |
| documents.delete path:id:slash | `documents.delete` | tool_err | pass |
| documents.delete path:id:dotdot | `documents.delete` | tool_err | pass |
| documents.delete path:id:empty | `documents.delete` | tool_err | pass |
| documents.delete path:id:array | `documents.delete` | tool_err | pass |
| documents.delete ghost:id | `documents.delete` | fail | pass |
| documents.delete path:doc_id:slash | `documents.delete` | tool_err | pass |
| documents.delete path:doc_id:dotdot | `documents.delete` | tool_err | pass |
| documents.delete path:doc_id:empty | `documents.delete` | tool_err | pass |
| documents.delete path:doc_id:array | `documents.delete` | tool_err | pass |
| documents.delete ghost:doc_id | `documents.delete` | fail | pass |
| documents.delete operator deny | `documents.delete` | deny | pass |

## WHX.documents.download — Per-tool негативы: WHX.documents.download (17 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| documents.download ∅args→err | `documents.download` | tool_err | pass |
| documents.download null:id | `documents.download` | tool_err | pass |
| documents.download null:doc_id | `documents.download` | tool_err | pass |
| documents.download unknown arg | `documents.download` | tool_err | pass |
| documents.download badtype:id | `documents.download` | fail | pass |
| documents.download badtype:doc_id | `documents.download` | fail | pass |
| documents.download path:id:slash | `documents.download` | tool_err | pass |
| documents.download path:id:dotdot | `documents.download` | tool_err | pass |
| documents.download path:id:empty | `documents.download` | tool_err | pass |
| documents.download path:id:array | `documents.download` | tool_err | pass |
| documents.download ghost:id | `documents.download` | fail | pass |
| documents.download path:doc_id:slash | `documents.download` | tool_err | pass |
| documents.download path:doc_id:dotdot | `documents.download` | tool_err | pass |
| documents.download path:doc_id:empty | `documents.download` | tool_err | pass |
| documents.download path:doc_id:array | `documents.download` | tool_err | pass |
| documents.download ghost:doc_id | `documents.download` | fail | pass |
| documents.download operator reach | `documents.download` | reach | pass |

## WHX.documents.list — Per-tool негативы: WHX.documents.list (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| documents.list ∅args→err | `documents.list` | tool_err | pass |
| documents.list null:id | `documents.list` | tool_err | pass |
| documents.list unknown arg | `documents.list` | tool_err | pass |
| documents.list badtype:id | `documents.list` | fail | pass |
| documents.list path:id:slash | `documents.list` | tool_err | pass |
| documents.list path:id:dotdot | `documents.list` | tool_err | pass |
| documents.list path:id:empty | `documents.list` | tool_err | pass |
| documents.list path:id:array | `documents.list` | tool_err | pass |
| documents.list ghost:id | `documents.list` | fail | pass |
| documents.list operator reach | `documents.list` | reach | pass |

## WHX.documents.upload — Per-tool негативы: WHX.documents.upload (14 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| documents.upload ∅args→err | `documents.upload` | tool_err | pass |
| documents.upload null:id | `documents.upload` | tool_err | pass |
| documents.upload null:kind | `documents.upload` | tool_err | pass |
| documents.upload null:file_name | `documents.upload` | tool_err | pass |
| documents.upload unknown arg | `documents.upload` | tool_err | pass |
| documents.upload badtype:id | `documents.upload` | fail | pass |
| documents.upload badtype:kind | `documents.upload` | fail | pass |
| documents.upload emptystr:kind | `documents.upload` | fail | pass |
| documents.upload path:id:slash | `documents.upload` | tool_err | pass |
| documents.upload path:id:dotdot | `documents.upload` | tool_err | pass |
| documents.upload path:id:empty | `documents.upload` | tool_err | pass |
| documents.upload path:id:array | `documents.upload` | tool_err | pass |
| documents.upload ghost:id | `documents.upload` | fail | pass |
| documents.upload operator deny | `documents.upload` | deny | pass |

## WHX.entities.create — Per-tool негативы: WHX.entities.create (6 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.create ∅args→err | `entities.create` | tool_err | pass |
| entities.create null:type_id | `entities.create` | tool_err | pass |
| entities.create unknown arg | `entities.create` | tool_err | pass |
| entities.create badtype:type_id | `entities.create` | fail | pass |
| entities.create emptystr:type_id | `entities.create` | fail | pass |
| entities.create operator deny | `entities.create` | deny | pass |

## WHX.entities.get — Per-tool негативы: WHX.entities.get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.get ∅args→err | `entities.get` | tool_err | pass |
| entities.get null:id | `entities.get` | tool_err | pass |
| entities.get unknown arg | `entities.get` | tool_err | pass |
| entities.get badtype:id | `entities.get` | fail | pass |
| entities.get path:id:slash | `entities.get` | tool_err | pass |
| entities.get path:id:dotdot | `entities.get` | tool_err | pass |
| entities.get path:id:empty | `entities.get` | tool_err | pass |
| entities.get path:id:array | `entities.get` | tool_err | pass |
| entities.get ghost:id | `entities.get` | fail | pass |
| entities.get operator reach | `entities.get` | reach | pass |

## WHX.entities.graph — Per-tool негативы: WHX.entities.graph (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.graph ∅args→err | `entities.graph` | tool_err | pass |
| entities.graph null:id | `entities.graph` | tool_err | pass |
| entities.graph unknown arg | `entities.graph` | tool_err | pass |
| entities.graph badtype:id | `entities.graph` | fail | pass |
| entities.graph path:id:slash | `entities.graph` | tool_err | pass |
| entities.graph path:id:dotdot | `entities.graph` | tool_err | pass |
| entities.graph path:id:empty | `entities.graph` | tool_err | pass |
| entities.graph path:id:array | `entities.graph` | tool_err | pass |
| entities.graph ghost:id | `entities.graph` | fail | pass |
| entities.graph operator reach | `entities.graph` | reach | pass |

## WHX.entities.history — Per-tool негативы: WHX.entities.history (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.history ∅args→err | `entities.history` | tool_err | pass |
| entities.history null:id | `entities.history` | tool_err | pass |
| entities.history unknown arg | `entities.history` | tool_err | pass |
| entities.history badtype:id | `entities.history` | fail | pass |
| entities.history path:id:slash | `entities.history` | tool_err | pass |
| entities.history path:id:dotdot | `entities.history` | tool_err | pass |
| entities.history path:id:empty | `entities.history` | tool_err | pass |
| entities.history path:id:array | `entities.history` | tool_err | pass |
| entities.history ghost:id | `entities.history` | fail | pass |
| entities.history operator reach | `entities.history` | reach | pass |

## WHX.entities.list — Per-tool негативы: WHX.entities.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.list ∅args | `entities.list` | ok | pass |
| entities.list unknown arg | `entities.list` | tool_err | pass |
| entities.list operator ok | `entities.list` | ok | pass |

## WHX.entities.merge — Per-tool негативы: WHX.entities.merge (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.merge ∅args→err | `entities.merge` | tool_err | pass |
| entities.merge null:id | `entities.merge` | tool_err | pass |
| entities.merge null:loser_public_id | `entities.merge` | tool_err | pass |
| entities.merge unknown arg | `entities.merge` | tool_err | pass |
| entities.merge badtype:id | `entities.merge` | fail | pass |
| entities.merge badtype:loser_public_id | `entities.merge` | fail | pass |
| entities.merge emptystr:loser_public_id | `entities.merge` | fail | pass |
| entities.merge path:id:slash | `entities.merge` | tool_err | pass |
| entities.merge path:id:dotdot | `entities.merge` | tool_err | pass |
| entities.merge path:id:empty | `entities.merge` | tool_err | pass |
| entities.merge path:id:array | `entities.merge` | tool_err | pass |
| entities.merge ghost:id | `entities.merge` | fail | pass |
| entities.merge operator deny | `entities.merge` | deny | pass |

## WHX.entities.query — Per-tool негативы: WHX.entities.query (5 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.query ∅args→err | `entities.query` | tool_err | pass |
| entities.query null:query | `entities.query` | tool_err | pass |
| entities.query unknown arg | `entities.query` | tool_err | pass |
| entities.query badtype:query | `entities.query` | fail | pass |
| entities.query operator reach | `entities.query` | reach | pass |

## WHX.entities.retire — Per-tool негативы: WHX.entities.retire (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.retire ∅args→err | `entities.retire` | tool_err | pass |
| entities.retire null:id | `entities.retire` | tool_err | pass |
| entities.retire unknown arg | `entities.retire` | tool_err | pass |
| entities.retire badtype:id | `entities.retire` | fail | pass |
| entities.retire path:id:slash | `entities.retire` | tool_err | pass |
| entities.retire path:id:dotdot | `entities.retire` | tool_err | pass |
| entities.retire path:id:empty | `entities.retire` | tool_err | pass |
| entities.retire path:id:array | `entities.retire` | tool_err | pass |
| entities.retire ghost:id | `entities.retire` | fail | pass |
| entities.retire operator deny | `entities.retire` | deny | pass |

## WHX.entities.update — Per-tool негативы: WHX.entities.update (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entities.update ∅args→err | `entities.update` | tool_err | pass |
| entities.update null:id | `entities.update` | tool_err | pass |
| entities.update unknown arg | `entities.update` | tool_err | pass |
| entities.update badtype:id | `entities.update` | fail | pass |
| entities.update path:id:slash | `entities.update` | tool_err | pass |
| entities.update path:id:dotdot | `entities.update` | tool_err | pass |
| entities.update path:id:empty | `entities.update` | tool_err | pass |
| entities.update path:id:array | `entities.update` | tool_err | pass |
| entities.update ghost:id | `entities.update` | fail | pass |
| entities.update operator deny | `entities.update` | deny | pass |

## WHX.entity_checks.list — Per-tool негативы: WHX.entity_checks.list (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| entity_checks.list ∅args→err | `entity_checks.list` | tool_err | pass |
| entity_checks.list null:id | `entity_checks.list` | tool_err | pass |
| entity_checks.list unknown arg | `entity_checks.list` | tool_err | pass |
| entity_checks.list badtype:id | `entity_checks.list` | fail | pass |
| entity_checks.list path:id:slash | `entity_checks.list` | tool_err | pass |
| entity_checks.list path:id:dotdot | `entity_checks.list` | tool_err | pass |
| entity_checks.list path:id:empty | `entity_checks.list` | tool_err | pass |
| entity_checks.list path:id:array | `entity_checks.list` | tool_err | pass |
| entity_checks.list ghost:id | `entity_checks.list` | fail | pass |
| entity_checks.list operator reach | `entity_checks.list` | reach | pass |

## WHX.factors.list — Per-tool негативы: WHX.factors.list (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| factors.list ∅args→err | `factors.list` | tool_err | pass |
| factors.list null:id | `factors.list` | tool_err | pass |
| factors.list unknown arg | `factors.list` | tool_err | pass |
| factors.list badtype:id | `factors.list` | fail | pass |
| factors.list path:id:slash | `factors.list` | tool_err | pass |
| factors.list path:id:dotdot | `factors.list` | tool_err | pass |
| factors.list path:id:empty | `factors.list` | tool_err | pass |
| factors.list path:id:array | `factors.list` | tool_err | pass |
| factors.list ghost:id | `factors.list` | fail | pass |
| factors.list operator reach | `factors.list` | reach | pass |

## WHX.factors.resolve — Per-tool негативы: WHX.factors.resolve (17 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| factors.resolve ∅args→err | `factors.resolve` | tool_err | pass |
| factors.resolve null:id | `factors.resolve` | tool_err | pass |
| factors.resolve null:factor_id | `factors.resolve` | tool_err | pass |
| factors.resolve unknown arg | `factors.resolve` | tool_err | pass |
| factors.resolve badtype:id | `factors.resolve` | fail | pass |
| factors.resolve badtype:factor_id | `factors.resolve` | fail | pass |
| factors.resolve path:id:slash | `factors.resolve` | tool_err | pass |
| factors.resolve path:id:dotdot | `factors.resolve` | tool_err | pass |
| factors.resolve path:id:empty | `factors.resolve` | tool_err | pass |
| factors.resolve path:id:array | `factors.resolve` | tool_err | pass |
| factors.resolve ghost:id | `factors.resolve` | fail | pass |
| factors.resolve path:factor_id:slash | `factors.resolve` | tool_err | pass |
| factors.resolve path:factor_id:dotdot | `factors.resolve` | tool_err | pass |
| factors.resolve path:factor_id:empty | `factors.resolve` | tool_err | pass |
| factors.resolve path:factor_id:array | `factors.resolve` | tool_err | pass |
| factors.resolve ghost:factor_id | `factors.resolve` | fail | pass |
| factors.resolve operator deny | `factors.resolve` | deny | pass |

## WHX.files.delete — Per-tool негативы: WHX.files.delete (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.delete ∅args→err | `files.delete` | tool_err | pass |
| files.delete null:id | `files.delete` | tool_err | pass |
| files.delete unknown arg | `files.delete` | tool_err | pass |
| files.delete badtype:id | `files.delete` | fail | pass |
| files.delete path:id:slash | `files.delete` | tool_err | pass |
| files.delete path:id:dotdot | `files.delete` | tool_err | pass |
| files.delete path:id:empty | `files.delete` | tool_err | pass |
| files.delete path:id:array | `files.delete` | tool_err | pass |
| files.delete ghost:id | `files.delete` | fail | pass |
| files.delete operator deny | `files.delete` | deny | pass |

## WHX.files.download — Per-tool негативы: WHX.files.download (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.download ∅args→err | `files.download` | tool_err | pass |
| files.download null:id | `files.download` | tool_err | pass |
| files.download unknown arg | `files.download` | tool_err | pass |
| files.download badtype:id | `files.download` | fail | pass |
| files.download path:id:slash | `files.download` | tool_err | pass |
| files.download path:id:dotdot | `files.download` | tool_err | pass |
| files.download path:id:empty | `files.download` | tool_err | pass |
| files.download path:id:array | `files.download` | tool_err | pass |
| files.download ghost:id | `files.download` | fail | pass |
| files.download operator deny | `files.download` | deny | pass |

## WHX.files.get — Per-tool негативы: WHX.files.get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.get ∅args→err | `files.get` | tool_err | pass |
| files.get null:id | `files.get` | tool_err | pass |
| files.get unknown arg | `files.get` | tool_err | pass |
| files.get badtype:id | `files.get` | fail | pass |
| files.get path:id:slash | `files.get` | tool_err | pass |
| files.get path:id:dotdot | `files.get` | tool_err | pass |
| files.get path:id:empty | `files.get` | tool_err | pass |
| files.get path:id:array | `files.get` | tool_err | pass |
| files.get ghost:id | `files.get` | fail | pass |
| files.get operator deny | `files.get` | deny | pass |

## WHX.files.upload_abort — Per-tool негативы: WHX.files.upload_abort (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.upload_abort ∅args→err | `files.upload_abort` | tool_err | pass |
| files.upload_abort null:id | `files.upload_abort` | tool_err | pass |
| files.upload_abort unknown arg | `files.upload_abort` | tool_err | pass |
| files.upload_abort badtype:id | `files.upload_abort` | fail | pass |
| files.upload_abort path:id:slash | `files.upload_abort` | tool_err | pass |
| files.upload_abort path:id:dotdot | `files.upload_abort` | tool_err | pass |
| files.upload_abort path:id:empty | `files.upload_abort` | tool_err | pass |
| files.upload_abort path:id:array | `files.upload_abort` | tool_err | pass |
| files.upload_abort ghost:id | `files.upload_abort` | fail | pass |
| files.upload_abort operator deny | `files.upload_abort` | deny | pass |

## WHX.files.upload_chunk — Per-tool негативы: WHX.files.upload_chunk (18 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.upload_chunk ∅args→err | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk null:id | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk null:index | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk null:data_base64 | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk unknown arg | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk badtype:id | `files.upload_chunk` | fail | pass |
| files.upload_chunk badtype:index | `files.upload_chunk` | fail | pass |
| files.upload_chunk path:id:slash | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk path:id:dotdot | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk path:id:empty | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk path:id:array | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk ghost:id | `files.upload_chunk` | fail | pass |
| files.upload_chunk path:index:slash | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk path:index:dotdot | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk path:index:empty | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk path:index:array | `files.upload_chunk` | tool_err | pass |
| files.upload_chunk ghost:index | `files.upload_chunk` | fail | pass |
| files.upload_chunk operator deny | `files.upload_chunk` | deny | pass |

## WHX.files.upload_complete — Per-tool негативы: WHX.files.upload_complete (14 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.upload_complete ∅args→err | `files.upload_complete` | tool_err | pass |
| files.upload_complete null:id | `files.upload_complete` | tool_err | pass |
| files.upload_complete null:final_hash | `files.upload_complete` | tool_err | pass |
| files.upload_complete null:total_size | `files.upload_complete` | tool_err | pass |
| files.upload_complete unknown arg | `files.upload_complete` | tool_err | pass |
| files.upload_complete badtype:id | `files.upload_complete` | fail | pass |
| files.upload_complete badtype:final_hash | `files.upload_complete` | fail | pass |
| files.upload_complete emptystr:final_hash | `files.upload_complete` | fail | pass |
| files.upload_complete path:id:slash | `files.upload_complete` | tool_err | pass |
| files.upload_complete path:id:dotdot | `files.upload_complete` | tool_err | pass |
| files.upload_complete path:id:empty | `files.upload_complete` | tool_err | pass |
| files.upload_complete path:id:array | `files.upload_complete` | tool_err | pass |
| files.upload_complete ghost:id | `files.upload_complete` | fail | pass |
| files.upload_complete operator deny | `files.upload_complete` | deny | pass |

## WHX.files.upload_init — Per-tool негативы: WHX.files.upload_init (9 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.upload_init ∅args→err | `files.upload_init` | tool_err | pass |
| files.upload_init null:filename | `files.upload_init` | tool_err | pass |
| files.upload_init null:expected_size | `files.upload_init` | tool_err | pass |
| files.upload_init null:expected_hash | `files.upload_init` | tool_err | pass |
| files.upload_init unknown arg | `files.upload_init` | tool_err | pass |
| files.upload_init badtype:filename | `files.upload_init` | fail | pass |
| files.upload_init badtype:expected_size | `files.upload_init` | fail | pass |
| files.upload_init emptystr:filename | `files.upload_init` | fail | pass |
| files.upload_init operator deny | `files.upload_init` | deny | pass |

## WHX.files.upload_status — Per-tool негативы: WHX.files.upload_status (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| files.upload_status ∅args→err | `files.upload_status` | tool_err | pass |
| files.upload_status null:id | `files.upload_status` | tool_err | pass |
| files.upload_status unknown arg | `files.upload_status` | tool_err | pass |
| files.upload_status badtype:id | `files.upload_status` | fail | pass |
| files.upload_status path:id:slash | `files.upload_status` | tool_err | pass |
| files.upload_status path:id:dotdot | `files.upload_status` | tool_err | pass |
| files.upload_status path:id:empty | `files.upload_status` | tool_err | pass |
| files.upload_status path:id:array | `files.upload_status` | tool_err | pass |
| files.upload_status ghost:id | `files.upload_status` | fail | pass |
| files.upload_status operator deny | `files.upload_status` | deny | pass |

## WHX.monitors.list — Per-tool негативы: WHX.monitors.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| monitors.list ∅args | `monitors.list` | ok | pass |
| monitors.list unknown arg | `monitors.list` | tool_err | pass |
| monitors.list operator deny | `monitors.list` | deny | pass |

## WHX.monitors.run — Per-tool негативы: WHX.monitors.run (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| monitors.run ∅args→err | `monitors.run` | tool_err | pass |
| monitors.run null:id | `monitors.run` | tool_err | pass |
| monitors.run unknown arg | `monitors.run` | tool_err | pass |
| monitors.run badtype:id | `monitors.run` | fail | pass |
| monitors.run path:id:slash | `monitors.run` | tool_err | pass |
| monitors.run path:id:dotdot | `monitors.run` | tool_err | pass |
| monitors.run path:id:empty | `monitors.run` | tool_err | pass |
| monitors.run path:id:array | `monitors.run` | tool_err | pass |
| monitors.run ghost:id | `monitors.run` | fail | pass |
| monitors.run operator deny | `monitors.run` | deny | pass |

## WHX.permissions.group_create — Per-tool негативы: WHX.permissions.group_create (9 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| permissions.group_create ∅args→err | `permissions.group_create` | tool_err | pass |
| permissions.group_create null:name | `permissions.group_create` | tool_err | pass |
| permissions.group_create null:class | `permissions.group_create` | tool_err | pass |
| permissions.group_create unknown arg | `permissions.group_create` | tool_err | pass |
| permissions.group_create badtype:name | `permissions.group_create` | fail | pass |
| permissions.group_create badtype:class | `permissions.group_create` | fail | pass |
| permissions.group_create emptystr:name | `permissions.group_create` | fail | pass |
| permissions.group_create emptystr:class | `permissions.group_create` | fail | pass |
| permissions.group_create operator deny | `permissions.group_create` | deny | pass |

## WHX.permissions.group_delete — Per-tool негативы: WHX.permissions.group_delete (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| permissions.group_delete ∅args→err | `permissions.group_delete` | tool_err | pass |
| permissions.group_delete null:id | `permissions.group_delete` | tool_err | pass |
| permissions.group_delete unknown arg | `permissions.group_delete` | tool_err | pass |
| permissions.group_delete badtype:id | `permissions.group_delete` | fail | pass |
| permissions.group_delete path:id:slash | `permissions.group_delete` | tool_err | pass |
| permissions.group_delete path:id:dotdot | `permissions.group_delete` | tool_err | pass |
| permissions.group_delete path:id:empty | `permissions.group_delete` | tool_err | pass |
| permissions.group_delete path:id:array | `permissions.group_delete` | tool_err | pass |
| permissions.group_delete ghost:id | `permissions.group_delete` | fail | pass |
| permissions.group_delete operator deny | `permissions.group_delete` | deny | pass |

## WHX.permissions.group_update — Per-tool негативы: WHX.permissions.group_update (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| permissions.group_update ∅args→err | `permissions.group_update` | tool_err | pass |
| permissions.group_update null:id | `permissions.group_update` | tool_err | pass |
| permissions.group_update null:name | `permissions.group_update` | tool_err | pass |
| permissions.group_update unknown arg | `permissions.group_update` | tool_err | pass |
| permissions.group_update badtype:id | `permissions.group_update` | fail | pass |
| permissions.group_update badtype:name | `permissions.group_update` | fail | pass |
| permissions.group_update emptystr:name | `permissions.group_update` | fail | pass |
| permissions.group_update path:id:slash | `permissions.group_update` | tool_err | pass |
| permissions.group_update path:id:dotdot | `permissions.group_update` | tool_err | pass |
| permissions.group_update path:id:empty | `permissions.group_update` | tool_err | pass |
| permissions.group_update path:id:array | `permissions.group_update` | tool_err | pass |
| permissions.group_update ghost:id | `permissions.group_update` | fail | pass |
| permissions.group_update operator deny | `permissions.group_update` | deny | pass |

## WHX.permissions.groups — Per-tool негативы: WHX.permissions.groups (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| permissions.groups ∅args | `permissions.groups` | ok | pass |
| permissions.groups unknown arg | `permissions.groups` | tool_err | pass |
| permissions.groups operator deny | `permissions.groups` | deny | pass |

## WHX.relations.create — Per-tool негативы: WHX.relations.create (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| relations.create ∅args→err | `relations.create` | tool_err | pass |
| relations.create null:id | `relations.create` | tool_err | pass |
| relations.create null:predicate | `relations.create` | tool_err | pass |
| relations.create unknown arg | `relations.create` | tool_err | pass |
| relations.create badtype:id | `relations.create` | fail | pass |
| relations.create badtype:predicate | `relations.create` | fail | pass |
| relations.create emptystr:predicate | `relations.create` | fail | pass |
| relations.create path:id:slash | `relations.create` | tool_err | pass |
| relations.create path:id:dotdot | `relations.create` | tool_err | pass |
| relations.create path:id:empty | `relations.create` | tool_err | pass |
| relations.create path:id:array | `relations.create` | tool_err | pass |
| relations.create ghost:id | `relations.create` | fail | pass |
| relations.create operator deny | `relations.create` | deny | pass |

## WHX.relations.delete — Per-tool негативы: WHX.relations.delete (17 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| relations.delete ∅args→err | `relations.delete` | tool_err | pass |
| relations.delete null:id | `relations.delete` | tool_err | pass |
| relations.delete null:relation_id | `relations.delete` | tool_err | pass |
| relations.delete unknown arg | `relations.delete` | tool_err | pass |
| relations.delete badtype:id | `relations.delete` | fail | pass |
| relations.delete badtype:relation_id | `relations.delete` | fail | pass |
| relations.delete path:id:slash | `relations.delete` | tool_err | pass |
| relations.delete path:id:dotdot | `relations.delete` | tool_err | pass |
| relations.delete path:id:empty | `relations.delete` | tool_err | pass |
| relations.delete path:id:array | `relations.delete` | tool_err | pass |
| relations.delete ghost:id | `relations.delete` | fail | pass |
| relations.delete path:relation_id:slash | `relations.delete` | tool_err | pass |
| relations.delete path:relation_id:dotdot | `relations.delete` | tool_err | pass |
| relations.delete path:relation_id:empty | `relations.delete` | tool_err | pass |
| relations.delete path:relation_id:array | `relations.delete` | tool_err | pass |
| relations.delete ghost:relation_id | `relations.delete` | fail | pass |
| relations.delete operator deny | `relations.delete` | deny | pass |

## WHX.relations.list — Per-tool негативы: WHX.relations.list (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| relations.list ∅args→err | `relations.list` | tool_err | pass |
| relations.list null:id | `relations.list` | tool_err | pass |
| relations.list unknown arg | `relations.list` | tool_err | pass |
| relations.list badtype:id | `relations.list` | fail | pass |
| relations.list path:id:slash | `relations.list` | tool_err | pass |
| relations.list path:id:dotdot | `relations.list` | tool_err | pass |
| relations.list path:id:empty | `relations.list` | tool_err | pass |
| relations.list path:id:array | `relations.list` | tool_err | pass |
| relations.list ghost:id | `relations.list` | fail | pass |
| relations.list operator reach | `relations.list` | reach | pass |

## WHX.reports.generate — Per-tool негативы: WHX.reports.generate (6 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| reports.generate ∅args→err | `reports.generate` | tool_err | pass |
| reports.generate null:entity_id | `reports.generate` | tool_err | pass |
| reports.generate unknown arg | `reports.generate` | tool_err | pass |
| reports.generate badtype:entity_id | `reports.generate` | fail | pass |
| reports.generate emptystr:entity_id | `reports.generate` | fail | pass |
| reports.generate operator deny | `reports.generate` | deny | pass |

## WHX.reports.list — Per-tool негативы: WHX.reports.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| reports.list ∅args | `reports.list` | ok | pass |
| reports.list unknown arg | `reports.list` | tool_err | pass |
| reports.list operator ok | `reports.list` | ok | pass |

## WHX.requests.create — Per-tool негативы: WHX.requests.create (9 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| requests.create ∅args→err | `requests.create` | tool_err | pass |
| requests.create null:target_entity_id | `requests.create` | tool_err | pass |
| requests.create null:requested_product | `requests.create` | tool_err | pass |
| requests.create unknown arg | `requests.create` | tool_err | pass |
| requests.create badtype:target_entity_id | `requests.create` | fail | pass |
| requests.create badtype:requested_product | `requests.create` | fail | pass |
| requests.create emptystr:target_entity_id | `requests.create` | fail | pass |
| requests.create emptystr:requested_product | `requests.create` | fail | pass |
| requests.create operator deny | `requests.create` | deny | pass |

## WHX.requests.list — Per-tool негативы: WHX.requests.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| requests.list ∅args | `requests.list` | ok | pass |
| requests.list unknown arg | `requests.list` | tool_err | pass |
| requests.list operator ok | `requests.list` | ok | pass |

## WHX.schema.draft_create — Per-tool негативы: WHX.schema.draft_create (6 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.draft_create ∅args→err | `schema.draft_create` | tool_err | pass |
| schema.draft_create null:name | `schema.draft_create` | tool_err | pass |
| schema.draft_create unknown arg | `schema.draft_create` | tool_err | pass |
| schema.draft_create badtype:name | `schema.draft_create` | fail | pass |
| schema.draft_create emptystr:name | `schema.draft_create` | fail | pass |
| schema.draft_create operator deny | `schema.draft_create` | deny | pass |

## WHX.schema.draft_delete — Per-tool негативы: WHX.schema.draft_delete (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.draft_delete ∅args→err | `schema.draft_delete` | tool_err | pass |
| schema.draft_delete null:id | `schema.draft_delete` | tool_err | pass |
| schema.draft_delete unknown arg | `schema.draft_delete` | tool_err | pass |
| schema.draft_delete badtype:id | `schema.draft_delete` | fail | pass |
| schema.draft_delete path:id:slash | `schema.draft_delete` | tool_err | pass |
| schema.draft_delete path:id:dotdot | `schema.draft_delete` | tool_err | pass |
| schema.draft_delete path:id:empty | `schema.draft_delete` | tool_err | pass |
| schema.draft_delete path:id:array | `schema.draft_delete` | tool_err | pass |
| schema.draft_delete ghost:id | `schema.draft_delete` | fail | pass |
| schema.draft_delete operator deny | `schema.draft_delete` | deny | pass |

## WHX.schema.draft_get — Per-tool негативы: WHX.schema.draft_get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.draft_get ∅args→err | `schema.draft_get` | tool_err | pass |
| schema.draft_get null:id | `schema.draft_get` | tool_err | pass |
| schema.draft_get unknown arg | `schema.draft_get` | tool_err | pass |
| schema.draft_get badtype:id | `schema.draft_get` | fail | pass |
| schema.draft_get path:id:slash | `schema.draft_get` | tool_err | pass |
| schema.draft_get path:id:dotdot | `schema.draft_get` | tool_err | pass |
| schema.draft_get path:id:empty | `schema.draft_get` | tool_err | pass |
| schema.draft_get path:id:array | `schema.draft_get` | tool_err | pass |
| schema.draft_get ghost:id | `schema.draft_get` | fail | pass |
| schema.draft_get operator deny | `schema.draft_get` | deny | pass |

## WHX.schema.draft_impact — Per-tool негативы: WHX.schema.draft_impact (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.draft_impact ∅args→err | `schema.draft_impact` | tool_err | pass |
| schema.draft_impact null:id | `schema.draft_impact` | tool_err | pass |
| schema.draft_impact unknown arg | `schema.draft_impact` | tool_err | pass |
| schema.draft_impact badtype:id | `schema.draft_impact` | fail | pass |
| schema.draft_impact path:id:slash | `schema.draft_impact` | tool_err | pass |
| schema.draft_impact path:id:dotdot | `schema.draft_impact` | tool_err | pass |
| schema.draft_impact path:id:empty | `schema.draft_impact` | tool_err | pass |
| schema.draft_impact path:id:array | `schema.draft_impact` | tool_err | pass |
| schema.draft_impact ghost:id | `schema.draft_impact` | fail | pass |
| schema.draft_impact operator deny | `schema.draft_impact` | deny | pass |

## WHX.schema.draft_publish — Per-tool негативы: WHX.schema.draft_publish (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.draft_publish ∅args→err | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish null:id | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish null:reason | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish unknown arg | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish badtype:id | `schema.draft_publish` | fail | pass |
| schema.draft_publish badtype:reason | `schema.draft_publish` | fail | pass |
| schema.draft_publish emptystr:reason | `schema.draft_publish` | fail | pass |
| schema.draft_publish path:id:slash | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish path:id:dotdot | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish path:id:empty | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish path:id:array | `schema.draft_publish` | tool_err | pass |
| schema.draft_publish ghost:id | `schema.draft_publish` | fail | pass |
| schema.draft_publish operator deny | `schema.draft_publish` | deny | pass |

## WHX.schema.draft_update — Per-tool негативы: WHX.schema.draft_update (14 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.draft_update ∅args→err | `schema.draft_update` | tool_err | pass |
| schema.draft_update null:id | `schema.draft_update` | tool_err | pass |
| schema.draft_update null:display_name | `schema.draft_update` | tool_err | pass |
| schema.draft_update null:semantic_version | `schema.draft_update` | tool_err | pass |
| schema.draft_update unknown arg | `schema.draft_update` | tool_err | pass |
| schema.draft_update badtype:id | `schema.draft_update` | fail | pass |
| schema.draft_update badtype:display_name | `schema.draft_update` | fail | pass |
| schema.draft_update emptystr:display_name | `schema.draft_update` | fail | pass |
| schema.draft_update path:id:slash | `schema.draft_update` | tool_err | pass |
| schema.draft_update path:id:dotdot | `schema.draft_update` | tool_err | pass |
| schema.draft_update path:id:empty | `schema.draft_update` | tool_err | pass |
| schema.draft_update path:id:array | `schema.draft_update` | tool_err | pass |
| schema.draft_update ghost:id | `schema.draft_update` | fail | pass |
| schema.draft_update operator deny | `schema.draft_update` | deny | pass |

## WHX.schema.draft_validate — Per-tool негативы: WHX.schema.draft_validate (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.draft_validate ∅args→err | `schema.draft_validate` | tool_err | pass |
| schema.draft_validate null:id | `schema.draft_validate` | tool_err | pass |
| schema.draft_validate unknown arg | `schema.draft_validate` | tool_err | pass |
| schema.draft_validate badtype:id | `schema.draft_validate` | fail | pass |
| schema.draft_validate path:id:slash | `schema.draft_validate` | tool_err | pass |
| schema.draft_validate path:id:dotdot | `schema.draft_validate` | tool_err | pass |
| schema.draft_validate path:id:empty | `schema.draft_validate` | tool_err | pass |
| schema.draft_validate path:id:array | `schema.draft_validate` | tool_err | pass |
| schema.draft_validate ghost:id | `schema.draft_validate` | fail | pass |
| schema.draft_validate operator deny | `schema.draft_validate` | deny | pass |

## WHX.schema.drafts — Per-tool негативы: WHX.schema.drafts (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.drafts ∅args | `schema.drafts` | ok | pass |
| schema.drafts unknown arg | `schema.drafts` | tool_err | pass |
| schema.drafts operator deny | `schema.drafts` | deny | pass |

## WHX.schema.predicates — Per-tool негативы: WHX.schema.predicates (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.predicates ∅args | `schema.predicates` | ok | pass |
| schema.predicates unknown arg | `schema.predicates` | tool_err | pass |
| schema.predicates operator ok | `schema.predicates` | ok | pass |

## WHX.schema.published — Per-tool негативы: WHX.schema.published (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.published ∅args | `schema.published` | ok | pass |
| schema.published unknown arg | `schema.published` | tool_err | pass |
| schema.published operator ok | `schema.published` | ok | pass |

## WHX.schema.revision_get — Per-tool негативы: WHX.schema.revision_get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.revision_get ∅args→err | `schema.revision_get` | tool_err | pass |
| schema.revision_get null:id | `schema.revision_get` | tool_err | pass |
| schema.revision_get unknown arg | `schema.revision_get` | tool_err | pass |
| schema.revision_get badtype:id | `schema.revision_get` | fail | pass |
| schema.revision_get path:id:slash | `schema.revision_get` | tool_err | pass |
| schema.revision_get path:id:dotdot | `schema.revision_get` | tool_err | pass |
| schema.revision_get path:id:empty | `schema.revision_get` | tool_err | pass |
| schema.revision_get path:id:array | `schema.revision_get` | tool_err | pass |
| schema.revision_get ghost:id | `schema.revision_get` | fail | pass |
| schema.revision_get operator reach | `schema.revision_get` | reach | pass |

## WHX.schema.revision_migration — Per-tool негативы: WHX.schema.revision_migration (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.revision_migration ∅args→err | `schema.revision_migration` | tool_err | pass |
| schema.revision_migration null:id | `schema.revision_migration` | tool_err | pass |
| schema.revision_migration unknown arg | `schema.revision_migration` | tool_err | pass |
| schema.revision_migration badtype:id | `schema.revision_migration` | fail | pass |
| schema.revision_migration path:id:slash | `schema.revision_migration` | tool_err | pass |
| schema.revision_migration path:id:dotdot | `schema.revision_migration` | tool_err | pass |
| schema.revision_migration path:id:empty | `schema.revision_migration` | tool_err | pass |
| schema.revision_migration path:id:array | `schema.revision_migration` | tool_err | pass |
| schema.revision_migration ghost:id | `schema.revision_migration` | fail | pass |
| schema.revision_migration operator reach | `schema.revision_migration` | reach | pass |

## WHX.schema.revision_retire — Per-tool негативы: WHX.schema.revision_retire (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.revision_retire ∅args→err | `schema.revision_retire` | tool_err | pass |
| schema.revision_retire null:id | `schema.revision_retire` | tool_err | pass |
| schema.revision_retire unknown arg | `schema.revision_retire` | tool_err | pass |
| schema.revision_retire badtype:id | `schema.revision_retire` | fail | pass |
| schema.revision_retire path:id:slash | `schema.revision_retire` | tool_err | pass |
| schema.revision_retire path:id:dotdot | `schema.revision_retire` | tool_err | pass |
| schema.revision_retire path:id:empty | `schema.revision_retire` | tool_err | pass |
| schema.revision_retire path:id:array | `schema.revision_retire` | tool_err | pass |
| schema.revision_retire ghost:id | `schema.revision_retire` | fail | pass |
| schema.revision_retire operator deny | `schema.revision_retire` | deny | pass |

## WHX.schema.type_get — Per-tool негативы: WHX.schema.type_get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.type_get ∅args→err | `schema.type_get` | tool_err | pass |
| schema.type_get null:alias | `schema.type_get` | tool_err | pass |
| schema.type_get unknown arg | `schema.type_get` | tool_err | pass |
| schema.type_get badtype:alias | `schema.type_get` | fail | pass |
| schema.type_get path:alias:slash | `schema.type_get` | tool_err | pass |
| schema.type_get path:alias:dotdot | `schema.type_get` | tool_err | pass |
| schema.type_get path:alias:empty | `schema.type_get` | tool_err | pass |
| schema.type_get path:alias:array | `schema.type_get` | tool_err | pass |
| schema.type_get ghost:alias | `schema.type_get` | fail | pass |
| schema.type_get operator reach | `schema.type_get` | reach | pass |

## WHX.schema.type_view — Per-tool негативы: WHX.schema.type_view (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.type_view ∅args→err | `schema.type_view` | tool_err | pass |
| schema.type_view null:alias | `schema.type_view` | tool_err | pass |
| schema.type_view null:view_type | `schema.type_view` | tool_err | pass |
| schema.type_view unknown arg | `schema.type_view` | tool_err | pass |
| schema.type_view badtype:alias | `schema.type_view` | fail | pass |
| schema.type_view badtype:view_type | `schema.type_view` | fail | pass |
| schema.type_view emptystr:view_type | `schema.type_view` | fail | pass |
| schema.type_view path:alias:slash | `schema.type_view` | tool_err | pass |
| schema.type_view path:alias:dotdot | `schema.type_view` | tool_err | pass |
| schema.type_view path:alias:empty | `schema.type_view` | tool_err | pass |
| schema.type_view path:alias:array | `schema.type_view` | tool_err | pass |
| schema.type_view ghost:alias | `schema.type_view` | fail | pass |
| schema.type_view operator reach | `schema.type_view` | reach | pass |

## WHX.schema.types — Per-tool негативы: WHX.schema.types (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| schema.types ∅args | `schema.types` | ok | pass |
| schema.types unknown arg | `schema.types` | tool_err | pass |
| schema.types operator ok | `schema.types` | ok | pass |

## WHX.tasks.cancel — Per-tool негативы: WHX.tasks.cancel (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| tasks.cancel ∅args→err | `tasks.cancel` | tool_err | pass |
| tasks.cancel null:id | `tasks.cancel` | tool_err | pass |
| tasks.cancel unknown arg | `tasks.cancel` | tool_err | pass |
| tasks.cancel badtype:id | `tasks.cancel` | fail | pass |
| tasks.cancel path:id:slash | `tasks.cancel` | tool_err | pass |
| tasks.cancel path:id:dotdot | `tasks.cancel` | tool_err | pass |
| tasks.cancel path:id:empty | `tasks.cancel` | tool_err | pass |
| tasks.cancel path:id:array | `tasks.cancel` | tool_err | pass |
| tasks.cancel ghost:id | `tasks.cancel` | fail | pass |
| tasks.cancel operator deny | `tasks.cancel` | deny | pass |

## WHX.tasks.complete — Per-tool негативы: WHX.tasks.complete (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| tasks.complete ∅args→err | `tasks.complete` | tool_err | pass |
| tasks.complete null:id | `tasks.complete` | tool_err | pass |
| tasks.complete unknown arg | `tasks.complete` | tool_err | pass |
| tasks.complete badtype:id | `tasks.complete` | fail | pass |
| tasks.complete path:id:slash | `tasks.complete` | tool_err | pass |
| tasks.complete path:id:dotdot | `tasks.complete` | tool_err | pass |
| tasks.complete path:id:empty | `tasks.complete` | tool_err | pass |
| tasks.complete path:id:array | `tasks.complete` | tool_err | pass |
| tasks.complete ghost:id | `tasks.complete` | fail | pass |
| tasks.complete operator deny | `tasks.complete` | deny | pass |

## WHX.tasks.create — Per-tool негативы: WHX.tasks.create (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| tasks.create ∅args→err | `tasks.create` | tool_err | pass |
| tasks.create null:id | `tasks.create` | tool_err | pass |
| tasks.create null:kind | `tasks.create` | tool_err | pass |
| tasks.create unknown arg | `tasks.create` | tool_err | pass |
| tasks.create badtype:id | `tasks.create` | fail | pass |
| tasks.create badtype:kind | `tasks.create` | fail | pass |
| tasks.create emptystr:kind | `tasks.create` | fail | pass |
| tasks.create path:id:slash | `tasks.create` | tool_err | pass |
| tasks.create path:id:dotdot | `tasks.create` | tool_err | pass |
| tasks.create path:id:empty | `tasks.create` | tool_err | pass |
| tasks.create path:id:array | `tasks.create` | tool_err | pass |
| tasks.create ghost:id | `tasks.create` | fail | pass |
| tasks.create operator deny | `tasks.create` | deny | pass |

## WHX.tasks.list — Per-tool негативы: WHX.tasks.list (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| tasks.list ∅args→err | `tasks.list` | tool_err | pass |
| tasks.list null:id | `tasks.list` | tool_err | pass |
| tasks.list unknown arg | `tasks.list` | tool_err | pass |
| tasks.list badtype:id | `tasks.list` | fail | pass |
| tasks.list path:id:slash | `tasks.list` | tool_err | pass |
| tasks.list path:id:dotdot | `tasks.list` | tool_err | pass |
| tasks.list path:id:empty | `tasks.list` | tool_err | pass |
| tasks.list path:id:array | `tasks.list` | tool_err | pass |
| tasks.list ghost:id | `tasks.list` | fail | pass |
| tasks.list operator reach | `tasks.list` | reach | pass |

## WHX.tasks.start — Per-tool негативы: WHX.tasks.start (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| tasks.start ∅args→err | `tasks.start` | tool_err | pass |
| tasks.start null:id | `tasks.start` | tool_err | pass |
| tasks.start unknown arg | `tasks.start` | tool_err | pass |
| tasks.start badtype:id | `tasks.start` | fail | pass |
| tasks.start path:id:slash | `tasks.start` | tool_err | pass |
| tasks.start path:id:dotdot | `tasks.start` | tool_err | pass |
| tasks.start path:id:empty | `tasks.start` | tool_err | pass |
| tasks.start path:id:array | `tasks.start` | tool_err | pass |
| tasks.start ghost:id | `tasks.start` | fail | pass |
| tasks.start operator deny | `tasks.start` | deny | pass |

## WHX.users.activity — Per-tool негативы: WHX.users.activity (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.activity ∅args→err | `users.activity` | tool_err | pass |
| users.activity null:id | `users.activity` | tool_err | pass |
| users.activity unknown arg | `users.activity` | tool_err | pass |
| users.activity badtype:id | `users.activity` | fail | pass |
| users.activity path:id:slash | `users.activity` | tool_err | pass |
| users.activity path:id:dotdot | `users.activity` | tool_err | pass |
| users.activity path:id:empty | `users.activity` | tool_err | pass |
| users.activity path:id:array | `users.activity` | tool_err | pass |
| users.activity ghost:id | `users.activity` | fail | pass |
| users.activity operator deny | `users.activity` | deny | pass |

## WHX.users.create — Per-tool негативы: WHX.users.create (9 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.create ∅args→err | `users.create` | tool_err | pass |
| users.create null:login | `users.create` | tool_err | pass |
| users.create null:display_name | `users.create` | tool_err | pass |
| users.create unknown arg | `users.create` | tool_err | pass |
| users.create badtype:login | `users.create` | fail | pass |
| users.create badtype:display_name | `users.create` | fail | pass |
| users.create emptystr:login | `users.create` | fail | pass |
| users.create emptystr:display_name | `users.create` | fail | pass |
| users.create operator deny | `users.create` | deny | pass |

## WHX.users.delete — Per-tool негативы: WHX.users.delete (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.delete ∅args→err | `users.delete` | tool_err | pass |
| users.delete null:id | `users.delete` | tool_err | pass |
| users.delete unknown arg | `users.delete` | tool_err | pass |
| users.delete badtype:id | `users.delete` | fail | pass |
| users.delete path:id:slash | `users.delete` | tool_err | pass |
| users.delete path:id:dotdot | `users.delete` | tool_err | pass |
| users.delete path:id:empty | `users.delete` | tool_err | pass |
| users.delete path:id:array | `users.delete` | tool_err | pass |
| users.delete ghost:id | `users.delete` | fail | pass |
| users.delete operator deny | `users.delete` | deny | pass |

## WHX.users.get — Per-tool негативы: WHX.users.get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.get ∅args→err | `users.get` | tool_err | pass |
| users.get null:id | `users.get` | tool_err | pass |
| users.get unknown arg | `users.get` | tool_err | pass |
| users.get badtype:id | `users.get` | fail | pass |
| users.get path:id:slash | `users.get` | tool_err | pass |
| users.get path:id:dotdot | `users.get` | tool_err | pass |
| users.get path:id:empty | `users.get` | tool_err | pass |
| users.get path:id:array | `users.get` | tool_err | pass |
| users.get ghost:id | `users.get` | fail | pass |
| users.get operator deny | `users.get` | deny | pass |

## WHX.users.list — Per-tool негативы: WHX.users.list (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.list ∅args | `users.list` | ok | pass |
| users.list unknown arg | `users.list` | tool_err | pass |
| users.list operator deny | `users.list` | deny | pass |

## WHX.users.role_groups — Per-tool негативы: WHX.users.role_groups (3 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.role_groups ∅args | `users.role_groups` | ok | pass |
| users.role_groups unknown arg | `users.role_groups` | tool_err | pass |
| users.role_groups operator deny | `users.role_groups` | deny | pass |

## WHX.users.token_create — Per-tool негативы: WHX.users.token_create (13 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.token_create ∅args→err | `users.token_create` | tool_err | pass |
| users.token_create null:id | `users.token_create` | tool_err | pass |
| users.token_create null:name | `users.token_create` | tool_err | pass |
| users.token_create unknown arg | `users.token_create` | tool_err | pass |
| users.token_create badtype:id | `users.token_create` | fail | pass |
| users.token_create badtype:name | `users.token_create` | fail | pass |
| users.token_create emptystr:name | `users.token_create` | fail | pass |
| users.token_create path:id:slash | `users.token_create` | tool_err | pass |
| users.token_create path:id:dotdot | `users.token_create` | tool_err | pass |
| users.token_create path:id:empty | `users.token_create` | tool_err | pass |
| users.token_create path:id:array | `users.token_create` | tool_err | pass |
| users.token_create ghost:id | `users.token_create` | fail | pass |
| users.token_create operator deny | `users.token_create` | deny | pass |

## WHX.users.token_revoke — Per-tool негативы: WHX.users.token_revoke (17 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.token_revoke ∅args→err | `users.token_revoke` | tool_err | pass |
| users.token_revoke null:id | `users.token_revoke` | tool_err | pass |
| users.token_revoke null:token_id | `users.token_revoke` | tool_err | pass |
| users.token_revoke unknown arg | `users.token_revoke` | tool_err | pass |
| users.token_revoke badtype:id | `users.token_revoke` | fail | pass |
| users.token_revoke badtype:token_id | `users.token_revoke` | fail | pass |
| users.token_revoke path:id:slash | `users.token_revoke` | tool_err | pass |
| users.token_revoke path:id:dotdot | `users.token_revoke` | tool_err | pass |
| users.token_revoke path:id:empty | `users.token_revoke` | tool_err | pass |
| users.token_revoke path:id:array | `users.token_revoke` | tool_err | pass |
| users.token_revoke ghost:id | `users.token_revoke` | fail | pass |
| users.token_revoke path:token_id:slash | `users.token_revoke` | tool_err | pass |
| users.token_revoke path:token_id:dotdot | `users.token_revoke` | tool_err | pass |
| users.token_revoke path:token_id:empty | `users.token_revoke` | tool_err | pass |
| users.token_revoke path:token_id:array | `users.token_revoke` | tool_err | pass |
| users.token_revoke ghost:token_id | `users.token_revoke` | fail | pass |
| users.token_revoke operator deny | `users.token_revoke` | deny | pass |

## WHX.users.tokens — Per-tool негативы: WHX.users.tokens (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.tokens ∅args→err | `users.tokens` | tool_err | pass |
| users.tokens null:id | `users.tokens` | tool_err | pass |
| users.tokens unknown arg | `users.tokens` | tool_err | pass |
| users.tokens badtype:id | `users.tokens` | fail | pass |
| users.tokens path:id:slash | `users.tokens` | tool_err | pass |
| users.tokens path:id:dotdot | `users.tokens` | tool_err | pass |
| users.tokens path:id:empty | `users.tokens` | tool_err | pass |
| users.tokens path:id:array | `users.tokens` | tool_err | pass |
| users.tokens ghost:id | `users.tokens` | fail | pass |
| users.tokens operator deny | `users.tokens` | deny | pass |

## WHX.users.update — Per-tool негативы: WHX.users.update (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| users.update ∅args→err | `users.update` | tool_err | pass |
| users.update null:id | `users.update` | tool_err | pass |
| users.update unknown arg | `users.update` | tool_err | pass |
| users.update badtype:id | `users.update` | fail | pass |
| users.update path:id:slash | `users.update` | tool_err | pass |
| users.update path:id:dotdot | `users.update` | tool_err | pass |
| users.update path:id:empty | `users.update` | tool_err | pass |
| users.update path:id:array | `users.update` | tool_err | pass |
| users.update ghost:id | `users.update` | fail | pass |
| users.update operator deny | `users.update` | deny | pass |

## WHX.workflows.create — Per-tool негативы: WHX.workflows.create (9 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| workflows.create ∅args→err | `workflows.create` | tool_err | pass |
| workflows.create null:definition_id | `workflows.create` | tool_err | pass |
| workflows.create null:target_entity_id | `workflows.create` | tool_err | pass |
| workflows.create unknown arg | `workflows.create` | tool_err | pass |
| workflows.create badtype:definition_id | `workflows.create` | fail | pass |
| workflows.create badtype:target_entity_id | `workflows.create` | fail | pass |
| workflows.create emptystr:definition_id | `workflows.create` | fail | pass |
| workflows.create emptystr:target_entity_id | `workflows.create` | fail | pass |
| workflows.create operator deny | `workflows.create` | deny | pass |

## WHX.workflows.get — Per-tool негативы: WHX.workflows.get (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| workflows.get ∅args→err | `workflows.get` | tool_err | pass |
| workflows.get null:id | `workflows.get` | tool_err | pass |
| workflows.get unknown arg | `workflows.get` | tool_err | pass |
| workflows.get badtype:id | `workflows.get` | fail | pass |
| workflows.get path:id:slash | `workflows.get` | tool_err | pass |
| workflows.get path:id:dotdot | `workflows.get` | tool_err | pass |
| workflows.get path:id:empty | `workflows.get` | tool_err | pass |
| workflows.get path:id:array | `workflows.get` | tool_err | pass |
| workflows.get ghost:id | `workflows.get` | fail | pass |
| workflows.get operator reach | `workflows.get` | reach | pass |

## WHX.workflows.tasks — Per-tool негативы: WHX.workflows.tasks (10 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| workflows.tasks ∅args→err | `workflows.tasks` | tool_err | pass |
| workflows.tasks null:id | `workflows.tasks` | tool_err | pass |
| workflows.tasks unknown arg | `workflows.tasks` | tool_err | pass |
| workflows.tasks badtype:id | `workflows.tasks` | fail | pass |
| workflows.tasks path:id:slash | `workflows.tasks` | tool_err | pass |
| workflows.tasks path:id:dotdot | `workflows.tasks` | tool_err | pass |
| workflows.tasks path:id:empty | `workflows.tasks` | tool_err | pass |
| workflows.tasks path:id:array | `workflows.tasks` | tool_err | pass |
| workflows.tasks ghost:id | `workflows.tasks` | fail | pass |
| workflows.tasks operator reach | `workflows.tasks` | reach | pass |

## WHX.workflows.transition — Per-tool негативы: WHX.workflows.transition (14 кейсов)

| Кейс | Инструмент | Ожидание | Итог |
|------|-----------|----------|------|
| workflows.transition ∅args→err | `workflows.transition` | tool_err | pass |
| workflows.transition null:id | `workflows.transition` | tool_err | pass |
| workflows.transition null:trigger | `workflows.transition` | tool_err | pass |
| workflows.transition null:to | `workflows.transition` | tool_err | pass |
| workflows.transition unknown arg | `workflows.transition` | tool_err | pass |
| workflows.transition badtype:id | `workflows.transition` | fail | pass |
| workflows.transition badtype:trigger | `workflows.transition` | fail | pass |
| workflows.transition emptystr:trigger | `workflows.transition` | fail | pass |
| workflows.transition path:id:slash | `workflows.transition` | tool_err | pass |
| workflows.transition path:id:dotdot | `workflows.transition` | tool_err | pass |
| workflows.transition path:id:empty | `workflows.transition` | tool_err | pass |
| workflows.transition path:id:array | `workflows.transition` | tool_err | pass |
| workflows.transition ghost:id | `workflows.transition` | fail | pass |
| workflows.transition operator deny | `workflows.transition` | deny | pass |
