---
name: enginer-mcp
description: Connect to the local EngineR MCP server and call its tools (JSON-RPC 2.0 over streamable HTTP). Use when the task requires reading/writing entities, relations, schema, workflows, permissions, audit, or any other domain operation through the MCP bridge instead of the raw HTTP API.
argument-hint: "[what to do through MCP]"
allowed-tools:
  - exec
  - read
  - grep
  - glob
---

# EngineR MCP bridge

The EngineR MCP server runs locally in Docker at `http://localhost:8020/mcp`
(override: `ENGINER_MCP` env var). It is a standards-compliant MCP endpoint
(streamable HTTP transport, JSON-RPC 2.0, protocol version `2025-06-18`) that
exposes the EngineR HTTP API as ~95 tools. **All domain operations must go
through MCP** — do not call `localhost:8010` API endpoints directly for domain
work when this skill is used.

## 1. Authentication

Every request needs `Authorization: Bearer <egr_...>` — an EngineR API token.

- Tokens for this repo's test deployments live in
  `docs/testing/warehouse/.wh_tokens.json` (`{"admin": ..., "operator": ...,
  "analyst": ...}`) — created by `docs/testing/warehouse/bootstrap.py`.
- The token catalog is filtered per principal: admin sees all ~95 tools,
  restricted personas see a subset (capability-filtered).
- Missing/garbage auth → HTTP `401`; a **revoked** token may return HTTP `200`
  with `result.isError` for up to ~30 s because the MCP context cache is warm —
  treat that as expired too.

## 2. Request shape

Streamable HTTP: `POST /mcp`, headers:

```
Content-Type: application/json
Accept: application/json, text/event-stream
Authorization: Bearer <token>
```

Body — JSON-RPC 2.0, one object or a batch array:

```json
{"jsonrpc": "2.0", "id": 1, "method": "tools/call",
 "params": {"name": "<tool>", "arguments": {...}}}
```

- Responses may be plain JSON **or** SSE (`data: {...}` lines) — parse both:
  take the `data:` payload if present.
- Notifications (`"jsonrpc":"2.0"`, no `id`) → HTTP 202, empty body.
- `GET /mcp` → 405 (no server-initiated stream).
- Unknown method → `{"error": {"code": -32601}}` at HTTP 200.

Standard sequence a client must do once per session:

1. `initialize` — params: `{"protocolVersion": "2025-06-18",
   "capabilities": {}, "clientInfo": {"name": "...", "version": "..."}}`
   → returns `serverInfo`, `capabilities`, `instructions`.
2. `notifications/initialized` — notify, no response needed.
3. `tools/list` → `{result: {tools: [{name, description, inputSchema, ...}]}}`
4. `tools/call` for actual work.

## 3. tools/call semantics

Response:

```json
{"result": {"content": [{"type": "text", "text": "<JSON payload>"}],
            "isError": false}}
```

- `result.isError == true` or a top-level `error` object = failure. The
  `content[].text` then holds the error message (often the upstream API
  status, e.g. `403`, `404`, `409`, `422`).
- Success text is pretty-printed JSON — parse it, do not regex.
- Domain writes can return `outcome` fields like `created` /
  `deduplicated` — a 200 with `deduplicated` means the identity policy
  matched an existing entity (idempotent), NOT an error.

## 4. Argument mapping

Every `inputSchema` property may carry `x-location` telling where the value
goes on the underlying API call:

| `x-location` | Meaning |
|---|---|
| `path` | substituted into the URL path — must look like a real id (`public_id` UUID or catalog id); garbage → tool error |
| `query` | query parameter |
| `body` | JSON body field |
| `header` | request header (e.g. `if_match` → `If-Match`) |
| `binary_body` | base64 → raw bytes (upload chunks) |

No `x-location` = body. Required fields are in `inputSchema.required`.
Path args with slashes/semicolons/`..` are rejected before dispatch
(path-safety check). Missing required → MCP tool error. Unknown extra args
are tolerated by the schema but usually rejected by the API.

## 5. Tool catalog (95 tools, capability-filtered per token)

Common tools by area:

- **Context/dash:** `context.get`, `dashboard.stats`
- **Entities:** `entities.list|get|create|update|retire|history|query`,
  `entities.documents`
- **Relations:** `relations.list|create|update|delete`
- **Schema admin:** `schema.published`, `schema.revisions`,
  `schema.revision_get|retire|migration`, `schema.drafts|draft_create|
  draft_get|draft_update|draft_validate|draft_impact|draft_publish|
  draft_delete`, `schema.types|type_get`
- **Domain (SKR/warehouse):** `checks.*`, `assessment.*`, `factors.*`,
  `tasks.*`, `workflows.*`, `requests.*`, `monitors.*`, `reports.*`
- **Files:** `files.upload_init|upload_chunk|upload_complete|upload_abort|
  get|download|delete`
- **Admin:** `users.list|get|create|update|tokens|token_*|activity`,
  `permissions.groups|group_create|group_update|group_delete`,
  `connectors.*`, `audit.query|checkpoint|verify|retention`

Draft lifecycle for schema changes (order is enforced — skipping a step
returns a tool error): `draft_create` (optional `base_revision`) →
`draft_update` (with `if_match` etag) → `draft_validate` → `draft_impact` →
re-`draft_get` for a fresh etag → `draft_publish` (`reason`, `if_match`).
`revision_retire` only works on the CURRENT published revision — publish a
replacement first; each write bumps the draft version, so always re-fetch
the etag before the next mutating call.

## 6. Entity/field conventions

- `entities.list` filter arg is `type` (type alias, e.g. `wh_warehouse`),
  NOT `type_alias`. `entities.create` takes `type_id` (the type's stable
  alias works as the id) plus `fields` and `identity_mode`
  (`return_existing|reject|allow`). `schema.type_get` takes `alias` (path).
  Always derive exact arg names from `tools/list` `inputSchema` — wrong
  names return `unknown argument '<name>'` as a tool error.
- Entity creation: `{"type_id": "wh_product", "fields":
  {"wh_sku": "...", "name": "..."}}`. Field values: scalars as strings,
  entity refs as `{"entity_id": "<uuid>"}`.
- `identity=true` fields participate in dedup (identity policies are
  derived at schema publish). Re-creating with the same identity values
  returns `outcome: "deduplicated"` and the SAME public_id — success, not
  an error. To avoid cross-run collisions in tests, suffix unique values
  with a run nonce.
- `entities.query` takes `{"query": {"filter": {"field": "<alias>",
  "op": "eq|prefix|contains", "value": "..."}}}` — a non-object `query`
  → 422.
- There is no `wh_supplier` type: suppliers are `legal_entity` entities
  linked via the `wh_supplied_by` predicate (domain `wh_product`, range
  `legal_entity`). Check `schema.predicates`/`schema.types` for real
  domain/type names before wiring relations.
- `relations.create` body: `{"subject_id", "predicate", "object_id"}` —
  duplicate active relation → 409 (idempotent, not a bug).

## 7. Error expectations

| Expectation | How it looks |
|---|---|
| success | `isError` absent/false; JSON in `content[].text` |
| validation | tool error text mentions `422` / `required` / `invalid` / `unknown argument` |
| not found | tool error `404` / `not_found` |
| conflict | tool error `409` (dup relation, hash mismatch, stale etag) |
| auth failure | HTTP `401`, or JSON-RPC `error.code -32001` (bad token) |
| capability deny | **top-level JSON-RPC `error.code -32602`**: `capability '<cap>' not granted to this token` — NOT `result.isError`; checked BEFORE arg validation, so even malformed args deny first |

A correct client must check BOTH failure channels: `error` (protocol-level:
unknown method `-32601`, bad token `-32001`, capability deny `-32602`) and
`result.isError` (tool-level upstream API errors).

## 8. Minimal client (Python)

```python
import json, urllib.request

TOK = "egr_..."            # from docs/testing/warehouse/.wh_tokens.json
MCP = "http://localhost:8020/mcp"
HDR = {"Content-Type": "application/json",
       "Accept": "application/json, text/event-stream",
       "Authorization": f"Bearer {TOK}"}

def rpc(method, params=None, idn=1):
    req = urllib.request.Request(MCP,
        data=json.dumps({"jsonrpc": "2.0", "id": idn,
                         "method": method,
                         **({"params": params} if params else {})}).encode(),
        headers=HDR)
    raw = urllib.request.urlopen(req, timeout=30).read().decode()
    for line in raw.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip(); break
    return json.loads(raw)

def call(tool, args=None, idn=1):
    d = rpc("tools/call", {"name": tool, "arguments": args or {}}, idn)
    if "error" in d: raise RuntimeError(d["error"])
    r = d["result"]
    if r.get("isError"): raise RuntimeError(r["content"][0]["text"])
    txt = r["content"][0]["text"]
    try: return json.loads(txt)
    except ValueError: return txt

rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
    "clientInfo": {"name": "agent", "version": "1.0"}})
tools = rpc("tools/list")["result"]["tools"]
print(call("entities.list", {"type_alias": "wh_warehouse"}))
```

## 9. Gotchas

- The JSON-RPC `id` must be echoed back; batched requests return a batch
  array.
- `tools/list` is cheap — cache it per session and reuse.
- Do not invent tool names or arguments: derive them from `tools/list`
  `inputSchema` (it is the source of truth, including `required`).
- `files.download` / `documents.download` return raw bytes/text, not a JSON
  wrapper — the `content[].text` IS the payload.
- `assessment.get`/`assessment.create` take the ENTITY public_id in `id`
  (`/entities/{id}/assessment`), not an assessment id.
- `permissions.group_update` `principals` expects logins, not UUIDs.
- Test artifacts: `docs/testing/warehouse/wh_test_runner.py` is a complete
  executable catalog of 1257 cases — reuse its `C()`-case patterns.
