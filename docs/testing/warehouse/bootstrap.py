"""Warehouse-suite bootstrap: login as admin, mint bearer tokens for the
existing qa-mcp-admin / qa-mcp-analyst service accounts. Saved to .wh_tokens.json.
Usage: python docs/testing/warehouse/bootstrap.py
"""
import http.cookiejar
import json
import urllib.error
import urllib.request
import os
import sys

API = os.environ.get("ENGINER_API", "http://localhost:8010")
HERE = os.path.dirname(os.path.abspath(__file__))


class Api:
    def __init__(self, base, login, password):
        self.base = base
        self.cj = http.cookiejar.CookieJar()
        self.op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.cj))
        self.csrf = None
        r = self.req("POST", "/api/v1/auth/login", {"username": login, "password": password})
        assert r.status == 200, f"login failed: {r.status} {r.read()[:200]}"
        try:
            self.csrf = json.loads(r.read())["data"]["csrf_token"]
        except Exception:
            self.csrf = None

    def req(self, method, path, body=None, headers=None):
        h = {"Content-Type": "application/json"}
        if self.csrf and method != "GET" and not path.endswith("/auth/login"):
            h["X-CSRF-Token"] = self.csrf
        if headers:
            h.update(headers)
        req = urllib.request.Request(
            self.base + path,
            data=json.dumps(body).encode() if body is not None else None,
            headers=h, method=method)
        try:
            return self.op.open(req)
        except urllib.error.HTTPError as e:
            e.body = e.read()
            return e

    def json(self, method, path, body=None, headers=None):
        r = self.req(method, path, body, headers)
        try:
            return r.status, json.loads(r.read() if not hasattr(r, "body") else r.body)
        except Exception:
            return r.status, {}


def ensure_user(api, login, groups):
    st, r = api.json("POST", "/api/v1/users", {
        "login": login, "display_name": f"QA {login}", "is_service_account": True,
        "groups": groups})
    if st == 200:
        return r["data"]["principal_id"]
    st, lst = api.json("GET", f"/api/v1/users?q={login}")
    for u in lst.get("data", []):
        if u.get("login") == login:
            api.json("PUT", f"/api/v1/users/{u['principal_id']}",
                     {"status": "active", "groups": groups})
            return u["principal_id"]
    raise RuntimeError(f"cannot provision {login}: {st} {r}")


def mint(api, pid, name):
    st, r = api.json("POST", f"/api/v1/users/{pid}/tokens", {"name": name})
    assert st == 200, f"token mint failed: {r}"
    return r["data"]["token"]


def main():
    login = os.environ.get("ENGINER_LOGIN", "admin")
    password = os.environ.get("ENGINER_PASSWORD", "enginer-demo")
    api = Api(API, login, password)
    admin_id = ensure_user(api, "qa-mcp-admin", ["admins", "all_data"])
    analyst_id = ensure_user(api, "qa-mcp-analyst", ["analysts", "all_data"])
    # warehouse operator: read-only-ish role to probe authorization boundaries
    operator_id = ensure_user(api, "qa-wh-operator", ["analysts", "all_data"])
    tokens = {
        "admin": mint(api, admin_id, "wh-runner"),
        "analyst": mint(api, analyst_id, "wh-runner"),
        "operator": mint(api, operator_id, "wh-runner"),
        "admin_pid": admin_id,
        "analyst_pid": analyst_id,
        "operator_pid": operator_id,
    }
    out = os.path.join(HERE, ".wh_tokens.json")
    with open(out, "w", encoding="utf-8") as f:
        json.dump(tokens, f)
    print("tokens saved to", out)


if __name__ == "__main__":
    sys.exit(main())
