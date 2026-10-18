"""Fix kb.onboarding content (backtick mangling) + enrich kb.mcp.gotchas
with the verified entities.query AST."""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kb_provision import rpc, call, show  # noqa: E402

ONBOARDING_PID = "01a1515a-1125-7970-a21c-b0a772c75a5f"
GOTCHAS_PID = "01a1515a-125e-7490-9dfd-b9fd386dfe0c"

ONBOARDING = """This database holds a project knowledge base for the EngineR repo.
All knowledge is stored as entities of type kb_article (edge predicate
kb_related links related articles). Work through MCP only:
POST http://localhost:8020/mcp, JSON-RPC 2.0, Bearer token from
docs/testing/warehouse/.wh_tokens.json (see kb.mcp.contract for the
handshake).

kb_article fields: kb_id (identity, required), kb_title (required),
kb_category (required), kb_summary, kb_content (full text), kb_sort
(integer), kb_tags (comma-separated string).

Suggested queries:
- List everything:  entities.list {"type": "kb_article", "limit": 100}
- Free-text search: entities.list {"type": "kb_article", "search": "docker"}
- Read one:         entities.get {"id": "<public_id from list>"}
- By category:      entities.query {"query": {
                    "from": {"type_id": "kb_article"},
                    "where": {"field": "kb_category", "op": "eq",
                              "value": "mcp"}}}
  Verified where ops: eq, contains, gt (plus lt/ge/le/ne family);
  combine with {"and": [...]}, or, not. Shorthand "filters":
  {"kb_category": "mcp"} and free-text "search": "..." also work.
  Pagination: "page": {"size": n, "after": "<cursor>"}.
  WARNING: unknown keys inside "query" are silently ignored — the query
  degrades to a match-all (e.g. writing "filter" instead of "where"
  returns EVERY row). A non-object "query" value is a 422.
- Neighbourhood:    entities.graph {"id": "<public_id>",
                    "direction": "both", "max_depth": 2} — follows
                    kb_related edges.

kb_id catalog (stable identity keys; fetch one directly via
entities.query where field=kb_id op=eq):
  kb.onboarding (meta)       — this article
  kb.layout (layout)         — repo directory map
  kb.stack.run (ops)         — docker compose services, ports, rebuilds
  kb.creds (security)        — DB/logins/tokens, never-commit list
  kb.db.access (data)        — psql access, schemas, migration order
  kb.db.state (data)         — current wiped-DB state and what was republished
  kb.mcp.contract (mcp)      — endpoint, handshake, request/response shape
  kb.mcp.gotchas (mcp)       — arg names, error channels, response shapes
  kb.schema.lifecycle (schema) — draft->validate->impact->publish, retire
  kb.tests (testing)         — cargo/vitest/e2e/MCP test commands
  kb.frontend (frontend)     — Nuxt 4 structure, i18n, pages
  kb.graph (frontend)        — GraphView force-bubble architecture
  kb.conventions (conventions) — code style rules + documented deviations
  kb.pitfalls (pitfalls)     — known traps (docker exec -i, nul file, ...)
  kb.docs.index (reference)  — index of docs/ normative specs + test dirs
  kb.phases (status)         — implementation phase status / test counts
"""

GOTCHAS_EXTRA = """
entities.query AST (verified against wh_test_runner.py WH4 cases):
  {"query": {"from": {"type_id": "<alias>"},
             "where": {"field": "<predicate_alias>", "op": "eq", "value": ...},
             "filters": {"<alias>": "<value>"},
             "search": "<free text>",
             "page": {"size": n, "after": "<cursor>"}}}
  where-ops seen live: eq, contains, gt; boolean combinators
  {"and": [...]}, or, not. Unknown keys inside "query" are SILENTLY
  IGNORED and the query match-alls (e.g. "filter" instead of "where"
  returns every row) — non-object "query" is the only hard 422.
"""


def main():
    rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "kb-fix", "version": "1"}})
    rpc("notifications/initialized")

    g = call("entities.get", {"id": GOTCHAS_PID})
    cur = ((g.get("data") or {}).get("fields") or {}).get("kb_content", "")
    r1 = call("entities.update",
              {"id": ONBOARDING_PID, "fields": {"kb_content": ONBOARDING}})
    print("onboarding update ok:", not r1.get("_isError"))
    r2 = call("entities.update",
              {"id": GOTCHAS_PID,
               "fields": {"kb_content": cur + GOTCHAS_EXTRA}})
    print("gotchas update ok:", not r2.get("_isError"))


if __name__ == "__main__":
    main()
