"""MCP tools/call helper for the 200-prompt warehouse scenario.

Usage:
    python wh_call.py <tool> '<json-args>'     -> prints parsed result JSON
    python wh_call.py tools                    -> prints tools/list names
    python wh_call.py schemas                  -> prints inputSchema of key tools
    python wh_call.py batch '<json-array-of-[tool,args]>' -> sequential calls

Exit code 0 always (errors are printed, not raised) — except argc errors.
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


def rpc_raw(method, params=None):
    _idn[0] += 1
    body = {"jsonrpc": "2.0", "id": _idn[0], "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(MCP, data=json.dumps(body).encode(),
                                 headers=HDR)
    try:
        raw = urllib.request.urlopen(req, timeout=90).read().decode()
    except urllib.error.HTTPError as e:
        return {"_http": e.code, "_body": e.read()[:1500].decode("utf-8",
                                                                 "replace")}
    if not raw.strip():
        return {}
    for line in raw.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip()
            break
    try:
        return json.loads(raw)
    except Exception:
        return {"_raw": raw[:1500]}


def call(tool, args=None):
    d = rpc_raw("tools/call", {"name": tool, "arguments": args or {}})
    if "error" in d:
        return {"_rpc_err": d["error"]}
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


def failed(resp):
    return bool(resp.get("_rpc_err") or resp.get("_http")
                or resp.get("_isError") or resp.get("error"))


def pub_id(resp):
    d = resp.get("data") or {}
    return d.get("public_id") or d.get("id") or d.get("entity_id")


_initialized = [False]


def init():
    if _initialized[0]:
        return
    rpc_raw("initialize", {"protocolVersion": "2025-06-18",
                           "capabilities": {},
                           "clientInfo": {"name": "wh-manager",
                                          "version": "1.0"}})
    rpc_raw("notifications/initialized")
    _initialized[0] = True


def emit(s):
    sys.stdout.buffer.write(s.encode("utf-8") + b"\n")
    sys.stdout.buffer.flush()


def main():
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    init()
    cmd = sys.argv[1]
    if cmd == "tools":
        tools = rpc_raw("tools/list").get("result", {}).get("tools", [])
        print(json.dumps([t["name"] for t in tools], ensure_ascii=False))
        return 0
    if cmd == "schemas":
        tools = rpc_raw("tools/list").get("result", {}).get("tools", [])
        want = sys.argv[2].split(",") if len(sys.argv) > 2 else None
        for t in tools:
            if want is None or t["name"] in want:
                print(f"===== {t['name']} =====")
                print(json.dumps(t.get("inputSchema"), ensure_ascii=False))
        return 0
    if cmd == "batch":
        calls = json.loads(sys.argv[2])
        out = []
        for tool, args in calls:
            out.append({"tool": tool, "result": call(tool, args)})
        print(json.dumps(out, ensure_ascii=False, indent=1))
        return 0
    args = json.loads(sys.argv[2]) if len(sys.argv) > 2 else {}
    print(json.dumps(call(cmd, args), ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
