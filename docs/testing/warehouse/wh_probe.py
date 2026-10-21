"""MCP probe for warehouse scenario — token-selectable variant of wh_call.py.

Usage:
    python wh_probe.py <tool> '<json-args>' [token=operator|analyst|admin]
    python wh_probe.py tools [token=operator]
    python wh_probe.py schemas '<comma-names>' [token=operator]
    python wh_probe.py batch '<json-array-of-[tool,args]>' [token=operator]
    python wh_probe.py raw '<method>' '<json-params-or-null>' [token=operator]
"""
import json
import os
import sys
import urllib.request
import urllib.error

HERE = os.path.dirname(os.path.abspath(__file__))
MCP = os.environ.get("ENGINER_MCP", "http://localhost:8020/mcp")
TOKENS = json.load(open(os.path.join(HERE, ".wh_tokens.json")))
_idn = [0]
HDR = {"Content-Type": "application/json",
       "Accept": "application/json, text/event-stream"}
_session = {"tok_name": None, "sid": None}


def set_token(name):
    _session["tok_name"] = name
    _session["sid"] = None
    HDR["Authorization"] = "Bearer " + TOKENS[name]


def rpc_raw(method, params=None):
    _idn[0] += 1
    body = {"jsonrpc": "2.0", "id": _idn[0], "method": method}
    if params is not None:
        body["params"] = params
    h = dict(HDR)
    if _session.get("sid"):
        h["Mcp-Session-Id"] = _session["sid"]
    req = urllib.request.Request(MCP, data=json.dumps(body).encode(),
                                 headers=h)
    try:
        resp = urllib.request.urlopen(req, timeout=90)
        sid = resp.headers.get("Mcp-Session-Id") or resp.headers.get(
            "mcp-session-id")
        if sid:
            _session["sid"] = sid
        raw = resp.read().decode()
        code = resp.status
    except urllib.error.HTTPError as e:
        return {"_http": e.code,
                "_body": e.read()[:2000].decode("utf-8", "replace")}
    if not raw.strip():
        return {"_http": code, "_empty": True}
    ctype = resp.headers.get("Content-Type", "")
    for line in raw.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip()
            break
    try:
        out = json.loads(raw)
    except Exception:
        return {"_http": code, "_raw": raw[:2000]}
    if isinstance(out, dict):
        out["_http"] = code
    return out


def call(tool, args=None):
    d = rpc_raw("tools/call", {"name": tool, "arguments": args or {}})
    if "_http" in d and "error" not in d and "result" not in d:
        return d  # transport-level error
    if "error" in d:
        return {"_rpc_err": d["error"], "_http": d.get("_http")}
    r = d.get("result") or {}
    txt = (r.get("content") or [{}])[0].get("text", "")
    try:
        out = json.loads(txt)
    except Exception:
        out = {"_text": txt[:4000]}
    if r.get("isError"):
        if isinstance(out, dict):
            out["_isError"] = True
        else:
            out = {"_isError": True, "_text": txt[:4000]}
    return out


_initialized = [False]


def init():
    if _initialized[0]:
        return
    r = rpc_raw("initialize", {"protocolVersion": "2025-06-18",
                               "capabilities": {},
                               "clientInfo": {"name": "wh-operator-probe",
                                              "version": "1.0"}})
    _initialized[0] = True
    return r


def main():
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    args = sys.argv[1:]
    tok = "operator"
    for i, a in enumerate(args):
        if a.startswith("token="):
            tok = a.split("=", 1)[1]
            args.pop(i)
            break
    set_token(tok)
    init_r = init()
    if init_r:
        print("== initialize ==")
        print(json.dumps(init_r, ensure_ascii=False)[:1200])
    rpc_raw("notifications/initialized")
    cmd = args[0]
    if cmd == "tools":
        tools = rpc_raw("tools/list").get("result", {}).get("tools", [])
        print(f"count={len(tools)}")
        print(json.dumps([t["name"] for t in tools], ensure_ascii=False))
        return 0
    if cmd == "schemas":
        want = args[1].split(",") if len(args) > 1 else None
        tools = rpc_raw("tools/list").get("result", {}).get("tools", [])
        for t in tools:
            if want is None or t["name"] in want:
                print(f"===== {t['name']} =====")
                print(json.dumps(t.get("inputSchema"), ensure_ascii=False))
        return 0
    if cmd == "raw":
        method = args[1]
        params = json.loads(args[2]) if len(args) > 2 and args[2] != "null" \
            else None
        print(json.dumps(rpc_raw(method, params), ensure_ascii=False,
                         indent=1))
        return 0
    if cmd == "batch":
        calls = json.loads(args[1])
        for tool, a in calls:
            print(f"===== {tool} {json.dumps(a, ensure_ascii=False)} =====")
            print(json.dumps(call(tool, a), ensure_ascii=False, indent=1))
        return 0
    a = json.loads(args[1]) if len(args) > 1 else {}
    print(json.dumps(call(cmd, a), ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
