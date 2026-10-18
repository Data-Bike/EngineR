# Knowledge-base write report

Run: 2026-10-18 · tool: `docs/testing/warehouse/kb_provision.py` (+ `kb_fix_content.py`)
· all writes via MCP `tools/call` at `http://localhost:8020/mcp` (admin token)

## Published schema

- **Published revision id: `1`** (semantic_version `1.0.0`, display name
  "Project knowledge base") — the first and only published revision; the
  catalog was empty before (`schema.published` → `{"data": []}`).
- Lifecycle executed in order: `schema.draft_create` (no `base_revision`)
  → `schema.draft_update` (if_match `schema:1:1`) → `draft_validate`
  (valid, 0 findings) → `draft_impact` (additive, non-breaking, 9 new
  fields) → `draft_get` → `draft_publish`.
- **Type `kb_article`** (entity_class `data`), fields:
  `kb_id` text — identity + required; `kb_title` text — required;
  `kb_category` text — required; `kb_summary` text; `kb_content` text;
  `kb_sort` integer; `kb_tags` text (comma-separated).
- **Scalar predicates:** `kb_id` (participates_in_identity), `kb_title`,
  `kb_category`, `kb_summary`, `kb_content`, `kb_sort`, `kb_tags`.
- **Edge predicate:** `kb_related` (domain `kb_article` → range
  `kb_article`).

## Articles written — 16 entities (`entities.create`, outcome `created`)

| kb_id | category | public_id |
|---|---|---|
| kb.onboarding | meta | 01a1515a-1125-7970-a21c-b0a772c75a5f |
| kb.layout | layout | 01a1515a-115c-7771-9dbf-989c1bec9884 |
| kb.stack.run | ops | 01a1515a-117d-7830-b6a4-da885f68934e |
| kb.creds | security | 01a1515a-11a9-7461-b17e-fa322e775a43 |
| kb.db.access | data | 01a1515a-11db-7822-95de-d537f7614b87 |
| kb.db.state | data | 01a1515a-11f7-7ee0-be6d-94263749d7fb |
| kb.mcp.contract | mcp | 01a1515a-122e-7941-b2e8-3854a1292133 |
| kb.mcp.gotchas | mcp | 01a1515a-125e-7490-9dfd-b9fd386dfe0c |
| kb.schema.lifecycle | schema | 01a1515a-1279-7ad1-9099-221cd0886c59 |
| kb.tests | testing | 01a1515a-12a1-7ef2-bb99-e94afa5842d3 |
| kb.frontend | frontend | 01a1515a-12d3-7d43-99fb-8499c22462ed |
| kb.graph | frontend | 01a1515a-1304-7791-a95f-60d8c36eb239 |
| kb.conventions | conventions | 01a1515a-1337-74a1-b4e8-e6a482af2cc1 |
| kb.pitfalls | pitfalls | 01a1515a-136c-7640-95c5-baebd1c36a42 |
| kb.docs.index | reference | 01a1515a-1387-73b1-8a15-f92dc1ea50ca |
| kb.phases | status | 01a1515a-13ab-73c3-8d02-27fc0009973e |

Content was later patched via `entities.update`: kb.onboarding and
kb.mcp.gotchas were corrected to document the *verified* query AST.

## Relations — 13 `kb_related` edges (all created)

onboarding→layout, onboarding→mcp.contract, onboarding→tests,
stack.run→creds, stack.run→db.access, mcp.contract→mcp.gotchas,
mcp.contract→schema.lifecycle, schema.lifecycle→db.state,
schema.lifecycle→mcp.gotchas, frontend→graph, pitfalls→conventions,
pitfalls→mcp.gotchas, docs.index→tests.

## Verification (MCP only)

| Check | Result |
|---|---|
| `entities.list {"type":"kb_article","limit":100}` | **16 rows** |
| `entities.list {"type":"kb_article","search":"docker"}` | 7 hits |
| `entities.query` `from.type_id=kb_article` + `where field=kb_category op=eq value=mcp` | **2 rows** (kb.mcp.contract, kb.mcp.gotchas) |
| `entities.query` `where field=kb_id op=eq value=kb.pitfalls` | 1 row |
| `entities.get` kb.mcp.contract / kb.onboarding | ok, all fields present |
| `entities.graph` kb.mcp.contract depth=2 both | 8 nodes, 8 edges |
| `relations.list` kb.mcp.contract both | kb_related outgoing+incoming |

**Discovery during verification:** the correct `entities.query` AST is
`{"from":{"type_id":…},"where":{"field","op","value"}}` (+ `filters`,
`search`, `page`). An unknown key such as `"filter"` is silently ignored
and the query match-alls — verified live (returned all 16 rows) and now
documented inside kb.onboarding and kb.mcp.gotchas. Reference for the
AST shape: `wh_test_runner.py` cases WH4-01…WH4-15.

## Reproduce / extend

- Re-run `python docs/testing/warehouse/kb_provision.py` safely — `kb_id`
  is the identity field, so re-creating returns
  `outcome: deduplicated` with the same public_id (dedup verified by
  design; no duplicate rows will appear).
- New articles: `entities.create {"type_id":"kb_article","fields":{...}}`
  with a new stable `kb_id`; link with `relations.create`
  `{"id":<subject_pid>,"predicate":"kb_related","target_entity_id":<pid>}`.
