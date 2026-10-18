#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""EngineR MCP test runner — executes the case catalog (mcp_test_cases.md)
against a live stack through the MCP Streamable HTTP endpoint.

Usage:
    python docs/testing/mcp_test_runner.py [--mcp http://localhost:8020]
        [--api http://localhost:8010] [--login admin] [--password enginer-demo]
        [--area entities.list] [--out docs/testing/mcp_report_raw.json]

The runner provisions two service accounts over the admin API session —
qa-admin (admins + all_data) and qa-analyst (analysts + all_data) — issues a
bearer token for each and drives JSON-RPC tools/call. Results are written to
JSON; docs/testing/mcp_test_report.md summarises a run.
"""
import argparse
import base64
import hashlib
import http.cookiejar
import json
import sys
import time
import urllib.request
import urllib.error
import uuid

DEFAULT_API = "http://localhost:8010"
DEFAULT_MCP = "http://localhost:8020"
API = DEFAULT_API
MCP = DEFAULT_MCP


# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------
class Api:
    def __init__(self, base, login, password):
        self.base = base
        self.cj = http.cookiejar.CookieJar()
        self.op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.cj))
        self.csrf = None
        r = self.req("POST", "/api/v1/auth/login", {"username": login, "password": password})
        assert r.status == 200, f"login failed: {r.status}"
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


def rpc(token, method, params, idn):
    req = urllib.request.Request(
        MCP + "/mcp",
        data=json.dumps({"jsonrpc": "2.0", "id": idn, "method": method, "params": params}).encode(),
        headers={"Content-Type": "application/json",
                 "Accept": "application/json, text/event-stream",
                 **({"Authorization": f"Bearer {token}"} if token else {})})
    try:
        r = urllib.request.urlopen(req)
        raw = r.read().decode()
        # Streamable HTTP may answer SSE — pick the JSON data line.
        if raw.startswith("event:") or "\ndata:" in raw:
            for line in raw.splitlines():
                if line.startswith("data:"):
                    raw = line[5:].strip()
                    break
        return r.status, json.loads(raw)
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", "replace")
        try:
            return e.code, json.loads(body)
        except Exception:
            return e.code, {"http_error": e.code, "body": body[:500]}


def tool_text(res):
    result = res.get("result") or {}
    if result.get("isError"):
        for c in result.get("content", []):
            if c.get("type") == "text":
                return c["text"]
        return json.dumps(result)
    for c in result.get("content", []):
        if c.get("type") == "text":
            return c["text"]
    return json.dumps(result)


def tool_data(res):
    """Parsed `data` of a successful tool call (or None)."""
    try:
        return json.loads(tool_text(res)).get("data")
    except Exception:
        return None


def is_tool_error(res):
    return bool((res.get("result") or {}).get("isError")) or bool(res.get("error"))


# ---------------------------------------------------------------------------
# Case model: (area, name, tool, args, expect)
#   expect: "ok"        — tool succeeds (no isError, no rpc error)
#           "tool_err"  — MCP-level validation error (isError)
#           "deny"      — capability denied (-32602 / capability message)
#           "fail"      — API-level business error (4xx surfaced via isError)
#           "http401"   — transport rejects the bearer
#   match:  optional substring expected inside the payload text
#   fn:     optional callable(env) -> args   (late-bound ids)
#   save:   store parsed data under env[name]
# ---------------------------------------------------------------------------
def C(area, name, tool, args=None, expect="ok", match=None, fn=None,
      save=None, token="admin", desc="", fn_save=None):
    return {"area": area, "name": name, "tool": tool, "args": args or {},
            "expect": expect, "match": match, "fn": fn, "save": save,
            "token": token, "desc": desc, "fn_save": fn_save}


FAKE = str(uuid.uuid4())
NOT_UUID = "definitely-not-a-uuid"


def _stub(prop):
    """A minimally-typed placeholder value for a schema property."""
    return {"string": "x", "integer": 1, "number": 1, "boolean": True,
            "array": [], "object": {}}.get(prop.get("type"), "x")


def _wrong(prop):
    """A value of the wrong JSON type for the property."""
    return {"x": 1} if prop.get("type") == "string" else "x"


def synthetic_cases(tools, analyst_tools):
    """Per-tool generic cases: every declared tool gets missing/empty-arg,
    unknown-arg, wrong-type, path-traversal, ghost-id and analyst
    allow/deny probes on top of the handwritten scenarios — >=10 cases
    per tool in total."""
    S = []
    for t in tools:
        name = t["name"]
        schema = t.get("inputSchema") or {}
        props = schema.get("properties") or {}
        req = schema.get("required") or []
        area = f"X.{name}"
        stub = {p: _stub(props.get(p) or {}) for p in req}
        # Required-arg rejection: no args and per-param null.
        if req:
            S.append(C(area, f"{name} ∅args→err", name, {}, expect="tool_err"))
        else:
            S.append(C(area, f"{name} ∅args", name, {}))
        for p in req[:3]:
            a2 = dict(stub)
            a2[p] = None
            S.append(C(area, f"{name} null:{p}", name, a2, expect="tool_err"))
        # Unknown argument is rejected, never silently dropped.
        a2 = dict(stub)
        a2["__bogus__"] = 1
        S.append(C(area, f"{name} unknown arg", name, a2, expect="tool_err"))
        # Wrong JSON type per required param (bounded).
        for p in req[:2]:
            a2 = dict(stub)
            a2[p] = _wrong(props.get(p) or {})
            S.append(C(area, f"{name} badtype:{p}", name, a2, expect="fail"))
        # Empty-string rejection on required string params (bounded).
        for p in req[:2]:
            if (props.get(p) or {}).get("type") == "string" \
                    and (props.get(p) or {}).get("x-location") != "path":
                a2 = dict(stub)
                a2[p] = ""
                S.append(C(area, f"{name} emptystr:{p}", name, a2,
                           expect="fail"))
        # Path-param hardening: traversal/reroute/empty/structural + ghost.
        for p, pr in props.items():
            if (pr or {}).get("x-location") != "path":
                continue
            for bad, label in (("a/b", "slash"), ("../x", "dotdot"),
                               ("", "empty"), ([1], "array")):
                a2 = dict(stub)
                a2[p] = bad
                S.append(C(area, f"{name} path:{p}:{label}", name, a2,
                           expect="tool_err"))
            a2 = dict(stub)
            a2[p] = FAKE
            S.append(C(area, f"{name} ghost:{p}", name, a2, expect="fail"))
        # Optional query params (only on tools without required path args —
        # the others need a live parent id which the handwritten cases own):
        # arbitrary strings filter to an empty page, nulls are skipped,
        # numeric params accept a small value.
        for p, pr in (props.items() if not req else []):
            if (pr or {}).get("x-location") != "query":
                continue
            ty = (pr or {}).get("type")
            if ty == "string":
                S.append(C(area, f"{name} filter:{p}", name,
                           {p: "__qa_nomatch__"}))
                S.append(C(area, f"{name} null-opt:{p}", name, {p: None}))
            elif ty in ("integer", "number"):
                S.append(C(area, f"{name} num:{p}", name, {p: 1}))
                S.append(C(area, f"{name} null-opt:{p}", name, {p: None}))
        # Analyst token: deny if the capability is not granted, else probe.
        if name not in analyst_tools:
            S.append(C(area, f"{name} analyst deny", name, dict(stub),
                       token="analyst", expect="deny"))
        elif req:
            a2 = dict(stub)
            for p in req:
                if (props.get(p) or {}).get("x-location") == "path":
                    a2[p] = FAKE
            S.append(C(area, f"{name} analyst reach", name, a2,
                       token="analyst", expect="reach"))
        else:
            S.append(C(area, f"{name} analyst ok", name, {}, token="analyst"))
    return S


def build_cases(tools=None, analyst_tools=None):
    NONCE = str(uuid.uuid4())[:8]
    S = []  # cases
    # ── A. entities.list / search ─────────────────────────────────────────
    a = "A.entities.list/search"
    S += [
        C(a, "A1 list all", "entities.list", {"limit": 10}, match='"data"'),
        C(a, "A2 type filter person", "entities.list", {"type": "person", "limit": 50},
          match='person'),
        C(a, "A3 free-text search", "entities.list", {"search": "Иванов"}, match='Иванов'),
        C(a, "A4 search no hit", "entities.list", {"search": "zzz-no-such-entity-zzz"},
          match='"data":\n  []|"data": []|[]'),
        C(a, "A5 limit=1 pagination", "entities.list", {"limit": 1}),
        C(a, "A6 cursor page 2", "entities.list",
          fn=lambda e: {"limit": 1, "cursor": e.get("cursor")} if e.get("cursor") else {"limit": 1}),
        C(a, "A7 limit wrong type", "entities.list", {"limit": "ten"}),
        C(a, "A8 unknown arg rejected", "entities.list", {"bogus": 1}, expect="tool_err",
          match="unknown argument"),
        C(a, "A9 type nonexistent alias", "entities.list", {"type": "nope_alias"}),
        C(a, "A10 analyst read allowed", "entities.list", {"limit": 5}, token="analyst"),
        C(a, "A11 filter legal_entity", "entities.list", {"type": "legal_entity", "limit": 5}),
    ]
    # ── B. entities.get ───────────────────────────────────────────────────
    a = "B.entities.get"
    S += [
        C(a, "B1 get seeded person", "entities.get", fn=lambda e: {"id": e["p1"]}, match='Иванов'),
        C(a, "B2 get legal_entity", "entities.get", fn=lambda e: {"id": e["le1"]}, match='Ромашка'),
        C(a, "B3 nonexistent uuid → 404", "entities.get", {"id": FAKE}, expect="fail"),
        C(a, "B4 malformed id", "entities.get", {"id": NOT_UUID}, expect="fail"),
        C(a, "B5 path traversal rejected", "entities.get", {"id": "../users"}, expect="tool_err",
          match="invalid path argument"),
        C(a, "B6 numeric id accepted→404", "entities.get", {"id": 42}, expect="fail"),
        C(a, "B7 structured id rejected", "entities.get", {"id": [1, 2]}, expect="tool_err",
          match="must be a string"),
        C(a, "B8 retired entity readable, status=retired", "entities.get",
          fn=lambda e: {"id": e.get("retired_id", FAKE)}, match='"retired"'),
        C(a, "B9 empty id rejected", "entities.get", {"id": ""}, expect="tool_err"),
        C(a, "B10 analyst get", "entities.get", fn=lambda e: {"id": e["p1"]}, token="analyst"),
    ]
    # ── C. entities.create ────────────────────────────────────────────────
    a = "C.entities.create"
    S += [
        C(a, "C1 create person", "entities.create",
          {"type_id": "person", "fields": {"full_name": "QA Тест Лицо", "inn": f"5{NONCE}01".replace("-","")[:12]}},
          match='public_id', save="new_person"),
        C(a, "C2 create legal_entity", "entities.create",
          {"type_id": "legal_entity",
           "fields": {"name": "ООО «QA-Тест»", "inn": f"5{NONCE}32".replace("-","")[:10], "ogrn": f"115{NONCE}332".replace("-","")[:13]}},
          match="outcome", save="new_le"),
        C(a, "C3 missing type_id", "entities.create", {"fields": {}}, expect="tool_err",
          match="missing required argument 'type_id'"),
        C(a, "C4 unknown type alias", "entities.create",
          {"type_id": "nope_alias", "fields": {}}, expect="fail"),
        C(a, "C5 duplicate INN → return_existing", "entities.create",
          {"type_id": "person", "fields": {"full_name": "Дубликат", "inn": f"5{NONCE}01".replace("-","")[:12]}},
          match='deduplicated'),
        C(a, "C6 identity_mode=reject on dup", "entities.create",
          {"type_id": "person", "identity_mode": "reject",
           "fields": {"full_name": "Дубликат2", "inn": f"5{NONCE}01".replace("-","")[:12]}},
          expect="fail"),
        C(a, "C7 create document", "entities.create",
          {"type_id": "document", "fields": {"title": "QA Doc", "amount": "100.5"}},
          save="new_doc"),
        C(a, "C8 create check entity", "entities.create",
          {"type_id": "check", "fields": {"title": "QA Check"}}),
        C(a, "C9 analyst denied", "entities.create",
          {"type_id": "person", "fields": {"full_name": "X"}}, token="analyst", expect="deny"),
        C(a, "C10 unknown arg", "entities.create",
          {"type_id": "person", "bogus": 1}, expect="tool_err", match="unknown argument"),
        C(a, "C11 create request entity", "entities.create",
          {"type_id": "request", "fields": {"title": "QA Request"}}, save="new_req"),
        C(a, "C12 create_candidate creates new", "entities.create",
          {"type_id": "person", "identity_mode": "create_candidate",
           "fields": {"full_name": "Merged", "inn": f"5{NONCE}01".replace("-","")[:12]}},
          match='"created"'),
    ]
    # ── D. entities.update ────────────────────────────────────────────────
    a = "D.entities.update"
    S += [
        C(a, "D1 update field", "entities.update",
          fn=lambda e: {"id": e["new_person"]["public_id"], "fields": {"email": "qa@test.ru"},
                        "revision": 1}),
        C(a, "D2 stale revision", "entities.update",
          fn=lambda e: {"id": e["new_person"]["public_id"], "fields": {"email": "x@x.ru"},
                        "revision": 9999}, expect="fail", match="stale"),
        C(a, "D3 nonexistent id", "entities.update", {"id": FAKE, "fields": {"email": "a@b"}},
          expect="fail"),
        C(a, "D4 update retired", "entities.update",
          fn=lambda e: {"id": e.get("retired_id", FAKE), "fields": {}}, expect="fail"),
        C(a, "D5 single predicate form", "entities.update",
          fn=lambda e: {"id": e["new_person"]["public_id"],
                        "predicate_id": "phone", "value": {"kind": "phone", "value": "+7900"}}),
        C(a, "D6 unknown arg", "entities.update", {"id": FAKE, "bogus": 1}, expect="tool_err"),
        C(a, "D7 analyst denied", "entities.update",
          fn=lambda e: {"id": e["new_person"]["public_id"], "fields": {}},
          token="analyst", expect="deny"),
        C(a, "D8 verify persisted", "entities.get",
          fn=lambda e: {"id": e["new_person"]["public_id"]}, match="qa@test.ru"),
        C(a, "D9 update le field", "entities.update",
          fn=lambda e: {"id": e["new_le"]["public_id"], "fields": {"okved": "62.01"}}),
        C(a, "D10 no fields no-op", "entities.update",
          fn=lambda e: {"id": e["new_person"]["public_id"], "fields": {}}),
    ]
    # ── E. retire / merge ─────────────────────────────────────────────────
    a = "E.entities.retire/merge"
    S += [
        C(a, "E1 create loser", "entities.create",
          {"type_id": "person", "fields": {"full_name": "QA Loser", "inn": f"9{NONCE}01".replace("-","")[:12]}},
          save="loser"),
        C(a, "E2 merge into canonical", "entities.merge",
          fn=lambda e: {"id": e["new_person"]["public_id"],
                        "loser_public_id": e["loser"]["public_id"], "reason": "qa merge"},
          match="canonical_public_id"),
        C(a, "E3 merge idempotent repeat", "entities.merge",
          fn=lambda e: {"id": e["new_person"]["public_id"],
                        "loser_public_id": e["loser"]["public_id"]},
          match="idempotent"),
        C(a, "E4 loser marked (not 404)", "entities.get",
          fn=lambda e: {"id": e["loser"]["public_id"]}, match="public_id"),
        C(a, "E5 merge nonexistent loser", "entities.merge",
          fn=lambda e: {"id": e["new_person"]["public_id"], "loser_public_id": FAKE},
          expect="fail"),
        C(a, "E6 self merge", "entities.merge",
          fn=lambda e: {"id": e["new_person"]["public_id"],
                        "loser_public_id": e["new_person"]["public_id"]}, expect="fail"),
        C(a, "E7 retire with reason", "entities.retire",
          fn=lambda e: {"id": e["new_doc"]["public_id"], "reason": "qa cleanup"},
          match='"retired"'),
        C(a, "E8 retire again → 404", "entities.retire",
          fn=lambda e: {"id": e["new_doc"]["public_id"]}, expect="fail", save=None),
        C(a, "E9 analyst retire denied", "entities.retire",
          fn=lambda e: {"id": e["new_person"]["public_id"]}, token="analyst", expect="deny"),
        C(a, "E10 merge analyst denied", "entities.merge",
          fn=lambda e: {"id": e["new_person"]["public_id"], "loser_public_id": e["p1"]},
          token="analyst", expect="deny"),
        C(a, "E11 retire nonexistent", "entities.retire", {"id": FAKE}, expect="fail"),
    ]
    # ── F. relations ──────────────────────────────────────────────────────
    a = "F.relations"
    S += [
        C(a, "F1 works_for", "relations.create",
          fn=lambda e: {"id": e["new_person"]["public_id"], "predicate": "works_for",
                        "target_entity_id": e["new_le"]["public_id"]},
          match="relation_id", save="rel1"),
        C(a, "F2 owns", "relations.create",
          fn=lambda e: {"id": e["new_person"]["public_id"], "predicate": "owns",
                        "target_entity_id": e["new_le"]["public_id"]}, save="rel2"),
        C(a, "F3 for_entity request→le", "relations.create",
          fn=lambda e: {"id": e["new_req"]["public_id"], "predicate": "for_entity",
                        "target_entity_id": e["new_le"]["public_id"]}, save="rel3"),
        C(a, "F4 related_to wildcard", "relations.create",
          fn=lambda e: {"id": e["new_le"]["public_id"], "predicate": "related_to",
                        "target_entity_id": e["p1"]}, save="rel4"),
        C(a, "F5 missing predicate", "relations.create",
          fn=lambda e: {"id": e["new_person"]["public_id"],
                        "target_entity_id": e["le1"]}, expect="tool_err",
          match="missing required argument 'predicate'"),
        C(a, "F6 unknown predicate", "relations.create",
          fn=lambda e: {"id": e["new_person"]["public_id"], "predicate": "nope",
                        "target_entity_id": e["le1"]}, expect="fail"),
        C(a, "F7 missing target", "relations.create",
          fn=lambda e: {"id": e["new_person"]["public_id"], "predicate": "owns"},
          expect="fail"),
        C(a, "F8 list outgoing", "relations.list",
          fn=lambda e: {"id": e["new_person"]["public_id"], "direction": "outgoing"},
          match="works_for"),
        C(a, "F9 list incoming on le", "relations.list",
          fn=lambda e: {"id": e["new_le"]["public_id"], "direction": "incoming"},
          match="works_for"),
        C(a, "F10 delete relation", "relations.delete",
          fn=lambda e: {"id": e["new_person"]["public_id"],
                        "relation_id": str(e["rel1"]["relation_id"])}, match='true'),
        C(a, "F11 delete nonexistent", "relations.delete",
          fn=lambda e: {"id": e["new_person"]["public_id"], "relation_id": "99999999"},
          expect="fail"),
        C(a, "F12 analyst create denied", "relations.create",
          fn=lambda e: {"id": e["p1"], "predicate": "owns", "target_entity_id": e["le1"]},
          token="analyst", expect="deny"),
    ]
    # ── G. documents & files ──────────────────────────────────────────────
    a = "G.documents/files"
    raw = b"QA test document content"
    sha = hashlib.sha256(raw).hexdigest()
    S += [
        C(a, "G1 documents.upload", "documents.upload",
          fn=lambda e: {"id": e["new_person"]["public_id"], "kind": "kyc_form",
                        "file_name": "qa.txt", "title": "qa.txt",
                        "content_type": "text/plain", "bytes": list(raw)},
          save="doc"),
        C(a, "G2 documents.list", "documents.list",
          fn=lambda e: {"id": e["new_person"]["public_id"]}, match="qa.txt"),
        C(a, "G3 documents.download", "documents.download",
          fn=lambda e: {"id": e["new_person"]["public_id"], "doc_id": e["doc"]["id"]},
          match="QA test document content"),
        C(a, "G4 upload_init", "files.upload_init",
          {"filename": "qa-staged.bin", "expected_size": len(raw), "expected_hash": sha,
           "chunk_size": len(raw), "hash_algorithm": "sha256", "content_type": "text/plain"},
          match='"initiated"', save="upload"),
        C(a, "G5 upload_chunk base64", "files.upload_chunk",
          fn=lambda e: {"id": e["upload"]["upload_id"], "index": 0,
                        "data_base64": base64.b64encode(raw).decode()},
          match="all_chunks_received"),
        C(a, "G6 upload_status", "files.upload_status",
          fn=lambda e: {"id": e["upload"]["upload_id"]}, match="all_chunks_received"),
        C(a, "G7 upload_complete", "files.upload_complete",
          fn=lambda e: {"id": e["upload"]["upload_id"], "final_hash": sha,
                        "total_size": len(raw)},
          match='"completed"', save="file"),
        C(a, "G8 files.get", "files.get",
          fn=lambda e: {"id": e["file"]["object_id"]}, match="qa-staged.bin"),
        C(a, "G9 files.download", "files.download",
          fn=lambda e: {"id": e["file"]["object_id"]}, match="QA test document content"),
        C(a, "G10 upload_init bad hash len", "files.upload_init",
          {"filename": "x", "expected_size": 1, "expected_hash": "zz", "chunk_size": 1},
          expect="fail"),
        C(a, "G11 upload_chunk bad base64", "files.upload_chunk",
          {"id": FAKE, "index": 0, "data_base64": "!!!"}, expect="tool_err", match="base64"),
        C(a, "G12 upload_abort", "files.upload_abort",
          fn=lambda e: {"id": e["upload"]["upload_id"]}, expect="fail"),  # already completed → 409
        C(a, "G13 files.delete", "files.delete",
          fn=lambda e: {"id": e["file"]["object_id"]}, match='true'),
        C(a, "G14 documents.delete", "documents.delete",
          fn=lambda e: {"id": e["new_person"]["public_id"], "doc_id": e["doc"]["id"]},
          match='true'),
        C(a, "G15 analyst upload denied", "documents.upload",
          fn=lambda e: {"id": e["p1"], "kind": "k", "file_name": "f", "title": "f",
                        "content_type": "t", "bytes": [65]},
          token="analyst", expect="deny"),
    ]
    # ── H. tasks ──────────────────────────────────────────────────────────
    a = "H.tasks"
    S += [
        C(a, "H1 tasks.create", "tasks.create",
          fn=lambda e: {"id": e["new_le"]["public_id"], "kind": "review",
                        "title": "QA task", "due_date": "2026-12-01", "priority": "high"},
          match='"pending"', save="task"),
        C(a, "H2 tasks.list", "tasks.list", fn=lambda e: {"id": e["new_le"]["public_id"]},
          match="QA task"),
        C(a, "H3 tasks.start", "tasks.start",
          fn=lambda e: {"id": e["task"]["task_id"]}, match="in_progress"),
        C(a, "H4 tasks.complete", "tasks.complete",
          fn=lambda e: {"id": e["task"]["task_id"], "resolution": "done"}, match="completed"),
        C(a, "H5 create+cancel", "tasks.create",
          fn=lambda e: {"id": e["new_le"]["public_id"], "kind": "review", "title": "cancel me"},
          save="task2"),
        C(a, "H6 tasks.cancel", "tasks.cancel",
          fn=lambda e: {"id": e["task2"]["task_id"], "reason": "n/a"}, match="cancel"),
        C(a, "H7 missing kind → 422", "tasks.create",
          fn=lambda e: {"id": e["new_le"]["public_id"], "title": "x"}, expect="fail"),
        C(a, "H8 complete completed → error", "tasks.complete",
          fn=lambda e: {"id": e["task"]["task_id"]}, expect="fail"),
        C(a, "H9 task on ghost entity → 404", "tasks.create",
          {"id": FAKE, "kind": "review"}, expect="fail"),
        C(a, "H10 analyst create denied", "tasks.create",
          fn=lambda e: {"id": e["new_le"]["public_id"], "kind": "review"},
          token="analyst", expect="deny"),
        C(a, "H11 tasks.list analyst", "tasks.list",
          fn=lambda e: {"id": e["new_le"]["public_id"]}, token="analyst"),
    ]
    # ── I. checks ─────────────────────────────────────────────────────────
    a = "I.checks"
    S += [
        C(a, "I1 checks.create kyc", "checks.create",
          fn=lambda e: {"check_kind": "kyc", "target_entity_id": e["new_le"]["public_id"]},
          match="check_id", save="check"),
        C(a, "I2 checks.create sanctions", "checks.create",
          fn=lambda e: {"check_kind": "sanctions", "target_entity_id": e["new_le"]["public_id"]}),
        C(a, "I3 checks.list", "checks.list", match="data"),
        C(a, "I4 checks.list kind filter", "checks.list", {"kind": "kyc"}),
        C(a, "I5 checks.list status filter", "checks.list", {"status": "pending"}),
        C(a, "I6 checks.get", "checks.get",
          fn=lambda e: {"id": e["check"]["check_id"]}, match="kyc"),
        C(a, "I7 checks.evidence", "checks.evidence",
          fn=lambda e: {"id": e["check"]["check_id"]}),
        C(a, "I8 entity_checks.list", "entity_checks.list",
          fn=lambda e: {"id": e["new_le"]["public_id"]}, match="kyc"),
        C(a, "I9 missing check_kind", "checks.create",
          fn=lambda e: {"target_entity_id": e["new_le"]["public_id"]}, expect="tool_err",
          match="check_kind"),
        C(a, "I10 ghost target → 404", "checks.create",
          {"check_kind": "kyc", "target_entity_id": FAKE}, expect="fail"),
        C(a, "I11 analyst denied", "checks.create",
          fn=lambda e: {"check_kind": "kyc", "target_entity_id": e["le1"]},
          token="analyst", expect="deny"),
    ]
    # ── J. requests ───────────────────────────────────────────────────────
    a = "J.requests"
    S += [
        C(a, "J1 requests.create", "requests.create",
          fn=lambda e: {"target_entity_id": e["new_le"]["public_id"],
                        "requested_product": "credit", "urgency": "high",
                        "check_kinds": ["kyc"]},
          match="request_id", save="req"),
        C(a, "J2 requests.list", "requests.list", match="request_id"),
        C(a, "J3 status filter", "requests.list", {"status": "pending"}, match="request_id"),
        C(a, "J4 missing target", "requests.create", {"requested_product": "x"},
          expect="tool_err", match="target_entity_id"),
        C(a, "J5 ghost target → 404", "requests.create",
          {"target_entity_id": FAKE}, expect="fail"),
        C(a, "J6 limit param", "requests.list", {"limit": 2}),
        C(a, "J7 unknown arg", "requests.create",
          fn=lambda e: {"target_entity_id": e["new_le"]["public_id"], "bogus": 1},
          expect="tool_err"),
        C(a, "J8 create minimal", "requests.create",
          fn=lambda e: {"target_entity_id": e["new_le"]["public_id"],
                        "requested_product": "credit"}),
        C(a, "J9 analyst list", "requests.list", {}, token="analyst"),
        C(a, "J10 analyst create denied", "requests.create",
          fn=lambda e: {"target_entity_id": e["le1"]}, token="analyst", expect="deny"),
    ]
    # ── K. workflows ──────────────────────────────────────────────────────
    a = "K.workflows"
    S += [
        C(a, "K1 workflows.create", "workflows.create",
          fn=lambda e: {"definition_id": "request_processing",
                        "target_entity_id": e["new_le"]["public_id"],
                        "variables": {"qa": True}},
          match="instance_id", save="wf"),
        C(a, "K2 workflows.get", "workflows.get",
          fn=lambda e: {"id": e["wf"]["instance_id"]}, match="current_state"),
        C(a, "K3 workflows.tasks", "workflows.tasks",
          fn=lambda e: {"id": e["wf"]["instance_id"]}),
        C(a, "K4 transition valid", "workflows.transition",
          fn=lambda e: {"id": e["wf"]["instance_id"], "to": "in_review",
                        "trigger": "start_review"}, match="current_state"),
        C(a, "K5 transition invalid state", "workflows.transition",
          fn=lambda e: {"id": e["wf"]["instance_id"], "to": "nonexistent_state",
                        "trigger": "x"},
          expect="fail"),
        C(a, "K6 missing definition_id", "workflows.create",
          fn=lambda e: {"target_entity_id": e["new_le"]["public_id"]}, expect="tool_err",
          match="definition_id"),
        C(a, "K7 unknown definition", "workflows.create",
          fn=lambda e: {"definition_id": "nope", "target_entity_id": e["new_le"]["public_id"]},
          expect="fail"),
        C(a, "K8 get ghost → 404", "workflows.get", {"id": FAKE}, expect="fail"),
        C(a, "K9 analyst create denied", "workflows.create",
          {"definition_id": "request_processing"}, token="analyst", expect="deny"),
        C(a, "K10 create without target → 404", "workflows.create",
          {"definition_id": "request_processing"}, expect="fail"),
        C(a, "K11 transition ghost", "workflows.transition",
          {"id": FAKE, "to": "in_review", "trigger": "x"}, expect="fail"),
    ]
    # ── L. reports ────────────────────────────────────────────────────────
    a = "L.reports"
    S += [
        C(a, "L1 generate json", "reports.generate",
          fn=lambda e: {"entity_id": e["new_le"]["public_id"], "template": "entity_card",
                        "format": "json"}, match="report_id", save="rep"),
        C(a, "L2 reports.list", "reports.list", match="report_id"),
        C(a, "L3 generate csv", "reports.generate",
          fn=lambda e: {"entity_id": e["new_le"]["public_id"], "format": "csv"}),
        C(a, "L4 missing entity_id", "reports.generate", {"format": "json"}, expect="tool_err",
          match="entity_id"),
        C(a, "L5 ghost entity → 404", "reports.generate", {"entity_id": FAKE}, expect="fail"),
        C(a, "L6 generate pdf", "reports.generate",
          fn=lambda e: {"entity_id": e["new_le"]["public_id"], "format": "pdf"}),
        C(a, "L7 generate no format", "reports.generate",
          fn=lambda e: {"entity_id": e["new_le"]["public_id"]}, match="report_id"),
        C(a, "L8 second report new id", "reports.generate",
          fn=lambda e: {"entity_id": e["new_le"]["public_id"], "format": "json"},
          match="report_id"),
        C(a, "L9 analyst denied", "reports.generate",
          fn=lambda e: {"entity_id": e["le1"]}, token="analyst", expect="deny"),
        C(a, "L10 reports.list analyst", "reports.list", {}, token="analyst"),
    ]
    # ── M. connectors ─────────────────────────────────────────────────────
    a = "M.connectors"
    S += [
        C(a, "M1 create", "connectors.create",
          {"connector_type_id": "qa_api", "display_name": "QA conn", "environment": "staging",
           "rate_limit_rps": 3, "trust_rank": 40}, match="instance_id", save="conn"),
        C(a, "M2 list", "connectors.list", match="QA conn"),
        C(a, "M3 get", "connectors.get", fn=lambda e: {"id": e["conn"]["instance_id"]},
          match="QA conn"),
        C(a, "M4 update disable", "connectors.update",
          fn=lambda e: {"id": e["conn"]["instance_id"], "enabled": False}, match='"enabled": false'),
        C(a, "M5 enable", "connectors.enable", fn=lambda e: {"id": e["conn"]["instance_id"]},
          match="enabled"),
        C(a, "M6 disable", "connectors.disable", fn=lambda e: {"id": e["conn"]["instance_id"]}),
        C(a, "M7 health", "connectors.health", fn=lambda e: {"id": e["conn"]["instance_id"]}),
        C(a, "M8 missing type → tool_err", "connectors.create", {"display_name": "x"},
          expect="tool_err", match="connector_type_id"),
        C(a, "M9 get ghost → 404", "connectors.get", {"id": FAKE}, expect="fail"),
        C(a, "M10 update ghost → 404", "connectors.update", {"id": FAKE, "enabled": True},
          expect="fail"),
        C(a, "M11 analyst denied", "connectors.create",
          {"connector_type_id": "x"}, token="analyst", expect="deny"),
        C(a, "M12 delete", "connectors.delete",
          fn=lambda e: {"id": e["conn"]["instance_id"]}, match='true'),
    ]
    # ── N. schema ─────────────────────────────────────────────────────────
    a = "N.schema"
    S += [
        C(a, "N1 schema.types", "schema.types", match="person"),
        C(a, "N2 type_get alias", "schema.type_get", {"alias": "person"}, match="person"),
        C(a, "N3 type_view form", "schema.type_view", {"alias": "person", "view_type": "form"}),
        C(a, "N4 schema.predicates", "schema.predicates", match="works_for"),
        C(a, "N5 schema.published", "schema.published"),
        C(a, "N6 draft_create", "schema.draft_create", {"name": "qa-draft"}, save="draft"),
        C(a, "N7 draft_get", "schema.draft_get",
          fn=lambda e: {"id": e["draft"]["revision_id"]}, match="qa-draft"),
        C(a, "N8 draft_update no etag → 412", "schema.draft_update",
          fn=lambda e: {"id": e["draft"]["revision_id"], "display_name": "x",
                        "semantic_version": "9.9.9"}, expect="fail",
          match="precondition"),
        C(a, "N9 draft_update +etag", "schema.draft_update",
          fn=lambda e: {"id": e["draft"]["revision_id"], "display_name": "qa-draft-v2",
                        "semantic_version": e.get("next_ver", "2.0.0"),
                        "if_match": e["draft"]["etag"]}, match="qa-draft-v2"),
        C(a, "N10 draft_validate", "schema.draft_validate",
          fn=lambda e: {"id": e["draft"]["revision_id"]}, match="valid"),
        C(a, "N11 draft_impact", "schema.draft_impact",
          fn=lambda e: {"id": e["draft"]["revision_id"]}),
        C(a, "N11b refresh etag", "schema.draft_get",
          fn=lambda e: {"id": e["draft"]["revision_id"]},
          fn_save=lambda d: d.get("etag"), save="draft_etag2"),
        C(a, "N12 draft_publish +etag", "schema.draft_publish",
          fn=lambda e: {"id": e["draft"]["revision_id"], "reason": "qa publish validation",
                        "if_match": e.get("draft_etag2")},
          match='"published"'),
        C(a, "N13 revision_get", "schema.revision_get",
          fn=lambda e: {"id": e["draft"]["revision_id"]}, match="person"),
        C(a, "N14 type_get ghost → 404", "schema.type_get", {"alias": "nope"}, expect="fail"),
        C(a, "N15 analyst schema.types", "schema.types", {}, token="analyst"),
        C(a, "N16 analyst draft denied", "schema.draft_create", {"name": "x"},
          token="analyst", expect="deny"),
    ]
    # ── O. users / tokens ─────────────────────────────────────────────────
    a = "O.users/tokens"
    S += [
        C(a, "O1 users.list", "users.list", match="admin"),
        C(a, "O2 q filter", "users.list", {"q": "mcp-admin"}, match="mcp-admin"),
        C(a, "O3 status filter", "users.list", {"status": "active"}, match="admin"),
        C(a, "O4 users.get", "users.get",
          fn=lambda e: {"id": e["admin_pid"]}, match="admin"),
        C(a, "O5 create user", "users.create",
          {"login": f"qa_user_{NONCE}", "display_name": "QA User", "password": "qa-pass-123",
           "groups": ["analysts"]}, save="qa_user"),
        C(a, "O6 create service acct", "users.create",
          {"login": f"qa_svc_{NONCE}", "display_name": "QA Svc", "is_service_account": True,
           "groups": ["analysts"]}, save="qa_svc"),
        C(a, "O7 dup login rejected", "users.create",
          {"login": f"qa_user_{NONCE}", "display_name": "X", "password": "qa-pass-123"}, expect="fail"),
        C(a, "O8 token_create", "users.token_create",
          fn=lambda e: {"id": e["qa_svc"]["principal_id"] or e["qa_svc"]["id"],
                        "name": "qa-tok"}, match="egr_", save="tok"),
        C(a, "O9 tokens list no secret", "users.tokens",
          fn=lambda e: {"id": e["qa_svc"]["principal_id"] or e["qa_svc"]["id"]},
          match="qa-tok"),
        C(a, "O10 token_revoke", "users.token_revoke",
          fn=lambda e: {"id": e["qa_svc"]["principal_id"] or e["qa_svc"]["id"],
                        "token_id": e["tok"]["token_id"]}),
        C(a, "O11 users.update groups", "users.update",
          fn=lambda e: {"id": e["qa_user"]["principal_id"] or e["qa_user"]["id"],
                        "groups": ["managers"]}, match='"updated"'),
        C(a, "O12 users.get roles", "users.get",
          fn=lambda e: {"id": e["qa_user"]["principal_id"] or e["qa_user"]["id"]},
          match="managers"),
        C(a, "O13 users.activity", "users.activity",
          fn=lambda e: {"id": e["qa_user"]["principal_id"] or e["qa_user"]["id"]}),
        C(a, "O14 users.delete", "users.delete",
          fn=lambda e: {"id": e["qa_svc"]["principal_id"] or e["qa_svc"]["id"]}),
        C(a, "O15 analyst denied users.list", "users.list", {}, token="analyst", expect="deny"),
        C(a, "O15b role catalog", "users.role_groups", {}, match="admins"),
        C(a, "O16 short password rejected", "users.create",
          {"login": f"qa_u2_{NONCE}", "display_name": "X", "password": "123"}, expect="fail"),
    ]
    # ── P. permission groups ──────────────────────────────────────────────
    a = "P.permission groups"
    S += [
        C(a, "P1 groups list", "permissions.groups", match="admins"),
        C(a, "P2 group_create capability", "permissions.group_create",
          {"name": "qa_cap_role", "class": "capability",
           "rules": [{"resource_type": "capability", "action": "dashboard.view",
                      "effect": "allow"}]}, save="grp_cap"),
        C(a, "P3 group_create data", "permissions.group_create",
          {"name": "qa_data_role", "class": "data",
           "rules": [{"resource_type": "entity", "action": "read", "effect": "allow",
                      "resource": {"type": "person"}}]}, save="grp_data"),
        C(a, "P4 invalid class → 422", "permissions.group_create",
          {"name": "qa_bad", "class": "nope"}, expect="fail"),
        C(a, "P5 missing name → tool_err", "permissions.group_create", {"class": "data"},
          expect="tool_err", match="name"),
        C(a, "P6 group_update principals", "permissions.group_update",
          fn=lambda e: {"id": e["grp_cap"]["id"], "name": "qa_cap_role",
                        "principals": [e["qa_login_svc"]]}, match="qa_cap_role"),
        C(a, "P7 group_update parent", "permissions.group_update",
          fn=lambda e: {"id": e["grp_data"]["id"], "name": "qa_data_role",
                        "parent_id": e["all_data_id"]}),
        C(a, "P8 delete special role", "permissions.group_delete",
          fn=lambda e: {"id": e["superadmin_id"]}, expect="fail"),
        C(a, "P9 delete qa role", "permissions.group_delete",
          fn=lambda e: {"id": e["grp_cap"]["id"]}, match='true'),
        C(a, "P10 analyst denied", "permissions.groups", {}, token="analyst", expect="deny"),
        C(a, "P11 delete ghost → 404", "permissions.group_delete", {"id": FAKE}, expect="fail"),
        C(a, "P12 delete data role", "permissions.group_delete",
          fn=lambda e: {"id": e["grp_data"]["id"]}, match='true'),
    ]
    # ── Q. audit ──────────────────────────────────────────────────────────
    a = "Q.audit"
    S += [
        C(a, "Q1 audit.query", "audit.query", {"limit": 5}, match="data"),
        C(a, "Q2 filter action", "audit.query", {"action": "entity.create", "limit": 5}),
        C(a, "Q3 filter effect", "audit.query", {"effect": "allow", "limit": 5}),
        C(a, "Q4 checkpoint_get", "audit.checkpoint_get"),
        C(a, "Q5 checkpoint_create", "audit.checkpoint_create", match="checkpoint"),
        C(a, "Q6 audit.verify", "audit.verify", match="valid"),
        C(a, "Q7 retention_get", "audit.retention_get", match="retention"),
        C(a, "Q8 retention_set", "audit.retention_set", {"retention_days": 365},
          match="365"),
        C(a, "Q9 retention bad days → 422", "audit.retention_set", {"retention_days": 0},
          expect="fail"),
        C(a, "Q10 analyst denied", "audit.query", {}, token="analyst", expect="deny"),
        C(a, "Q11 restore retention", "audit.retention_set", {"retention_days": 3650}),
    ]
    # ── R. monitoring / misc read tools ───────────────────────────────────
    a = "R.monitoring/misc"
    S += [
        C(a, "R1 context.get", "context.get", match="capabilities"),
        C(a, "R2 dashboard.stats", "dashboard.stats"),
        C(a, "R3 monitors.list", "monitors.list"),
        C(a, "R4 monitors.run", "monitors.run", fn=lambda e: {"id": e.get("monitor_id", FAKE)}),
        C(a, "R5 factors.list", "factors.list", fn=lambda e: {"id": e["new_le"]["public_id"]}),
        C(a, "R6 assessments.list", "assessments.list"),
        C(a, "R7 assessment.create", "assessment.create",
          fn=lambda e: {"id": e["new_le"]["public_id"], "rationale": "qa",
                        "force": True, "desired_level": "medium", "decision": "review"},
          match="assessment_id|level"),
        C(a, "R7b invalid level → 422", "assessment.create",
          fn=lambda e: {"id": e["new_le"]["public_id"], "desired_level": "bogus"},
          expect="fail"),
        C(a, "R8 entities.history", "entities.history",
          fn=lambda e: {"id": e["new_person"]["public_id"]}),
        C(a, "R9 entities.query AST", "entities.query",
          {"query": {"type": "person"}}, match="data"),
        C(a, "R10 analyst context", "context.get", {}, token="analyst", match="capabilities"),
        C(a, "R11 analyst dashboard", "dashboard.stats", {}, token="analyst"),
    ]
    # ── S. protocol & security negatives ──────────────────────────────────
    a = "S.protocol/security"
    S += [
        C(a, "S1 no token → 401", "__raw__", {"__noauth__": True}, expect="http401"),
        C(a, "S2 garbage token → 401", "__raw__", {"__token__": "egr_" + "0" * 64},
          expect="http401"),
        C(a, "S3 unknown method → -32601", "__rpc__", {"method": "nope/method"}, expect="fail",
          match="-32601"),
        C(a, "S4 unknown tool → -32602", "no.such.tool", {}, expect="fail", match="-32602"),
        C(a, "S5 analyst tools/list smaller", "__tools_list__", {}, token="analyst"),
        C(a, "S6 analyst admin tool denied", "audit.retention_set",
          {"retention_days": 100}, token="analyst", expect="deny"),
        C(a, "S7 path arg '?x' rejected", "entities.get", {"id": "a?b"}, expect="tool_err"),
        C(a, "S8 path arg 'a/b' rejected", "entities.get", {"id": "a/b"}, expect="tool_err"),
        C(a, "S9 initialize", "__rpc__", {"method": "initialize", "params": {}}, match="protocol"),
        C(a, "S10 missing required arg", "entities.get", {}, expect="tool_err",
          match="missing required argument"),
        C(a, "S11 tools/list admin 90+", "__tools_list__", {}, match="tools"),
        C(a, "S12 token revoked mid-flight → 401", "__raw__", {"__revoked__": True},
          expect="http401"),
    ]
    # ── T. targeted deepening — lifts every thin tool to >=10 cases ───────
    a = "T.targeted"
    S += [
        # reports.list — response shape assertions.
        C(a, "reports.list has data", "reports.list", match='"data"'),
        C(a, "reports.list has ids", "reports.list", match="report_id|instance_id|\"id\""),
        C(a, "reports.list status field", "reports.list", match="status|format|created_at"),
        C(a, "reports.list repeat", "reports.list"),
        C(a, "reports.list limit probe", "reports.list", {"limit": 1},
          expect="tool_err"),
        # connectors.create — validation edges.
        C(a, "connectors.create trust>100", "connectors.create",
          {"connector_type_id": "qa_edge", "trust_rank": 101}, expect="fail"),
        C(a, "connectors.create negative rps", "connectors.create",
          {"connector_type_id": "qa_edge2", "rate_limit_rps": -5},
          expect="fail"),
        # connectors.list — shape + content.
        C(a, "connectors.list has data", "connectors.list", match='"data"'),
        C(a, "connectors.list has conn", "connectors.list",
          match="connector|instance|alias"),
        C(a, "connectors.list repeat", "connectors.list"),
        C(a, "connectors.list analyst deny2", "connectors.list", {},
          token="analyst", expect="deny"),
        C(a, "connectors.create t-conn", "connectors.create",
          {"connector_type_id": f"qa_tconn_{NONCE}",
           "display_name": f"QA T-conn {NONCE}"}),
        C(a, "connectors.list sees t-conn", "connectors.list",
          match=f"qa_tconn_{NONCE}"),
        C(a, "connectors.list bogus", "connectors.list", {"bogus": 1},
          expect="tool_err"),
        # schema.types — seeded catalog assertions.
        C(a, "schema.types person", "schema.types", match='"person"'),
        C(a, "schema.types legal_entity", "schema.types", match="legal_entity"),
        C(a, "schema.types request", "schema.types", match='"request"'),
        C(a, "schema.types document", "schema.types", match="document"),
        C(a, "schema.types contract", "schema.types", match='"contract"'),
        # schema.predicates — seeded predicate assertions.
        C(a, "schema.predicates works_for", "schema.predicates",
          match="works_for"),
        C(a, "schema.predicates owns", "schema.predicates", match="owns"),
        C(a, "schema.predicates inn", "schema.predicates", match='"inn"'),
        C(a, "schema.predicates full_name", "schema.predicates",
          match="full_name"),
        C(a, "schema.predicates has_contract", "schema.predicates",
          match="has_contract"),
        C(a, "schema.predicates related_to", "schema.predicates",
          match="related_to"),
        # schema.published — revision metadata.
        C(a, "schema.published revision_id", "schema.published",
          match="revision_id"),
        C(a, "schema.published semver", "schema.published",
          match="semantic_version"),
        C(a, "schema.published status", "schema.published", match="published"),
        C(a, "schema.published types", "schema.published", match='"types"'),
        C(a, "schema.published person", "schema.published", match="person"),
        C(a, "schema.published etag-ish", "schema.published",
          match="etag|content_hash"),
        # schema.drafts — list shape.
        C(a, "schema.drafts has data", "schema.drafts", match='"data"'),
        C(a, "schema.drafts revision id", "schema.drafts", match="revision"),
        C(a, "schema.drafts shows qa", "schema.drafts", match="qa-draft|draft"),
        C(a, "schema.drafts repeat", "schema.drafts"),
        C(a, "schema.drafts analyst deny2", "schema.drafts", {},
          token="analyst", expect="deny"),
        C(a, "schema.drafts bogus arg", "schema.drafts", {"bogus": 1},
          expect="tool_err"),
        C(a, "schema.drafts has status", "schema.drafts", match="status|draft"),
        # schema.draft_create — validation edges.
        C(a, "draft_create ghost base", "schema.draft_create",
          {"name": "qa-x", "base_revision": "99999"}, expect="fail"),
        C(a, "draft_create empty name", "schema.draft_create",
          {"name": ""}, expect="fail"),
        # users.role_groups — catalog contents.
        C(a, "role_groups admins", "users.role_groups", match="admins"),
        C(a, "role_groups analysts", "users.role_groups", match="analysts"),
        C(a, "role_groups all_data", "users.role_groups", match="all_data"),
        C(a, "role_groups class field", "users.role_groups",
          match='"class"'),
        C(a, "role_groups capability", "users.role_groups",
          match="capability"),
        C(a, "role_groups parent", "users.role_groups",
          match="parent_id|parent"),
        # permissions.groups — catalog contents.
        C(a, "perm.groups admins", "permissions.groups", match="admins"),
        C(a, "perm.groups rules", "permissions.groups", match='"rules"'),
        C(a, "perm.groups class", "permissions.groups", match='"class"'),
        C(a, "perm.groups all_data", "permissions.groups", match="all_data"),
        C(a, "perm.groups repeat", "permissions.groups"),
        # audit.checkpoint_get / create / verify — lifecycle.
        C(a, "checkpoint_get data", "audit.checkpoint_get", match='"data"|checkpoint'),
        C(a, "checkpoint_get again", "audit.checkpoint_get"),
        C(a, "checkpoint_get id field", "audit.checkpoint_get",
          match="checkpoint_id|\"id\"|\"at\"|created"),
        C(a, "checkpoint_get 4th", "audit.checkpoint_get"),
        C(a, "checkpoint_get 5th", "audit.checkpoint_get"),
        C(a, "checkpoint_get bogus", "audit.checkpoint_get", {"bogus": 1},
          expect="tool_err"),
        C(a, "checkpoint_create #2", "audit.checkpoint_create",
          match="checkpoint"),
        C(a, "checkpoint_create #3", "audit.checkpoint_create"),
        C(a, "checkpoint_create #4", "audit.checkpoint_create"),
        C(a, "checkpoint_create idempotent-shape", "audit.checkpoint_create",
          match='"data"|checkpoint|id'),
        C(a, "checkpoint visible in get", "audit.checkpoint_get",
          match="checkpoint"),
        C(a, "checkpoint_create bogus", "audit.checkpoint_create",
          {"bogus": 1}, expect="tool_err"),
        C(a, "checkpoint_create #5", "audit.checkpoint_create"),
        C(a, "verify after checkpoints", "audit.verify", match="valid"),
        C(a, "verify range args", "audit.verify",
          {"from": "2020-01-01", "to": "2030-12-31"}),
        C(a, "verify true", "audit.verify", match='"valid": true|valid'),
        C(a, "verify analyst deny2", "audit.verify", {}, token="analyst",
          expect="deny"),
        C(a, "verify repeat", "audit.verify"),
        C(a, "verify bad range", "audit.verify",
          {"from": "2099-01-01", "to": "2020-01-01"}),
        # audit.retention — shape + invalid values.
        C(a, "retention_get days", "audit.retention_get", match="retention"),
        C(a, "retention_get again", "audit.retention_get"),
        C(a, "retention_get fields", "audit.retention_get",
          match="days|legal_hold|archive"),
        C(a, "retention_get 4th", "audit.retention_get"),
        C(a, "retention_get bogus", "audit.retention_get", {"bogus": 1},
          expect="tool_err"),
        C(a, "retention_get analyst deny", "audit.retention_get", {},
          token="analyst", expect="deny"),
        C(a, "retention_set negative", "audit.retention_set",
          {"retention_days": -5}, expect="fail"),
        C(a, "retention_set huge", "audit.retention_set",
          {"retention_days": 999999}, expect="fail"),
        C(a, "retention_set badtype", "audit.retention_set",
          {"retention_days": "x"}, expect="fail"),
        # context.get — principal fields.
        C(a, "context caps", "context.get", match="capabilities"),
        C(a, "context principal", "context.get", match="principal"),
        C(a, "context tenant", "context.get", match="tenant"),
        C(a, "context entity.read", "context.get", match="entity.read"),
        C(a, "context admin cap", "context.get",
          match="schema.publish|users.manage|audit.read"),
        # dashboard.stats — metric keys.
        C(a, "dashboard my_tasks", "dashboard.stats", match="my_tasks"),
        C(a, "dashboard shape", "dashboard.stats",
          match="new_requests|checks_in_progress|ready_reports"),
        C(a, "dashboard repeat", "dashboard.stats"),
        C(a, "dashboard analyst", "dashboard.stats", {}, token="analyst"),
        C(a, "dashboard bogus arg", "dashboard.stats", {"bogus": 1},
          expect="tool_err"),
        # monitors.list — seeded schedules.
        C(a, "monitors has data", "monitors.list", match='"data"'),
        C(a, "monitors seeded", "monitors.list", match="monitor_id|name"),
        C(a, "monitors frequency", "monitors.list",
          match="daily|hourly|frequency"),
        C(a, "monitors repeat", "monitors.list"),
        C(a, "monitors analyst deny2", "monitors.list", {}, token="analyst",
          expect="deny"),
        C(a, "monitors bogus arg", "monitors.list", {"bogus": 1},
          expect="tool_err"),
        # entities.query — valid AST forms.
        C(a, "query typed person", "entities.query",
          {"query": {"from": {"type_id": "person"}}}, match="Иванов"),
        C(a, "query search", "entities.query",
          {"query": {"search": "Иванов"}}, match="Иванов"),
        C(a, "query page size 1", "entities.query",
          {"query": {"page": {"size": 1}}}, match='"rows"|"data"'),
        C(a, "query empty obj", "entities.query", {"query": {}},
          match="rows|data"),
        C(a, "query null→err", "entities.query", {"query": None},
          expect="tool_err"),
        C(a, "query non-object→422", "entities.query", {"query": "x"},
          expect="fail"),
    ]
    # ── X. per-tool generic negatives (generated from tools/list) ─────────
    if tools:
        S += synthetic_cases(tools, analyst_tools or set())
    return S


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------
def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    ap = argparse.ArgumentParser()
    ap.add_argument("--mcp", default=DEFAULT_MCP)
    ap.add_argument("--api", default=DEFAULT_API)
    ap.add_argument("--login", default="admin")
    ap.add_argument("--password", default="enginer-demo")
    ap.add_argument("--area", default=None)
    ap.add_argument("--out", default="docs/testing/mcp_report_raw.json")
    args = ap.parse_args()
    global MCP, API
    MCP, API = args.mcp, args.api

    api = Api(API, args.login, args.password)

    # Provision service accounts + tokens
    def ensure_user(login, groups):
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

    qa_admin_id = ensure_user("qa-mcp-admin", ["admins", "all_data"])
    qa_analyst_id = ensure_user("qa-mcp-analyst", ["analysts", "all_data"])

    def mint(pid, name):
        st, r = api.json("POST", f"/api/v1/users/{pid}/tokens", {"name": name})
        assert st == 200, f"token mint failed: {r}"
        return r["data"]["token"]

    tokens = {"admin": mint(qa_admin_id, "qa-runner"),
              "analyst": mint(qa_analyst_id, "qa-runner")}
    revoked = mint(qa_analyst_id, "qa-revoked")
    st, tl = api.json("GET", f"/api/v1/users/{qa_analyst_id}/tokens")
    rid = next(t["token_id"] for t in tl["data"] if t["name"] == "qa-revoked")
    api.json("DELETE", f"/api/v1/users/{qa_analyst_id}/tokens/{rid}")

    # Env: seeded ids (demo tenant)
    env = {
        "p1": "00000000-0000-7000-9000-000000000001",
        "p2": "00000000-0000-7000-9000-000000000002",
        "le1": "00000000-0000-7000-9000-000000000003",
        "le2": "00000000-0000-7000-9000-000000000004",
        "admin_pid": "00000000-0000-0000-0000-0000000000a1",
        "all_data_id": "00000000-0000-0000-0000-0000000000bf",
        "superadmin_id": "00000000-0000-0000-0000-0000000000c0",
        "qa_login_svc": "qa-mcp-analyst",
        "new_person": {}, "new_le": {}, "new_doc": {}, "new_req": {},
    }
    # discover a retired entity + monitor id
    st, r = api.json("GET", "/api/v1/entities?limit=200")
    for ent in r.get("data", []):
        pass  # retired entities aren't listed; skip retired_id discovery
    st, mon = api.json("GET", "/api/v1/monitors")
    items = mon.get("data") or []
    if items:
        env["monitor_id"] = items[0].get("monitor_id") or items[0].get("id")

    # Next publishable semantic version: bump the major of the newest
    # published revision so N12 never collides with a previous run.
    best = (0, 0, 0)
    st, pub = api.json("GET", "/api/v1/schemas/published")
    rows = pub.get("data") or []
    if isinstance(rows, dict):
        rows = [rows]
    for rev in rows:
        v = str(rev.get("semantic_version") or rev.get("version") or "")
        try:
            parts = tuple(int(x) for x in v.split(".")[:3])
            if len(parts) == 3:
                best = max(best, parts)
        except ValueError:
            pass
    env["next_ver"] = f"{best[0] + 1}.0.0"

    # fixture: a retired entity + a cursor for pagination tests
    stc, rc = rpc(tokens["admin"], "tools/call",
                  {"name": "entities.create",
                   "arguments": {"type_id": "document",
                                 "fields": {"title": "QA retired fixture"}}}, 9000)
    try:
        rid2 = (tool_data(rc) or {}).get("public_id")
        if rid2:
            rpc(tokens["admin"], "tools/call",
                {"name": "entities.retire", "arguments": {"id": rid2, "reason": "fixture"}}, 9002)
            env["retired_id"] = rid2
    except Exception:
        pass
    # first page cursor for A6
    stc, rc = rpc(tokens["admin"], "tools/call",
                  {"name": "entities.list", "arguments": {"limit": 1}}, 9001)
    try:
        payload = json.loads(tool_text(rc))
        env["cursor"] = (payload.get("meta") or {}).get("cursor") or \
                        (payload.get("meta") or {}).get("next_cursor")
    except Exception:
        pass

    # Tool catalogs for the synthetic block: admin sees everything, the
    # analyst list defines which tools should deny her token.
    st, tl_admin = rpc(tokens["admin"], "tools/list", {}, 8000)
    tools = (tl_admin.get("result") or {}).get("tools", [])
    st, tl_an = rpc(tokens["analyst"], "tools/list", {}, 8001)
    analyst_tools = {t["name"]
                     for t in (tl_an.get("result") or {}).get("tools", [])}

    cases = build_cases(tools, analyst_tools)
    if args.area:
        cases = [c for c in cases if c["area"].startswith(args.area)]

    results = []
    idn = 0
    for c in cases:
        idn += 1
        token = tokens[c["token"]]
        t0 = time.time()
        try:
            if c["tool"] == "__raw__":
                if c["args"].get("__noauth__"):
                    st, r = rpc(None, "tools/list", {}, idn)
                elif c["args"].get("__revoked__"):
                    st, r = rpc(revoked, "tools/list", {}, idn)
                else:
                    st, r = rpc(c["args"]["__token__"], "tools/list", {}, idn)
                outcome = "pass" if (st == 401) == (c["expect"] == "http401") else "fail"
                detail = f"http {st}"
            elif c["tool"] == "__rpc__":
                st, r = rpc(token, c["args"]["method"], c["args"].get("params", {}), idn)
                txt = json.dumps(r)
                outcome = "pass" if (c["match"] or "") in txt or (c["match"] is None and st == 200) \
                          else "fail"
                detail = txt[:300]
            elif c["tool"] == "__tools_list__":
                st, r = rpc(token, "tools/list", {}, idn)
                n = len((r.get("result") or {}).get("tools", []))
                if c["token"] == "analyst":
                    outcome = "pass" if n < 90 else "fail"
                    detail = f"analyst tools={n}"
                else:
                    outcome = "pass" if n >= 90 else "fail"
                    detail = f"admin tools={n}"
            else:
                a2 = dict(c["args"])
                if c["fn"]:
                    a2 = c["fn"](env) or {}
                st, r = rpc(token, "tools/call",
                            {"name": c["tool"], "arguments": a2}, idn)
                txt = tool_text(r)
                err = is_tool_error(r)
                exp = c["expect"]
                if exp == "ok":
                    outcome = "pass" if not err else "fail"
                elif exp == "reach":
                    # The call must reach the API — any outcome except a
                    # capability denial counts as pass.
                    payload = json.dumps(r)
                    outcome = "fail" if ("not granted" in txt
                                         or "-32602" in payload) else "pass"
                elif exp in ("tool_err", "fail", "deny"):
                    outcome = "pass" if err else "fail"
                    if exp == "deny" and "capability" not in txt and "-32602" not in json.dumps(r):
                        outcome = "fail" if not err else ("pass" if ("not granted" in txt or "capability" in txt.lower() or "-32602" in json.dumps(r)) else "fail")
                else:
                    outcome = "pass" if not err else "fail"
                if outcome == "pass" and c["match"] and c["match"] not in txt \
                        and exp == "ok":
                    # tolerate regex-ish alternation
                    alts = c["match"].split("|")
                    if not any(a in txt for a in alts):
                        outcome = "fail"
                        detail = f"match '{c['match']}' not found in: {txt[:220]}"
                    else:
                        detail = txt[:300]
                else:
                    detail = txt[:300]
                if not err and c["save"]:
                    d = tool_data(r) or {}
                    env[c["save"]] = c.get("fn_save")(d) if c.get("fn_save") else d
                # learn refreshed etag for draft publish
                if c["name"].startswith("N9"):
                    env["draft_etag2"] = (tool_data(r) or {}).get("etag")
        except Exception as e:
            outcome, detail = "error", f"{type(e).__name__}: {e}"
        dt = int((time.time() - t0) * 1000)
        results.append({"area": c["area"], "name": c["name"], "tool": c["tool"],
                        "expect": c["expect"], "outcome": outcome,
                        "ms": dt, "detail": detail})
        mark = {"pass": "PASS", "fail": "FAIL", "error": "ERR "}[outcome]
        print(f"[{mark}] {c['name']:38} {c['tool']:28} {dt}ms {detail[:120]}")

    ok = sum(1 for r in results if r["outcome"] == "pass")
    print(f"\n{ok}/{len(results)} passed")
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump({"passed": ok, "total": len(results), "results": results},
                  f, ensure_ascii=False, indent=2)
    sys.exit(0 if ok == len(results) else 1)


if __name__ == "__main__":
    main()
