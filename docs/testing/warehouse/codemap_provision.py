"""Code-map provisioning: extends the published schema with a `cm_node`
type + cm_* edge predicates (draft -> validate -> impact -> publish) and
writes one entity per code-map node plus traversable edges.

All writes via MCP tools/call — no raw SQL, no raw HTTP API.
Run:  python docs/testing/warehouse/codemap_provision.py [--nodes-only|--edges-only|--verify-only]
"""
import json
import os
import sys
import urllib.request
import urllib.error

HERE = os.path.dirname(os.path.abspath(__file__))
MCP = os.environ.get("ENGINER_MCP", "http://localhost:8020/mcp")
TOK = json.load(open(os.path.join(HERE, ".wh_tokens.json")))["admin"]
HDR = {"Content-Type": "application/json",
       "Accept": "application/json, text/event-stream",
       "Authorization": "Bearer " + TOK}
_idn = [0]

sys.path.insert(0, HERE)
import codemap_data as CD  # noqa: E402


def rpc(method, params=None):
    _idn[0] += 1
    body = {"jsonrpc": "2.0", "id": _idn[0], "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(MCP, data=json.dumps(body).encode(),
                                 headers=HDR)
    try:
        raw = urllib.request.urlopen(req, timeout=120).read().decode()
    except urllib.error.HTTPError as e:
        return {"_http": e.code,
                "_body": e.read()[:600].decode("utf-8", "replace")}
    for line in raw.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip()
            break
    return json.loads(raw)


def call(tool, args=None):
    d = rpc("tools/call", {"name": tool, "arguments": args or {}})
    if "error" in d:
        return {"_rpc_err": d["error"]}
    r = d.get("result") or {}
    txt = (r.get("content") or [{}])[0].get("text", "")
    try:
        out = json.loads(txt)
    except Exception:
        out = {"_text": txt[:3000]}
    if r.get("isError"):
        if isinstance(out, dict):
            out["_isError"] = True
        else:
            out = {"_isError": True, "_text": txt[:3000]}
    return out


def failed(resp):
    return bool(resp.get("_rpc_err") or resp.get("_http")
                or resp.get("_isError"))


def show(resp, n=600):
    return json.dumps(resp, ensure_ascii=False)[:n]


# ---------------------------------------------------------------- schema

CM_TYPE = {
    "type_id": "cm_node", "stable_alias": "cm_node",
    "entity_class": "data", "label": "Code Map Node",
    "fields": [
        {"kind": "text", "alias": "cm_id", "label": "Code-map id",
         "identity": True, "required": True},
        {"kind": "text", "alias": "cm_kind", "label": "Node kind",
         "required": True},
        {"kind": "text", "alias": "cm_name", "label": "Name",
         "required": True},
        {"kind": "text", "alias": "cm_path", "label": "File/route path"},
        {"kind": "text", "alias": "cm_method", "label": "HTTP method"},
        {"kind": "text", "alias": "cm_capability",
         "label": "PEP capability gate"},
        {"kind": "text", "alias": "cm_summary", "label": "Summary"},
        {"kind": "text", "alias": "cm_detail", "label": "Detail"},
        {"kind": "integer", "alias": "cm_sort", "label": "Sort order"},
        {"kind": "text", "alias": "cm_tags",
         "label": "Tags (comma-separated)"},
    ],
}

CM_SCALARS = [
    {"stable_alias": "cm_id", "scalar_kind": "text", "label": "Code-map id",
     "participates_in_identity": True},
    {"stable_alias": "cm_kind", "scalar_kind": "text", "label": "Node kind"},
    {"stable_alias": "cm_name", "scalar_kind": "text", "label": "Name"},
    {"stable_alias": "cm_path", "scalar_kind": "text",
     "label": "File/route path"},
    {"stable_alias": "cm_method", "scalar_kind": "text",
     "label": "HTTP method"},
    {"stable_alias": "cm_capability", "scalar_kind": "text",
     "label": "PEP capability gate"},
    {"stable_alias": "cm_summary", "scalar_kind": "text",
     "label": "Summary"},
    {"stable_alias": "cm_detail", "scalar_kind": "text", "label": "Detail"},
    {"stable_alias": "cm_sort", "scalar_kind": "integer",
     "label": "Sort order"},
    {"stable_alias": "cm_tags", "scalar_kind": "text",
     "label": "Tags (comma-separated)"},
]

CM_EDGES = [
    {"stable_alias": "cm_contains", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "contains", "inverse_label": "contained in"},
    {"stable_alias": "cm_depends", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "depends on", "inverse_label": "depended on by"},
    {"stable_alias": "cm_exposes", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "exposes route", "inverse_label": "exposed by"},
    {"stable_alias": "cm_maps_to", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "maps to", "inverse_label": "mapped from"},
    {"stable_alias": "cm_calls", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "calls", "inverse_label": "called by"},
    {"stable_alias": "cm_uses", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "uses", "inverse_label": "used by"},
    {"stable_alias": "cm_persists", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "persists to", "inverse_label": "persisted by"},
    {"stable_alias": "cm_defines", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "defines", "inverse_label": "defined by"},
    {"stable_alias": "cm_tests", "is_edge": True,
     "domain": "cm_node", "range": "cm_node",
     "label": "tests", "inverse_label": "tested by"},
    {"stable_alias": "cm_implements", "is_edge": True,
     "domain": "cm_node", "range": "kb_article",
     "label": "implements spec", "inverse_label": "implemented by"},
]


def publish_schema():
    pub = call("schema.published")
    rows = pub.get("data") or []
    base = rows[0]["revision_id"] if rows else None
    print("base published revision:", base)

    d = call("schema.draft_create",
             {"name": "code-map", "base_revision": str(base)})
    if failed(d):
        print("draft_create FAILED:", show(d)); sys.exit(1)
    draft = d.get("data") or {}
    did = str(draft.get("revision_id") or draft.get("id"))
    etag = draft.get("etag")
    if not etag or not (draft.get("types") or draft.get("predicates")):
        g = call("schema.draft_get", {"id": did})
        draft = g.get("data") or draft
        etag = draft.get("etag") or etag
    print("draft:", did, "etag:", etag)

    types = list(draft.get("types") or [])
    preds = list(draft.get("predicates") or [])
    existing_t = {t.get("stable_alias") or t.get("type_id") for t in types}
    existing_p = {p.get("stable_alias") for p in preds}
    if "cm_node" not in existing_t:
        types.append(CM_TYPE)
    for p in CM_SCALARS + CM_EDGES:
        if p["stable_alias"] not in existing_p:
            preds.append(p)
    print(f"draft has {len(types)} types, {len(preds)} predicates")

    u = call("schema.draft_update", {
        "id": did, "display_name": "Project KB + code map",
        "semantic_version": "1.1.0",
        "description": "kb_article + cm_node code-map type with cm_* edges",
        "types": types, "predicates": preds,
        "changes": [{"kind": "add", "resource_kind": "type",
                     "resource_id": "cm_node",
                     "summary": "Add code-map node type + cm_* edges"}],
        "if_match": etag})
    if failed(u):
        print("draft_update FAILED:", show(u)); sys.exit(1)

    v = call("schema.draft_validate", {"id": did})
    print("validate:", show(v, 400))
    if failed(v):
        sys.exit(1)
    im = call("schema.draft_impact", {"id": did})
    print("impact:", show(im, 400))
    if failed(im):
        sys.exit(1)
    g = call("schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag
    p = call("schema.draft_publish", {
        "id": did, "reason": "Add code-map node type + cm_* edges",
        "if_match": etag})
    if failed(p):
        print("publish FAILED:", show(p)); sys.exit(1)
    pub_rev = (p.get("data") or {}).get("revision_id", did)
    print("PUBLISHED revision:", pub_rev)
    return str(pub_rev)


# ---------------------------------------------------------------- nodes

def create_nodes():
    pid_by_cm = {}
    n_ok = n_dup = n_fail = 0
    for nd in CD.NODES:
        fields = {
            "cm_id": nd["cm_id"],
            "cm_kind": nd["kind"],
            "cm_name": nd["name"],
            "cm_path": nd["path"],
            "cm_method": nd["method"],
            "cm_capability": nd["cap"],
            "cm_summary": nd["summary"],
            "cm_detail": nd["detail"],
            "cm_sort": nd["sort"],
            "cm_tags": nd["tags"],
        }
        r = call("entities.create", {"type_id": "cm_node",
                                     "fields": fields})
        if failed(r) and isinstance(fields["cm_sort"], int):
            fields["cm_sort"] = str(fields["cm_sort"])
            r = call("entities.create", {"type_id": "cm_node",
                                         "fields": fields})
        if failed(r):
            print(f"CREATE FAILED {nd['cm_id']}:", show(r, 300))
            n_fail += 1
            continue
        data = r.get("data") or r
        pid = data.get("public_id") or data.get("id")
        outcome = data.get("outcome", "created")
        pid_by_cm[nd["cm_id"]] = pid
        if outcome == "deduplicated":
            n_dup += 1
        else:
            n_ok += 1
    print(f"nodes: {n_ok} created, {n_dup} deduplicated, {n_fail} failed")
    return pid_by_cm


def _query_page(typ, after=None):
    q = {"from": {"type_id": typ}, "page": {"size": 500}}
    if after:
        q["page"]["after"] = after
    r = call("entities.query", {"query": q})
    data = r.get("data") or {}
    return data.get("rows") or [], data.get("cursor")


def resolve_all():
    """cm_id/kb_id -> public_id map from the DB (authoritative)."""
    m = {}
    for typ, key in (("cm_node", "cm_id"), ("kb_article", "kb_id")):
        after = None
        total = 0
        for _ in range(20):
            rows, after = _query_page(typ, after)
            for row in rows:
                v = row.get(key)
                if v:
                    m[v] = row["public_id"]
            total += len(rows)
            if not after:
                break
        print(f"resolved {typ}: {total} rows")
    return m


def create_edges(pid_map):
    ok = dup = fail = 0
    for s, pred, o in CD.EDGES:
        sid, oid = pid_map.get(s), pid_map.get(o)
        if not sid or not oid:
            print("SKIP (unresolved):", s, pred, o)
            fail += 1
            continue
        r = call("relations.create", {
            "id": sid, "predicate": pred, "target_entity_id": oid})
        if failed(r):
            txt = show(r, 200)
            if "409" in txt or "conflict" in txt.lower():
                dup += 1
            else:
                print(f"REL FAILED {s} -{pred}-> {o}:", txt)
                fail += 1
        else:
            ok += 1
    print(f"edges: {ok} created, {dup} already existed, {fail} failed")


# ---------------------------------------------------------------- verify

def verify(pid_map):
    print("\n== VERIFY ==")
    from collections import Counter
    r = call("entities.list", {"type": "cm_node", "limit": 500})
    rows = r.get("data") or []
    kinds = Counter((row.get("fields") or {}).get("cm_kind")
                    for row in rows)
    print("cm_node count:", len(rows))
    print("by kind:", dict(kinds))

    for probe in ("app.api", "crate.domain", "route.post.api.v1.entities",
                  "tool.entities.query"):
        pid = pid_map.get(probe)
        g = call("entities.get", {"id": pid}) if pid else {"_skip": True}
        ok = not failed(g)
        print(f"entities.get {probe}: {'ok' if ok else show(g, 200)}")

    pid = pid_map.get("app.api")
    if pid:
        g = call("entities.graph",
                 {"id": pid, "direction": "both", "max_depth": 2})
        d = g.get("data") or {}
        print("graph from app.api depth2:",
              len(d.get("nodes") or []), "nodes,",
              len(d.get("edges") or []), "edges")

    q = call("entities.query", {"query": {
        "from": {"type_id": "cm_node"},
        "where": {"field": "cm_kind", "op": "eq", "value": "route"}}})
    rows = ((q.get("data") or {}).get("rows")
            if isinstance(q.get("data"), dict) else q.get("data")) or []
    print("entities.query kind=route:", len(rows))
    q = call("entities.query", {"query": {
        "from": {"type_id": "cm_node"},
        "where": {"field": "cm_path", "op": "eq",
                  "value": "/api/v1/entities"}}})
    rows = ((q.get("data") or {}).get("rows")
            if isinstance(q.get("data"), dict) else q.get("data")) or []
    print("query cm_path=/api/v1/entities:", len(rows),
          [((r.get('fields') or {}).get('cm_method') if isinstance(r, dict)
            else None) for r in rows[:4]])


if __name__ == "__main__":
    rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "codemap", "version": "1.0"}})
    rpc("notifications/initialized")

    mode = sys.argv[1] if len(sys.argv) > 1 else "all"
    if mode in ("all", "schema"):
        publish_schema()
    pid_map = {}
    if mode in ("all", "nodes", "edges", "verify"):
        pid_map = resolve_all()
        print("resolved ids:", len(pid_map))
    if mode in ("all", "nodes"):
        created = create_nodes()
        pid_map.update(created)
    if mode in ("all", "edges"):
        if len(pid_map) < 10:
            pid_map = resolve_all()
        create_edges(pid_map)
    if mode in ("all", "verify"):
        if not pid_map:
            pid_map = resolve_all()
        verify(pid_map)
