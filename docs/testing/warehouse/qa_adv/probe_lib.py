"""Shared probe harness for adversarial QA against live EngineR stack."""
import json, os, urllib.request, urllib.error, urllib.parse, time

API = "http://localhost:8010/api/v1"
MCP = "http://localhost:8020/mcp"
TOK = json.load(open(os.path.join(os.path.dirname(__file__), "..", ".wh_tokens.json")))

RESULTS = []

def rec(scenario, name, expected, actual, verdict):
    RESULTS.append({"scenario": scenario, "probe": name, "expected": expected,
                    "actual": actual, "verdict": verdict})
    print(f"[{verdict}] {scenario} :: {name}")
    print(f"      expected: {expected}")
    print(f"      actual  : {actual}")

def api(persona, method, path, body=None, headers=None, raw=None):
    """HTTP call to API. Returns (status, parsed_or_text, headers)."""
    url = API + path
    data = None
    h = {"Authorization": "Bearer " + TOK[persona]}
    if raw is not None:
        data = raw if isinstance(raw, bytes) else raw.encode("utf-8")
        h.setdefault("Content-Type", "application/octet-stream")
    elif body is not None:
        data = json.dumps(body).encode("utf-8")
        h["Content-Type"] = "application/json"
    if headers:
        h.update(headers)
    req = urllib.request.Request(url, data=data, method=method, headers=h)
    try:
        r = urllib.request.urlopen(req, timeout=60)
        b = r.read()
        try:
            return r.status, json.loads(b), dict(r.headers)
        except Exception:
            return r.status, b, dict(r.headers)
    except urllib.error.HTTPError as e:
        b = e.read()
        try:
            return e.code, json.loads(b), dict(e.headers)
        except Exception:
            try:
                return e.code, b.decode("utf-8", "replace"), dict(e.headers)
            except Exception:
                return e.code, repr(b[:300]), dict(e.headers)

def mcp(persona, method, params=None, rid=1, notification=False):
    """JSON-RPC call to MCP. Returns (status, json)."""
    msg = {"jsonrpc": "2.0", "method": method}
    if not notification:
        msg["id"] = rid
    if params is not None:
        msg["params"] = params
    data = json.dumps(msg).encode("utf-8")
    req = urllib.request.Request(MCP, data=data, method="POST",
        headers={"Authorization": "Bearer " + TOK[persona],
                 "Content-Type": "application/json",
                 "Accept": "application/json"})
    try:
        r = urllib.request.urlopen(req, timeout=60)
        return r.status, json.loads(r.read() or b"null")
    except urllib.error.HTTPError as e:
        b = e.read()
        try:
            return e.code, json.loads(b)
        except Exception:
            return e.code, b.decode("utf-8", "replace")[:300]

def brief(resp):
    """Compact description of an api() result."""
    st, body, _ = resp
    if isinstance(body, dict):
        if "error" in body:
            e = body["error"]
            return f"{st} err:{e.get('code')}:{str(e.get('message'))[:80]}"
        d = body.get("data")
        s = json.dumps(d, ensure_ascii=True)
        return f"{st} {s[:160]}"
    if isinstance(body, bytes):
        return f"{st} bytes:{len(body)}"
    return f"{st} {str(body)[:160]}"

def etag_of(resp):
    return resp[2].get("ETag") or resp[2].get("etag")

def sql(q):
    """Run a read query via docker exec psql; returns stdout text."""
    import subprocess
    r = subprocess.run(["docker", "exec", "-i", "enginer-postgres",
                        "psql", "-U", "enginer_admin", "-d", "enginer",
                        "-tA", "-c", q],
                       capture_output=True, text=True, timeout=60)
    return (r.stdout or "").strip() + (" |ERR|" + r.stderr.strip() if r.returncode else "")

def sql_i(q):
    """Run any SQL (writes allowed); returns combined output."""
    import subprocess
    r = subprocess.run(["docker", "exec", "-i", "enginer-postgres",
                        "psql", "-U", "enginer_admin", "-d", "enginer",
                        "-tA", "-v", "ON_ERROR_STOP=1", "-c", q],
                       capture_output=True, text=True, timeout=60)
    return r.returncode, (r.stdout or "").strip(), (r.stderr or "").strip()

TENANT = None
def tenant_id():
    global TENANT
    if TENANT is None:
        TENANT = sql("SELECT tenant_id FROM core.tenant LIMIT 1").split("\n")[0]
    return TENANT

def wait(pred, tries=30, delay=1.0):
    for _ in range(tries):
        if pred():
            return True
        time.sleep(delay)
    return False
