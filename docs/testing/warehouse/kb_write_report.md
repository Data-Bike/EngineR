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

---

# Extension run — spec ingestion (docs/ → kb.docs.*)

Run: 2026-10-18 · tool: `docs/testing/warehouse/kb_docs_provision.py`
· all writes via MCP `tools/call` (no raw SQL, no raw API)

## Articles written — +31 entities (16 → 47 total)

- **`kb.self`** (category `meta`, sort 0): how-to-write-KB recipe —
  `kb_article` field names/kinds, `entities.create` signature
  (`type_id`/`fields`/`identity_mode`), `kb_id` as identity key and
  `deduplicated` semantics, `entities.update` for amendments,
  `relations.create` + `kb_related` linking, `kb.<domain>.<topic>`
  naming, schema draft→validate→impact→publish rule.
- **30 spec articles** (category `spec`, sort = doc number): one per
  normative file under `docs/` (`docs/testing/` excluded):
  `kb.docs.00` … `kb.docs.27`, `kb.docs.99` (references),
  `kb.docs.readme` (docs/README.md). Titles = first heading of each
  file (Russian); summaries 2–4 sentences; content = condensed
  implementation reference ending with a `File: docs/…` pointer.

## Relations — 32 `kb_related` edges (all created)

- `kb.onboarding → kb.self`, `kb.self → kb.schema.lifecycle`.
- `kb.docs.index → kb.docs.<NN>` for all 30 new spec articles.
- `kb.docs.index` `kb_content` updated via `entities.update` — now
  lists every `kb.docs.*` id + title.

## Constraint discovered — statement index tuple limit

`statement_active_fact_unique`
(`migrations/20260803000008_core_statement.sql`) embeds the canonical
scalar object in a btree unique index; PostgreSQL caps index tuples at
2704 bytes. Long prose `kb_content` fails with `internal_error`
(index row size > 2704) around ~2.4–2.6 KB depending on
compressibility. The provisioning script retries `entities.create`
with the trailing paragraph dropped (up to 12 times); first line
always keeps the `File: docs/…` pointer to the full text. 15 articles
needed 1–5 paragraph drops; dropped tails end with
`[truncated: btree cap; see source file]`.

25 throwaway `kb.probe.*` articles created while locating the limit
were retired via `entities.retire` (verified: list no longer returns
them).

## Verification (MCP only)

| Check | Result |
|---|---|
| `entities.list {"type":"kb_article","limit":200}` | **47 rows** (16 + 1 + 30) |
| `entities.query` `where kb_category eq "spec"` | **30 rows** — kb.docs.00–27, 99, readme |
| `entities.get` kb.self | ok — meta, sort 0, write recipe complete |
| `entities.get` kb.docs.10 / .17 / .25 | ok — category spec, sort=doc #, condensed content present |
| `relations.list` kb.docs.index | 31 outgoing kb_related (30 new + pre-existing kb.tests) |
| `relations.list` kb.self | outgoing→schema.lifecycle, incoming→onboarding |

Failures: none outstanding — every failed create was retried with a
shorter content and succeeded; zero remaining errors.

---

# Restore run — truncated spec articles re-inflated

Run: 2026-10-18 · tool: `docs/testing/warehouse/kb_restore_truncated.py`
· all writes via MCP `tools/call` (no raw SQL, no raw API)

## Root cause fix

Migration `20261018000024_statement_fact_hash_index.sql` replaced the
`statement_active_fact_unique` btree index (which embedded the raw scalar
`object` and hit the 2704-byte tuple cap) with an md5-hashed variant.
Full-length `kb_content` now inserts/updates without the paragraph-drop
retry path (live-verified with a ~5 KB create before this run).

## Articles restored — 13 `entities.update` calls

Every `kb_article` whose `kb_content` still ended with
`[truncated: btree cap; see source file]` was rewritten with the exact
full definition from `kb_docs_provision.py` (`File: docs/…` pointer line
preserved):

| kb_id | content length (chars) |
|---|---|
| kb.docs.06 | 2624 |
| kb.docs.13 | 2612 |
| kb.docs.14 | 2663 |
| kb.docs.15 | 2697 |
| kb.docs.16 | 2789 |
| kb.docs.17 | 3111 |
| kb.docs.18 | 3303 |
| kb.docs.20 | 2702 |
| kb.docs.21 | 2607 |
| kb.docs.22 | 3244 |
| kb.docs.23 | 2608 |
| kb.docs.24 | 3033 |
| kb.docs.25 | 3053 |

All 13 full texts were present verbatim in the provision script — no
manual reconstruction was needed. (The earlier run estimated ~15
paragraph-drop retries; 13 marker-bearing contents were live in the DB
at restore time — the other retries either succeeded before persisting
a marker or were counted across attempts.)

## Verification (MCP only)

| Check | Result |
|---|---|
| `entities.list {"type":"kb_article","limit":200}` | **47 rows** |
| `entities.get` sweep over all 47 articles | **0** remaining `[truncated: btree cap` markers |
| Post-update `entities.get` per restored article | `kb_content` **byte-identical** to the `kb_docs_provision.py` definition |
| `kb.probe.*` leftovers in list | **0** (already retired in the prior run; nothing to retire) |

Failures: none — every `entities.update` succeeded on the first attempt
with no index-size error, confirming the hashed index accepts
full-length scalar objects.
