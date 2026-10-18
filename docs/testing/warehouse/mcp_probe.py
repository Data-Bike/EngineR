"""Scratch MCP probe — inspect tools, published schema, draft_create shape."""
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


def rpc_raw(method, params=None):
    _idn[0] += 1
    body = {"jsonrpc": "2.0", "id": _idn[0], "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(MCP, data=json.dumps(body).encode(), headers=HDR)
    try:
        raw = urllib.request.urlopen(req, timeout=60).read().decode()
    except urllib.error.HTTPError as e:
        return {"_http": e.code, "_body": e.read()[:500].decode("utf-8", "replace")}
    for line in raw.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip()
            break
    return json.loads(raw)


def call(tool, args=None):
    d = rpc_raw("tools/call", {"name": tool, "arguments": args or {}})
    if "error" in d:
        return {"_rpc_err": d["error"]}
    r = d.get("result") or {}
    txt = (r.get("content") or [{}])[0].get("text", "")
    try:
        out = json.loads(txt)
    except Exception:
        out = {"_text": txt[:2000]}
    if r.get("isError"):
        if isinstance(out, dict):
            out["_isError"] = True
        else:
            out = {"_isError": True, "_text": txt[:2000]}
    return out


def short(x, n=1500):
    return json.dumps(x, ensure_ascii=False)[:n]


if __name__ == "__main__":
    print("== initialize ==")
    print(short(rpc_raw("initialize", {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "probe", "version": "1.0"}}), 400))
    rpc_raw("notifications/initialized")

    cmd = sys.argv[1] if len(sys.argv) > 1 else "all"

    if cmd in ("all", "tools"):
        tools = rpc_raw("tools/list")["result"]["tools"]
        names = [t["name"] for t in tools]
        print(f"\n== {len(names)} tools ==")
        print(json.dumps(names, ensure_ascii=False))

    if cmd in ("all", "published"):
        print("\n== schema.published ==")
        print(short(call("schema.published"), 800))

    if cmd in ("all", "schemas"):
        tools = rpc_raw("tools/list")["result"]["tools"]
        for t in tools:
            if t["name"] in ("schema.draft_create", "schema.draft_update",
                             "entities.create", "relations.create",
                             "entities.list", "entities.graph",
                             "audit.query", "context.get", "relations.list",
                             "schema.type_get", "entities.get"):
                print(f"\n== {t['name']} inputSchema ==")
                print(json.dumps(t.get("inputSchema"), ensure_ascii=False)[:3000])
