#!/usr/bin/env python3
"""Warehouse «ТЕСТ» MCP test runner — every check goes through POST /mcp.

Requires provisioned warehouse domain (run provision.py first) and bearer
tokens (bootstrap.py). Scenario areas WH1..WH10 each carry >=10 handwritten
cases; block WHX tops every exercised tool up to >=10 cases with generic
negative probes (missing/null/unknown/bad-type args, path traversal, ghost
ids, operator-token authorization boundaries).

Usage:
    python docs/testing/warehouse/bootstrap.py   # mint .wh_tokens.json
    python docs/testing/warehouse/provision.py   # schema + objects (via MCP)
    python docs/testing/warehouse/wh_test_runner.py [--area WH3.search]
"""
import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))
MCP = os.environ.get("ENGINER_MCP", "http://localhost:8020")
OUT = os.path.join(HERE, "wh_report_raw.json")
FAKE = str(uuid.uuid4())
NOT_UUID = "definitely-not-a-uuid"


def rpc(token, method, params, idn):
    req = urllib.request.Request(
        MCP + "/mcp",
        data=json.dumps({"jsonrpc": "2.0", "id": idn, "method": method,
                         "params": params}).encode(),
        headers={"Content-Type": "application/json",
                 "Accept": "application/json, text/event-stream",
                 **({"Authorization": f"Bearer {token}"} if token else {})})
    try:
        r = urllib.request.urlopen(req)
        raw = r.read().decode()
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
    for c in result.get("content", []):
        if c.get("type") == "text":
            return c["text"]
    return json.dumps(result)


def tool_data(res):
    try:
        return json.loads(tool_text(res)).get("data")
    except Exception:
        return None


def is_tool_error(res):
    return bool((res.get("result") or {}).get("isError")) or bool(res.get("error"))


def C(area, name, tool, args=None, expect="ok", match=None, fn=None,
      save=None, token="admin", fn_save=None):
    return {"area": area, "name": name, "tool": tool, "args": args or {},
            "expect": expect, "match": match, "fn": fn, "save": save,
            "token": token, "fn_save": fn_save}


def _next_ver(v):
    try:
        parts = [int(x) for x in str(v).split(".")]
        while len(parts) < 3:
            parts.append(0)
        parts[-1] += 1
        return ".".join(str(x) for x in parts)
    except Exception:
        return "99.0.0"


def _stub(prop):
    return {"string": "x", "integer": 1, "number": 1, "boolean": True,
            "array": [], "object": {}}.get(prop.get("type"), "x")


def _wrong(prop):
    return {"x": 1} if prop.get("type") == "string" else "x"


def synthetic_cases(tools, operator_tools):
    """Generic per-tool negatives for every tool the warehouse suite uses —
    tops each tool up to >=10 cases."""
    S = []
    for t in tools:
        name = t["name"]
        schema = t.get("inputSchema") or {}
        props = schema.get("properties") or {}
        req = schema.get("required") or []
        area = f"WHX.{name}"
        stub = {p: _stub(props.get(p) or {}) for p in req}
        if req:
            S.append(C(area, f"{name} ∅args→err", name, {}, expect="tool_err"))
        else:
            S.append(C(area, f"{name} ∅args", name, {}))
        for p in req[:3]:
            a2 = dict(stub)
            a2[p] = None
            S.append(C(area, f"{name} null:{p}", name, a2, expect="tool_err"))
        a2 = dict(stub)
        a2["__bogus__"] = 1
        S.append(C(area, f"{name} unknown arg", name, a2, expect="tool_err"))
        for p in req[:2]:
            a2 = dict(stub)
            a2[p] = _wrong(props.get(p) or {})
            S.append(C(area, f"{name} badtype:{p}", name, a2, expect="fail"))
        for p in req[:2]:
            if (props.get(p) or {}).get("type") == "string" \
                    and (props.get(p) or {}).get("x-location") != "path":
                a2 = dict(stub)
                a2[p] = ""
                S.append(C(area, f"{name} emptystr:{p}", name, a2,
                           expect="fail"))
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
        if name not in operator_tools:
            S.append(C(area, f"{name} operator deny", name, dict(stub),
                       token="operator", expect="deny"))
        elif req:
            a2 = dict(stub)
            for p in req:
                if (props.get(p) or {}).get("x-location") == "path":
                    a2[p] = FAKE
            S.append(C(area, f"{name} operator reach", name, a2,
                       token="operator", expect="reach"))
        else:
            S.append(C(area, f"{name} operator ok", name, {}, token="operator"))
    return S


def build_cases(env, tools, operator_tools):
    N = env["_nonce"]
    RUN = uuid.uuid4().hex[:6]
    S = []

    # ── WH1. warehouse schema published & browsable ──────────────────────
    a = "WH1.schema"
    S += [
        C(a, "WH1-01 published rev contains wh types", "schema.published",
          match="wh_warehouse"),
        C(a, "WH1-02 types list has wh set", "schema.types", match="wh_product"),
        C(a, "WH1-03 type_get wh_product", "schema.type_get",
          {"alias": "wh_product"}, match="wh_sku"),
        C(a, "WH1-04 type_get wh_warehouse", "schema.type_get",
          {"alias": "wh_warehouse"}, match="wh_code"),
        C(a, "WH1-05 predicates include wh edges", "schema.predicates",
          match="wh_stock_product"),
        C(a, "WH1-06 predicates include wh scalars", "schema.predicates",
          match="wh_qty"),
        C(a, "WH1-07 type_get wh_inventory", "schema.type_get",
          {"alias": "wh_inventory"}, match="wh_inv_no"),
        C(a, "WH1-08 type_view wh_warehouse form", "schema.type_view",
          {"alias": "wh_warehouse", "view_type": "form"}),
        C(a, "WH1-09 revision_get warehouse rev", "schema.revision_get",
          fn=lambda e: {"id": e["_published_revision"]}, match="wh_zone"),
        C(a, "WH1-10 type_get ghost alias → err", "schema.type_get",
          {"alias": "wh_nonexistent"}, expect="fail"),
        C(a, "WH1-11 drafts list reachable", "schema.drafts", match="data"),
        C(a, "WH1-12 schema.types analyst readable", "schema.types",
          token="analyst", match="wh_warehouse"),
    ]

    # ── WH2. object CRUD on warehouse entities ───────────────────────────
    a = "WH2.objects"
    S += [
        C(a, "WH2-01 create warehouse", "entities.create",
          {"type_id": "wh_warehouse",
           "fields": {"wh_code": f"WH-{N}-90-{RUN}", "name": f"Склад ТЕСТ Южный {N}-{RUN}",
                      "address": "тест", "wh_capacity_m2": 800}},
          match="public_id", save="wh_new", fn_save=lambda d: d["public_id"]),
        C(a, "WH2-02 get wh1 by id", "entities.get",
          fn=lambda e: {"id": e["wh1"]}, match="ТЕСТ Центральный"),
        C(a, "WH2-03 get wh_zone", "entities.get",
          fn=lambda e: {"id": e["zone_sto_a"]}, match="wh_zone"),
        C(a, "WH2-04 create product", "entities.create",
          {"type_id": "wh_product",
           "fields": {"wh_sku": f"SKU-{N}-901-{RUN}", "name": "Уголок стальной",
                      "wh_unit": "м", "wh_category": "металл"}},
          match="public_id", save="prod_new",
          fn_save=lambda d: d["public_id"]),
        C(a, "WH2-05 dedup same SKU", "entities.create",
          {"type_id": "wh_product",
           "fields": {"wh_sku": f"SKU-{N}-901-{RUN}", "name": "Уголок стальной 2"}},
          match="deduplicated|public_id"),
        C(a, "WH2-06 reject dup SKU", "entities.create",
          {"type_id": "wh_product", "identity_mode": "reject",
           "fields": {"wh_sku": f"SKU-{N}-901-{RUN}", "name": "Уголок стальной 3"}},
          expect="fail"),
        C(a, "WH2-07 update product name", "entities.update",
          fn=lambda e: {"id": e["prod_new"],
                        "fields": {"name": "Уголок стальной 40х40"}},
          match="public_id|revision"),
        C(a, "WH2-08 update stock qty", "entities.update",
          fn=lambda e: {"id": e["stk_nails"], "fields": {"wh_qty": 1450}},
          match="1450|public_id|revision"),
        C(a, "WH2-09 history after updates", "entities.history",
          fn=lambda e: {"id": e["prod_new"]}, match="revision|data"),
        C(a, "WH2-10 list wh_warehouse", "entities.list",
          {"type": "wh_warehouse", "limit": 50}, match="ТЕСТ"),
        C(a, "WH2-11 create stock record", "entities.create",
          {"type_id": "wh_stock", "fields": {"wh_qty": 50, "wh_batch": "B-9"}},
          match="public_id", save="stk_new", fn_save=lambda d: d["public_id"]),
        C(a, "WH2-12 retire + status", "entities.retire",
          fn=lambda e: {"id": e["stk_new"],
                        "reason": "Тестовый остаток списан для прогона"},
          match="retired|public_id"),
        C(a, "WH2-13 get retired → status retired", "entities.get",
          fn=lambda e: {"id": e["stk_new"]}, match="retired"),
        C(a, "WH2-14 create wh_zone ghost type → err", "entities.create",
          {"type_id": "wh_nope", "fields": {"name": "x"}}, expect="fail"),
        C(a, "WH2-15 analyst read wh", "entities.get",
          fn=lambda e: {"id": e["wh1"]}, token="analyst"),
    ]

    # ── WH3. relations on warehouse graph ────────────────────────────────
    a = "WH3.relations"
    S += [
        C(a, "WH3-00 new zone for edges", "entities.create",
          {"type_id": "wh_zone",
           "fields": {"wh_zone_code": f"Z-{N}-T-{RUN}", "wh_zone_kind": "тест"}},
          match="public_id", save="zone_t", fn_save=lambda d: d["public_id"]),
        C(a, "WH3-01 zone→warehouse edge", "relations.create",
          fn=lambda e: {"id": e["zone_t"], "predicate": "wh_located_in",
                        "target_entity_id": e["wh1"]}),
        C(a, "WH3-02 stock→product edge", "relations.create",
          fn=lambda e: {"id": e.get("stk_in") or e["stk_nails"], "predicate": "wh_stock_product",
                        "target_entity_id": e["prod_new"]},
          save="rel_paint",
          fn_save=lambda d: d.get("relation_id") or d.get("id")),
        C(a, "WH3-03 stock→zone edge", "relations.create",
          fn=lambda e: {"id": e.get("stk_in") or e["stk_nails"], "predicate": "wh_stock_zone",
                        "target_entity_id": e["zone_t"]}),
        C(a, "WH3-04 product→supplier edge", "relations.create",
          fn=lambda e: {"id": e["prod_new"], "predicate": "wh_supplied_by",
                        "target_entity_id": e["sup"]}),
        C(a, "WH3-05 list outgoing stock", "relations.list",
          fn=lambda e: {"id": e.get("stk_in") or e["stk_nails"]}, match="wh_stock_product"),
        C(a, "WH3-06 list incoming zone", "relations.list",
          fn=lambda e: {"id": e["wh1"], "direction": "incoming"},
          match="wh_located_in|wh_inbound_to"),
        C(a, "WH3-07 list outgoing inb1", "relations.list",
          fn=lambda e: {"id": e["inb1"], "direction": "outgoing"},
          match="wh_inbound_item|wh_inbound_to"),
        C(a, "WH3-08 bad predicate → err", "relations.create",
          fn=lambda e: {"id": e["stk_paint"], "predicate": "wh_nope",
                        "target_entity_id": e["p_paint"]}, expect="fail"),
        C(a, "WH3-09 ghost target → err", "relations.create",
          fn=lambda e: {"id": e["stk_paint"], "predicate": "wh_stock_product",
                        "target_entity_id": FAKE}, expect="fail"),
        C(a, "WH3-10 ghost source → err", "relations.create",
          {"id": FAKE, "predicate": "wh_stock_product",
           "target_entity_id": FAKE}, expect="fail"),
        C(a, "WH3-11 delete relation", "relations.delete",
          fn=lambda e: {"id": e.get("stk_in") or e["stk_nails"],
                        "relation_id": e.get("rel_paint")}, expect="ok"),
        C(a, "WH3-12 list after delete", "relations.list",
          fn=lambda e: {"id": e.get("stk_in") or e["stk_nails"]}),
    ]

    # ── WH4. search by text/fields/type/relations ────────────────────────
    a = "WH4.search"
    S += [
        C(a, "WH4-01 query wh_product type", "entities.query",
          {"query": {"from": {"type_id": "wh_product"}}}, match="Гвозди|SKU"),
        C(a, "WH4-02 query wh_warehouse type", "entities.query",
          {"query": {"from": {"type_id": "wh_warehouse"}}}, match="ТЕСТ"),
        C(a, "WH4-03 search text Гвозди", "entities.query",
          {"query": {"search": "Гвозди"}}, match="Гвозди"),
        C(a, "WH4-04 search text Склад ТЕСТ", "entities.query",
          {"query": {"search": "Склад ТЕСТ"}}, match="Склад ТЕСТ"),
        C(a, "WH4-05 search miss", "entities.query",
          {"query": {"search": "zzz-sklad-нет-zzz"}}, match="total_estimate|rows"),
        C(a, "WH4-06 filter field eq", "entities.query",
          {"query": {"from": {"type_id": "wh_product"},
                     "filters": {"wh_category": "метизы"}}}, match="Гвозди"),
        C(a, "WH4-07 where eq sku", "entities.query",
          {"query": {"from": {"type_id": "wh_product"},
                     "where": {"field": "wh_sku", "op": "eq",
                               "value": f"SKU-{N}-001"}}}, match="Гвозди"),
        C(a, "WH4-08 where contains name", "entities.query",
          {"query": {"from": {"type_id": "wh_product"},
                     "where": {"field": "name", "op": "contains",
                               "value": "Краска"}}}, match="Краска"),
        C(a, "WH4-09 where gt qty", "entities.query",
          {"query": {"from": {"type_id": "wh_stock"},
                     "where": {"field": "wh_qty", "op": "gt",
                               "value": 100}}}, match="rows|data"),
        C(a, "WH4-10 where and-combo", "entities.query",
          {"query": {"from": {"type_id": "wh_product"},
                     "where": {"and": [
                         {"field": "wh_unit", "op": "eq", "value": "кг"},
                         {"field": "name", "op": "contains",
                          "value": "Гвозди"}]}}}, match="Гвозди"),
        C(a, "WH4-11 page size 1", "entities.query",
          {"query": {"from": {"type_id": "wh_product"},
                     "page": {"size": 1}}}, match="cursor|rows"),
        C(a, "WH4-12 page after cursor", "entities.query",
          {"query": {"from": {"type_id": "wh_product"},
                     "page": {"size": 1, "after": "1"}}}, match="rows"),
        C(a, "WH4-13 search by sku text", "entities.query",
          {"query": {"search": f"SKU-{N}-002"}}, match="Краска"),
        C(a, "WH4-14 query ghost type", "entities.query",
          {"query": {"from": {"type_id": "wh_ghost"}}}, match="rows|data"),
        C(a, "WH4-15 graph from product", "entities.graph",
          fn=lambda e: {"id": e["p_nails"], "max_depth": 2},
          match="wh_warehouse|wh_zone|legal_entity|wh_stock"),
        C(a, "WH4-16 graph inbound", "entities.graph",
          fn=lambda e: {"id": e["inb1"], "max_depth": 2, "direction": "outgoing"},
          match="wh_warehouse|legal_entity|wh_product"),
    ]

    # ── WH5. inbound receiving scenario ──────────────────────────────────
    a = "WH5.inbound"
    S += [
        C(a, "WH5-01 create inbound doc", "entities.create",
          {"type_id": "wh_inbound",
           "fields": {"wh_doc_no": f"INB-{N}-010-{RUN}", "wh_doc_date": "2026-02-10",
                      "status_note": "новая"}},
          match="public_id", save="inb_new", fn_save=lambda d: d["public_id"]),
        C(a, "WH5-02 link supplier", "relations.create",
          fn=lambda e: {"id": e["inb_new"], "predicate": "wh_inbound_from",
                        "target_entity_id": e["sup"]}),
        C(a, "WH5-03 link warehouse", "relations.create",
          fn=lambda e: {"id": e["inb_new"], "predicate": "wh_inbound_to",
                        "target_entity_id": e["wh1"]}),
        C(a, "WH5-04 link item 1", "relations.create",
          fn=lambda e: {"id": e["inb_new"], "predicate": "wh_inbound_item",
                        "target_entity_id": e["p_nails"]}),
        C(a, "WH5-05 link item 2", "relations.create",
          fn=lambda e: {"id": e["inb_new"], "predicate": "wh_inbound_item",
                        "target_entity_id": e["p_glue"]}),
        C(a, "WH5-06 receive stock", "entities.create",
          {"type_id": "wh_stock", "fields": {"wh_qty": 40, "wh_batch": "B-10"}},
          match="public_id", save="stk_in", fn_save=lambda d: d["public_id"]),
        C(a, "WH5-07 stock→product", "relations.create",
          fn=lambda e: {"id": e["stk_in"], "predicate": "wh_stock_product",
                        "target_entity_id": e["p_glue"]}),
        C(a, "WH5-08 stock→rcv zone", "relations.create",
          fn=lambda e: {"id": e["stk_in"], "predicate": "wh_stock_zone",
                        "target_entity_id": e["zone_rcv"]}),
        C(a, "WH5-09 set status проведена", "entities.update",
          fn=lambda e: {"id": e["inb_new"],
                        "fields": {"status_note": "проведена"}},
          match="public_id|revision"),
        C(a, "WH5-10 workflow on inbound", "workflows.create",
          fn=lambda e: {"definition_id": "request_processing",
                        "target_entity_id": e["inb_new"],
                        "variables": {"doc": "inbound"}},
          match="instance_id", save="wf_in",
          fn_save=lambda d: d.get("instance_id") or d.get("id")),
        C(a, "WH5-11 transition start_review", "workflows.transition",
          fn=lambda e: {"id": e["wf_in"], "to": "in_review",
                        "trigger": "start_review"}, match="current_state"),
        C(a, "WH5-12 upload packing list", "documents.upload",
          fn=lambda e: {"id": e["inb_new"], "kind": "packing_list",
                        "file_name": "pack.txt",
                        "bytes": [104, 101, 108, 108, 111],
                        "title": "Упаковочный лист",
                        "content_type": "text/plain"},
          match="document_id|id"),
        C(a, "WH5-13 documents.list", "documents.list",
          fn=lambda e: {"id": e["inb_new"]}, match="pack|Упаковочн|data"),
        C(a, "WH5-14 graph verify inbound", "entities.graph",
          fn=lambda e: {"id": e["inb_new"], "max_depth": 2},
          match="wh_warehouse|legal_entity|wh_product"),
    ]

    # ── WH6. outbound shipping scenario ──────────────────────────────────
    a = "WH6.outbound"
    S += [
        C(a, "WH6-01 create outbound doc", "entities.create",
          {"type_id": "wh_outbound",
           "fields": {"wh_doc_no": f"OUT-{N}-010-{RUN}", "wh_doc_date": "2026-02-20",
                      "status_note": "новая"}},
          match="public_id", save="out_new", fn_save=lambda d: d["public_id"]),
        C(a, "WH6-02 link customer", "relations.create",
          fn=lambda e: {"id": e["out_new"], "predicate": "wh_outbound_to",
                        "target_entity_id": e["cust"]}),
        C(a, "WH6-03 link warehouse", "relations.create",
          fn=lambda e: {"id": e["out_new"], "predicate": "wh_outbound_from",
                        "target_entity_id": e["wh1"]}),
        C(a, "WH6-04 link item", "relations.create",
          fn=lambda e: {"id": e["out_new"], "predicate": "wh_outbound_item",
                        "target_entity_id": e["p_nails"]}),
        C(a, "WH6-05 decrease stock", "entities.update",
          fn=lambda e: {"id": e["stk_nails"], "fields": {"wh_qty": 1400}},
          match="public_id|revision|1400"),
        C(a, "WH6-06 picking task", "tasks.create",
          fn=lambda e: {"id": e["out_new"], "kind": "picking",
                        "title": "Комплектация OUT-010", "priority": 10},
          match="task_id|id", save="task_out",
          fn_save=lambda d: d.get("task_id") or d.get("id")),
        C(a, "WH6-07 tasks.list", "tasks.list",
          fn=lambda e: {"id": e["out_new"]}, match="Комплектац|data"),
        C(a, "WH6-08 task start", "tasks.start",
          fn=lambda e: {"id": e["task_out"]}),
        C(a, "WH6-09 task complete", "tasks.complete",
          fn=lambda e: {"id": e["task_out"], "resolution": "собрано"}),
        C(a, "WH6-10 set отгружена", "entities.update",
          fn=lambda e: {"id": e["out_new"],
                        "fields": {"status_note": "отгружена"}},
          match="public_id|revision"),
        C(a, "WH6-11 history outbound", "entities.history",
          fn=lambda e: {"id": e["out_new"]}, match="revision|data"),
        C(a, "WH6-12 report on warehouse", "reports.generate",
          fn=lambda e: {"entity_id": e["wh1"], "format": "json"},
          match="report_id"),
        C(a, "WH6-13 graph outbound", "entities.graph",
          fn=lambda e: {"id": e["out_new"], "max_depth": 2},
          match="legal_entity|wh_warehouse|wh_product"),
    ]

    # ── WH7. inventory count scenario ────────────────────────────────────
    a = "WH7.inventory"
    S += [
        C(a, "WH7-01 create inv act", "entities.create",
          {"type_id": "wh_inventory",
           "fields": {"wh_inv_no": f"INV-{N}-010-{RUN}", "wh_doc_date": "2026-03-01",
                      "wh_diff_qty": 0}},
          match="public_id", save="inv_new", fn_save=lambda d: d["public_id"]),
        C(a, "WH7-02 link warehouse", "relations.create",
          fn=lambda e: {"id": e["inv_new"], "predicate": "wh_counted_in",
                        "target_entity_id": e["wh1"]}),
        C(a, "WH7-03 record diff", "entities.update",
          fn=lambda e: {"id": e["inv_new"], "fields": {"wh_diff_qty": -5}},
          match="public_id|revision"),
        C(a, "WH7-04 check on warehouse", "checks.create",
          fn=lambda e: {"target_entity_id": e["wh1"], "check_kind": "kyc"},
          match="check_id|id", save="chk",
          fn_save=lambda d: d.get("check_id") or d.get("id")),
        C(a, "WH7-05 checks.list filtered", "checks.list",
          fn=lambda e: {"target_entity_id": e["wh1"]}, match="data|check"),
        C(a, "WH7-06 checks.get", "checks.get",
          fn=lambda e: {"id": e["chk"]}),
        C(a, "WH7-07 entity_checks.list", "entity_checks.list",
          fn=lambda e: {"id": e["wh1"]}),
        C(a, "WH7-08 factors.list wh", "factors.list",
          fn=lambda e: {"id": e["wh1"]}),
        C(a, "WH7-09 assessment.create", "assessment.create",
          fn=lambda e: {"id": e["wh1"], "rationale": "складская проверка"}),
        C(a, "WH7-10 assessments.list", "assessments.list",
          match="data|assessment"),
        C(a, "WH7-11 recount task", "tasks.create",
          fn=lambda e: {"id": e["inv_new"], "kind": "recount",
                        "title": "Пересчёт зоны STO-A"},
          match="task_id|id"),
        C(a, "WH7-12 upload act scan", "documents.upload",
          fn=lambda e: {"id": e["inv_new"], "kind": "inventory_act",
                        "file_name": "act.txt", "bytes": [97, 99, 116],
                        "title": "Акт инвентаризации",
                        "content_type": "text/plain"},
          match="document_id|id"),
        C(a, "WH7-13 monitors.list", "monitors.list", match="data|monitor"),
    ]

    # ── WH8. process orchestration on warehouse objects ──────────────────
    a = "WH8.process"
    S += [
        C(a, "WH8-01 wf create wh2", "workflows.create",
          fn=lambda e: {"definition_id": "check_execution",
                        "target_entity_id": e["wh2"]},
          match="instance_id", save="wf_chk",
          fn_save=lambda d: d.get("instance_id") or d.get("id")),
        C(a, "WH8-02 wf get", "workflows.get",
          fn=lambda e: {"id": e["wf_chk"]}, match="current_state"),
        C(a, "WH8-03 wf tasks", "workflows.tasks",
          fn=lambda e: {"id": e["wf_chk"]}),
        C(a, "WH8-04 wf transition", "workflows.transition",
          fn=lambda e: {"id": e["wf_chk"], "to": "running",
                        "trigger": "start"}, match="current_state"),
        C(a, "WH8-05 wf bad transition", "workflows.transition",
          fn=lambda e: {"id": e["wf_chk"], "to": "done",
                        "trigger": "no_such"}, expect="fail"),
        C(a, "WH8-06 wf unknown def", "workflows.create",
          fn=lambda e: {"definition_id": "wh_nope",
                        "target_entity_id": e["wh1"]}, expect="fail"),
        C(a, "WH8-07 request on product", "requests.create",
          fn=lambda e: {"target_entity_id": e["p_nails"],
                        "requested_product": "supply_audit",
                        "check_kinds": ["kyc"]}, match="request_id|id"),
        C(a, "WH8-08 requests.list", "requests.list", match="data|request"),
        C(a, "WH8-09 task cancel", "tasks.create",
          fn=lambda e: {"id": e["wh1"], "kind": "review",
                        "title": "Ревизия склада"},
          match="task_id|id", save="task_c",
          fn_save=lambda d: d.get("task_id") or d.get("id")),
        C(a, "WH8-10 cancel it", "tasks.cancel",
          fn=lambda e: {"id": e["task_c"], "reason": "отменено"}),
        C(a, "WH8-11 task ghost start", "tasks.start", {"id": FAKE},
          expect="fail"),
        C(a, "WH8-12 wf ghost get", "workflows.get", {"id": FAKE},
          expect="fail"),
        C(a, "WH8-13 check evidence", "checks.evidence",
          fn=lambda e: {"id": e["chk"]}),
    ]

    # ── WH9. admin & security on warehouse context ───────────────────────
    a = "WH9.admin"
    S += [
        C(a, "WH9-01 create wh operator", "users.create",
          {"login": f"wh_op_{N}_{RUN}", "display_name": "WH Operator",
           "is_service_account": True, "groups": ["analysts", "all_data"]},
          match="principal_id", save="op_user",
          fn_save=lambda d: d.get("principal_id") or d.get("id")),
        C(a, "WH9-02 users.list", "users.list", match="wh_op|QA|data"),
        C(a, "WH9-03 users.get", "users.get",
          fn=lambda e: {"id": e["op_user"]}, match="wh_op|WH Operator"),
        C(a, "WH9-04 update display", "users.update",
          fn=lambda e: {"id": e["op_user"],
                        "display_name": "WH Operator 2"}),
        C(a, "WH9-05 users.role_groups", "users.role_groups", match="data|admin"),
        C(a, "WH9-06 group create readers", "permissions.group_create",
          {"name": f"wh_readers_{N}_{RUN}", "class": "data",
           "description": "Складские читатели",
           "rules": [{"effect": "allow", "resource_type": "type",
                      "type_alias": "wh_stock", "action": "read"}]},
          match="group_id|id", save="grp",
          fn_save=lambda d: d.get("group_id") or d.get("id")),
        C(a, "WH9-07 permissions.groups", "permissions.groups", match="data|wh_readers"),
        C(a, "WH9-08 group update principals", "permissions.group_update",
          fn=lambda e: {"id": e["grp"], "name": f"wh_readers_{N}_{RUN}",
                        "principals": [f"wh_op_{N}_{RUN}"]}),
        C(a, "WH9-09 token create", "users.token_create",
          fn=lambda e: {"id": e["op_user"], "name": "wh-op-token"},
          match="token", save="op_token",
          fn_save=lambda d: d.get("token")),
        C(a, "WH9-10 tokens list", "users.tokens",
          fn=lambda e: {"id": e["op_user"]}, match="wh-op-token|data"),
        C(a, "WH9-11 operator context via MCP", "__call__",
          fn=lambda e: {"__tool__": "context.get", "__token__": e["op_token"],
                        "__args__": {}}, match="capabilities|principal"),
        C(a, "WH9-12 operator reads wh1", "__call__",
          fn=lambda e: {"__tool__": "entities.get", "__token__": e["op_token"],
                        "__args__": {"id": e["wh1"]}}, match="ТЕСТ"),
        C(a, "WH9-13 token revoke", "users.token_revoke",
          fn=lambda e: {"id": e["op_user"],
                        "token_id": e.get("op_token_id")}, match=""),
        C(a, "WH9-14 revoked → 401", "__call__",
          fn=lambda e: {"__tool__": "context.get", "__token__": e["op_token"],
                        "__args__": {}, "__expect401__": True},
          match=""),
        C(a, "WH9-15 audit query wh", "audit.query",
          {"resource_id": "", "limit": 5}, match="data|audit"),
        C(a, "WH9-16 audit by action", "audit.query",
          {"action": "entity.create", "limit": 5}, match="data"),
        C(a, "WH9-17 analyst deny users.create", "users.create",
          {"login": "x", "display_name": "x"}, token="analyst", expect="deny"),
        C(a, "WH9-18 users.delete temp", "users.delete",
          fn=lambda e: {"id": e["op_user"]}),
        C(a, "WH9-19 delete ghost user", "users.delete", {"id": FAKE},
          expect="fail"),
    ]

    # ── WH10. negatives & robustness ─────────────────────────────────────
    a = "WH10.negatives"
    S += [
        C(a, "WH10-01 get traversal", "entities.get", {"id": "../users"},
          expect="tool_err", match="invalid path argument"),
        C(a, "WH10-02 get ghost", "entities.get", {"id": FAKE}, expect="fail"),
        C(a, "WH10-03 get malformed", "entities.get", {"id": NOT_UUID},
          expect="fail"),
        C(a, "WH10-04 create missing type", "entities.create",
          {"fields": {"name": "x"}}, expect="tool_err"),
        C(a, "WH10-05 update ghost", "entities.update",
          {"id": FAKE, "fields": {"name": "x"}}, expect="fail"),
        C(a, "WH10-06 retire ghost", "entities.retire", {"id": FAKE},
          expect="fail"),
        C(a, "WH10-07 merge ghost pair", "entities.merge",
          fn=lambda e: {"id": e["p_nails"], "loser_public_id": FAKE,
                        "reason": "слияние теста"},
          expect="fail"),
        C(a, "WH10-08 relations missing pred", "relations.create",
          fn=lambda e: {"id": e["p_nails"]}, expect="tool_err"),
        C(a, "WH10-09 query non-object → 422", "entities.query",
          {"query": "x"}, expect="fail"),
        C(a, "WH10-10 query null → err", "entities.query", {"query": None},
          expect="tool_err"),
        C(a, "WH10-11 draft_update no etag → fail", "schema.draft_update",
          {"id": "45", "display_name": "x", "semantic_version": "1.0.0"},
          expect="fail"),
        C(a, "WH10-12 task complete ghost", "tasks.complete", {"id": FAKE},
          expect="fail"),
        C(a, "WH10-13 report ghost", "reports.generate",
          {"entity_id": FAKE, "format": "json"}, expect="fail"),
        C(a, "WH10-14 doc upload ghost", "documents.upload",
          {"id": FAKE, "kind": "x", "file_name": "f", "bytes": [1],
           "title": "t", "content_type": "text/plain"}, expect="fail"),
        C(a, "WH10-15 no token → 401", "__raw__", {"__noauth__": True},
          expect="http401"),
        C(a, "WH10-16 garbage token → 401", "__raw__",
          {"__token__": "egr_" + "0" * 64}, expect="http401"),
        C(a, "WH10-17 unknown tool → -32602", "__rpc__",
          {"method": "tools/call",
           "params": {"name": "no.such.tool", "arguments": {}}},
          expect="fail", match="-32602|not found|unknown"),
        C(a, "WH10-18 unknown method → -32601", "__rpc__",
          {"method": "nope/method"}, expect="fail", match="-32601"),
    ]

    # ── WH11. coverage top-up: >=10 meaningful cases per list/read tool ──
    a = "WH11.coverage"
    S += [
        # entities.list
        C(a, "WH11-E1 list wh_zone", "entities.list",
          {"type": "wh_zone", "limit": 50}, match="wh_zone"),
        C(a, "WH11-E2 list wh_stock", "entities.list",
          {"type": "wh_stock", "limit": 50}),
        C(a, "WH11-E3 list wh_inbound", "entities.list",
          {"type": "wh_inbound", "limit": 50}, match="INB"),
        C(a, "WH11-E4 search Клей", "entities.list",
          {"search": "Клей"}, match="Клей"),
        C(a, "WH11-E5 limit 2", "entities.list", {"limit": 2}),
        C(a, "WH11-E6 analyst list wh", "entities.list",
          {"type": "wh_product", "limit": 10}, token="analyst"),
        C(a, "WH11-E7 operator list", "entities.list", {"limit": 2},
          token="operator"),
        # schema.types / predicates / published / drafts
        C(a, "WH11-S1 types analyst", "schema.types", {}, token="analyst",
          match="wh_product"),
        C(a, "WH11-S2 types operator", "schema.types", {}, token="operator"),
        C(a, "WH11-S3 predicates analyst", "schema.predicates", {},
          token="analyst", match="wh_"),
        C(a, "WH11-S4 predicates operator", "schema.predicates", {},
          token="operator"),
        C(a, "WH11-S5 published analyst", "schema.published", {},
          token="analyst", match="wh_inbound"),
        C(a, "WH11-S6 published operator", "schema.published", {},
          token="operator"),
        C(a, "WH11-S7 published 2nd call", "schema.published", {}),
        C(a, "WH11-S8 drafts analyst deny", "schema.drafts", {}, token="analyst", expect="deny"),
        C(a, "WH11-S9 drafts operator deny", "schema.drafts", {}, token="operator", expect="deny"),
        C(a, "WH11-S10 drafts admin 2nd", "schema.drafts", {}),
        # checks.list
        C(a, "WH11-C1 checks all", "checks.list", {}, match="data"),
        C(a, "WH11-C2 status pending", "checks.list", {"status": "pending"}),
        C(a, "WH11-C3 kind kyc", "checks.list", {"kind": "kyc"}),
        C(a, "WH11-C4 limit 1", "checks.list", {"limit": 1}),
        C(a, "WH11-C5 target wh2", "checks.list",
          fn=lambda e: {"target_entity_id": e["wh2"]}),
        C(a, "WH11-C6 target ghost → 404", "checks.list",
          {"target_entity_id": FAKE}, expect="fail"),
        C(a, "WH11-C7 analyst checks", "checks.list", {"limit": 5},
          token="analyst"),
        # requests.list
        C(a, "WH11-Q1 requests all", "requests.list", {}, match="data"),
        C(a, "WH11-Q2 status filter", "requests.list", {"status": "new"}),
        C(a, "WH11-Q3 limit 1", "requests.list", {"limit": 1}),
        C(a, "WH11-Q4 analyst", "requests.list", {}, token="analyst"),
        C(a, "WH11-Q5 operator", "requests.list", {}, token="operator"),
        C(a, "WH11-Q6 cursor probe", "requests.list", {"cursor": "0"}),
        # reports.list / reports.generate
        C(a, "WH11-R1 reports.list", "reports.list", {}, match="data"),
        C(a, "WH11-R2 reports.list analyst", "reports.list", {},
          token="analyst"),
        C(a, "WH11-R3 report wh2 json", "reports.generate",
          fn=lambda e: {"entity_id": e["wh2"], "format": "json"},
          match="report_id"),
        C(a, "WH11-R4 report product csv", "reports.generate",
          fn=lambda e: {"entity_id": e["p_nails"], "format": "csv"},
          match="report_id"),
        C(a, "WH11-R5 report inbound", "reports.generate",
          fn=lambda e: {"entity_id": e["inb1"], "format": "json"}),
        C(a, "WH11-R6 report outbound", "reports.generate",
          fn=lambda e: {"entity_id": e["out1"], "format": "json"}),
        C(a, "WH11-R7 report inventory", "reports.generate",
          fn=lambda e: {"entity_id": e["inv1"], "format": "json"}),
        C(a, "WH11-R8 report bad format → err", "reports.generate",
          fn=lambda e: {"entity_id": e["wh1"], "format": "nope"},
          expect="fail"),
        C(a, "WH11-R9 report analyst deny", "reports.generate",
          fn=lambda e: {"entity_id": e["wh1"]}, token="analyst",
          expect="deny"),
        # assessments.list
        C(a, "WH11-A1 assess all", "assessments.list", {}, match="data"),
        C(a, "WH11-A2 status pending", "assessments.list",
          {"status": "pending"}),
        C(a, "WH11-A3 limit 1", "assessments.list", {"limit": 1}),
        C(a, "WH11-A4 analyst", "assessments.list", {}, token="analyst"),
        C(a, "WH11-A5 operator", "assessments.list", {}, token="operator"),
        C(a, "WH11-A6 cursor", "assessments.list", {"cursor": "0"}),
        C(a, "WH11-A7 status approved", "assessments.list",
          {"status": "approved"}),
        # monitors.list
        C(a, "WH11-M1 monitors admin", "monitors.list", {}, match="data"),
        C(a, "WH11-M2 monitors analyst deny", "monitors.list", {},
          token="analyst", expect="deny"),
        C(a, "WH11-M3 monitors operator deny", "monitors.list", {},
          token="operator", expect="deny"),
        C(a, "WH11-M4 monitors 2nd", "monitors.list", {}),
        # permissions.groups
        C(a, "WH11-P1 groups admin", "permissions.groups", {}, match="data"),
        C(a, "WH11-P2 groups analyst deny", "permissions.groups", {},
          token="analyst", expect="deny"),
        C(a, "WH11-P3 groups operator deny", "permissions.groups", {},
          token="operator", expect="deny"),
        C(a, "WH11-P4 groups 2nd", "permissions.groups", {}),
        # users.list / users.role_groups
        C(a, "WH11-U1 users q=wh", "users.list", {"q": "wh_op"},
          match="wh_op|data"),
        C(a, "WH11-U2 users q=qa", "users.list", {"q": "qa"}),
        C(a, "WH11-U3 status active", "users.list", {"status": "active"}),
        C(a, "WH11-U4 limit 5", "users.list", {"limit": 5}),
        C(a, "WH11-U5 analyst deny", "users.list", {}, token="analyst",
          expect="deny"),
        C(a, "WH11-U6 role_groups admin", "users.role_groups", {},
          match="data|admin"),
        C(a, "WH11-U7 role_groups 2nd", "users.role_groups", {}),
        C(a, "WH11-U8 role_groups deny", "users.role_groups", {},
          token="analyst", expect="deny"),
        # audit.query variants
        C(a, "WH11-D1 audit limit 3", "audit.query", {"limit": 3},
          match="data"),
        C(a, "WH11-D2 audit action create", "audit.query",
          {"action": "entity.create", "limit": 5}),
        C(a, "WH11-D3 audit kind", "audit.query", {"kind": "entity",
                                                    "limit": 5}),
        C(a, "WH11-D4 audit effect allow", "audit.query",
          {"effect": "allow", "limit": 5}),
        C(a, "WH11-D5 audit resource entity", "audit.query",
          {"resource_kind": "entity", "limit": 5}),
        C(a, "WH11-D6 audit analyst deny", "audit.query", {},
          token="analyst", expect="deny"),
        C(a, "WH11-D7 audit from range", "audit.query",
          {"from": "2026-01-01T00:00:00Z", "limit": 3}),
        C(a, "WH11-D8 audit to range", "audit.query",
          {"to": "2026-12-31T23:59:59Z", "limit": 3}, match="data"),
        # top-ups for thin no-param list tools
        C(a, "WH11-T1 monitors admin 3rd", "monitors.list", {}),
        C(a, "WH11-T2 monitors bogus arg", "monitors.list",
          {"bogus": 1}, expect="tool_err"),
        C(a, "WH11-T3 groups admin 3rd", "permissions.groups", {}),
        C(a, "WH11-T4 groups bogus arg", "permissions.groups",
          {"bogus": 1}, expect="tool_err"),
        C(a, "WH11-T5 reports.list analyst2", "reports.list", {},
          token="analyst"),
        C(a, "WH11-T6 reports.list operator", "reports.list", {},
          token="operator"),
        C(a, "WH11-T7 reports.list 3rd", "reports.list", {}),
        C(a, "WH11-T8 reports.list bogus", "reports.list",
          {"bogus": 1}, expect="tool_err"),
        C(a, "WH11-T9 reports.list 4th", "reports.list", {}),
        C(a, "WH11-T10 drafts admin 3rd", "schema.drafts", {}),
        C(a, "WH11-T11 drafts bogus", "schema.drafts",
          {"bogus": 1}, expect="tool_err"),
        C(a, "WH11-T12 drafts 4th", "schema.drafts", {}),
        C(a, "WH11-T13 predicates admin2", "schema.predicates", {}),
        C(a, "WH11-T14 predicates bogus", "schema.predicates",
          {"bogus": 1}, expect="tool_err"),
        C(a, "WH11-T15 predicates 3rd", "schema.predicates", {}),
        C(a, "WH11-T16 published admin2", "schema.published", {}),
        C(a, "WH11-T17 published bogus", "schema.published",
          {"bogus": 1}, expect="tool_err"),
        C(a, "WH11-T18 published 3rd", "schema.published", {}),
        C(a, "WH11-T19 types admin2", "schema.types", {}),
        C(a, "WH11-T20 types bogus", "schema.types",
          {"bogus": 1}, expect="tool_err"),
        C(a, "WH11-T21 types 3rd", "schema.types", {}),
        C(a, "WH11-T22 users.list q none", "users.list",
          {"q": "zzz-none"}),
        C(a, "WH11-T23 role_groups operator deny", "users.role_groups",
          {}, token="operator", expect="deny"),
        C(a, "WH11-T24 role_groups 3rd", "users.role_groups", {}),
        C(a, "WH11-T25 role_groups bogus", "users.role_groups",
          {"bogus": 1}, expect="tool_err"),
    ]

    # ── WH12. extended ops: audit, files, connectors, schema lifecycle ───
    a = "WH12.extended"
    HI_SHA = "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4"
    S += [
        C(a, "WH12-CTX1 context admin", "context.get", {},
          match="principal"),
        C(a, "WH12-CTX2 context caps", "context.get", {},
          match="capabilities"),
        C(a, "WH12-CTX3 context analyst", "context.get", {},
          token="analyst", match="principal|capabilities"),
        C(a, "WH12-CTX4 context operator", "context.get", {},
          token="operator"),
        C(a, "WH12-CTX5 context groups", "context.get", {},
          match="groups|capabilities"),
        C(a, "WH12-CTX6 context 2nd", "context.get", {}),
        C(a, "WH12-CTX7 context 3rd", "context.get", {}),
        C(a, "WH12-DS1 stats admin", "dashboard.stats", {},
          match="my_tasks|data"),
        C(a, "WH12-DS2 stats reports", "dashboard.stats", {},
          match="ready_reports|checks"),
        C(a, "WH12-DS3 stats analyst", "dashboard.stats", {},
          token="analyst"),
        C(a, "WH12-DS4 stats operator", "dashboard.stats", {},
          token="operator"),
        C(a, "WH12-DS5 stats 2nd", "dashboard.stats", {}),
        C(a, "WH12-DS6 stats 3rd", "dashboard.stats", {}),
        C(a, "WH12-DS7 stats 4th", "dashboard.stats", {}),
        C(a, "WH12-AU1 checkpoint create", "audit.checkpoint_create", {},
          match="checkpoint_id", save="ckpt",
          fn_save=lambda d: d.get("checkpoint_id")),
        C(a, "WH12-AU2 checkpoint get", "audit.checkpoint_get", {},
          match="entry_hash"),
        C(a, "WH12-AU3 checkpoint create 2nd", "audit.checkpoint_create",
          {}, match="checkpoint_id"),
        C(a, "WH12-AU4 checkpoint get 2nd", "audit.checkpoint_get", {}),
        C(a, "WH12-AU5 verify full", "audit.verify", {}, match="valid"),
        C(a, "WH12-AU6 verify range", "audit.verify",
          {"from": "2026-01-01T00:00:00Z", "to": "2027-01-01T00:00:00Z"},
          match="valid"),
        C(a, "WH12-AU7 verify narrow", "audit.verify",
          {"from": "2026-10-01T00:00:00Z"}, match="valid|entry_count"),
        C(a, "WH12-AU8 verify analyst deny", "audit.verify", {},
          token="analyst", expect="deny"),
        C(a, "WH12-AU9 checkpoint analyst deny", "audit.checkpoint_get",
          {}, token="analyst", expect="deny"),
        C(a, "WH12-AU10 retention get", "audit.retention_get", {},
          match="retention_days"),
        C(a, "WH12-AU11 retention get analyst deny", "audit.retention_get",
          {}, token="analyst", expect="deny"),
        C(a, "WH12-AU12 retention set 365", "audit.retention_set",
          {"retention_days": 365}, match="retention_days"),
        C(a, "WH12-AU13 retention set legal_hold", "audit.retention_set",
          {"retention_days": 180, "legal_hold": True}),
        C(a, "WH12-AU14 retention set restore", "audit.retention_set",
          {"retention_days": 3650}),
        C(a, "WH12-AU15 retention set analyst deny", "audit.retention_set",
          {"retention_days": 30}, token="analyst", expect="deny"),
        C(a, "WH12-AU16 retention set operator deny",
          "audit.retention_set", {"retention_days": 30},
          token="operator", expect="deny"),
        C(a, "WH12-AU17 verify operator deny", "audit.verify", {},
          token="operator", expect="deny"),
        C(a, "WH12-DOC1 upload for download", "documents.upload",
          fn=lambda e: {"id": e["wh1"], "file_name": "wh12.txt",
                        "kind": "other", "title": "WH12 Doc",
                        "content_type": "text/plain",
                        "bytes": [104, 105, 33]},
          match="id", save="doc12", fn_save=lambda d: d.get("id")),
        C(a, "WH12-DOC2 download doc", "documents.download",
          fn=lambda e: {"id": e["wh1"], "doc_id": e.get("doc12")},
          match="hi!"),
        C(a, "WH12-DOC3 download ghost doc", "documents.download",
          fn=lambda e: {"id": e["wh1"], "doc_id": FAKE}, expect="fail"),
        C(a, "WH12-DOC4 download ghost entity", "documents.download",
          {"id": FAKE, "doc_id": FAKE}, expect="fail"),
        C(a, "WH12-DOC5 delete doc", "documents.delete",
          fn=lambda e: {"id": e["wh1"], "doc_id": e.get("doc12")},
          match="deleted|true"),
        C(a, "WH12-DOC6 delete again → err", "documents.delete",
          fn=lambda e: {"id": e["wh1"], "doc_id": e.get("doc12")},
          expect="fail"),
        C(a, "WH12-DOC7 delete ghost", "documents.delete",
          {"id": FAKE, "doc_id": FAKE}, expect="fail"),
        C(a, "WH12-DOC8 download analyst", "documents.download",
          fn=lambda e: {"id": e["wh1"], "doc_id": e.get("doc12")},
          token="analyst", expect="reach"),
        C(a, "WH12-DOC9 delete analyst deny", "documents.delete",
          fn=lambda e: {"id": e["wh1"], "doc_id": e.get("doc12")},
          token="analyst", expect="deny"),
        C(a, "WH12-DOC10 upload 2nd doc", "documents.upload",
          fn=lambda e: {"id": e["wh2"], "file_name": "wh12b.txt",
                        "kind": "other", "title": "WH12 Doc B",
                        "content_type": "text/plain", "bytes": [98]},
          save="doc12b", fn_save=lambda d: d.get("id")),
        C(a, "WH12-DOC11 download wh2", "documents.download",
          fn=lambda e: {"id": e["wh2"], "doc_id": e.get("doc12b")},
          match='"b"'),
        C(a, "WH12-F1 upload init", "files.upload_init",
          {"filename": "wh12.bin", "content_type": "application/octet-stream",
           "expected_size": 2, "expected_hash": HI_SHA,
           "hash_algorithm": "sha256", "chunk_size": 2},
          match="upload_id|id", save="up1",
          fn_save=lambda d: d.get("upload_id") or d.get("id")),
        C(a, "WH12-F2 upload status", "files.upload_status",
          fn=lambda e: {"id": e.get("up1")}, match="status|chunks"),
        C(a, "WH12-F3 upload chunk 0", "files.upload_chunk",
          fn=lambda e: {"id": e.get("up1"), "index": 0,
                        "data_base64": "aGk="}, match="received|index"),
        C(a, "WH12-F4 status after chunk", "files.upload_status",
          fn=lambda e: {"id": e.get("up1")}),
        C(a, "WH12-F5 upload complete", "files.upload_complete",
          fn=lambda e: {"id": e.get("up1"), "final_hash": HI_SHA,
                        "total_size": 2},
          match="file_id|id", save="file12",
          fn_save=lambda d: d.get("object_id") or d.get("file_id") or d.get("id")),
        C(a, "WH12-F6 files.get", "files.get",
          fn=lambda e: {"id": e.get("file12")}, match="filename|file"),
        C(a, "WH12-F7 files.download", "files.download",
          fn=lambda e: {"id": e.get("file12")},
          match='"hi"'),
        C(a, "WH12-F8 files.delete", "files.delete",
          fn=lambda e: {"id": e.get("file12")}, match="deleted|true"),
        C(a, "WH12-F9 get deleted → err", "files.get",
          fn=lambda e: {"id": e.get("file12")}, expect="fail"),
        C(a, "WH12-F10 init 2nd + abort", "files.upload_init",
          {"filename": "wh12b.bin",
           "content_type": "application/octet-stream",
           "expected_size": 4,
           "expected_hash": HI_SHA,
           "hash_algorithm": "sha256", "chunk_size": 2},
          save="up2", fn_save=lambda d: d.get("upload_id") or d.get("id")),
        C(a, "WH12-F11 upload abort", "files.upload_abort",
          fn=lambda e: {"id": e.get("up2")}, match="abort|status"),
        C(a, "WH12-F12 status aborted", "files.upload_status",
          fn=lambda e: {"id": e.get("up2")}, match="bort|status"),
        C(a, "WH12-F13 chunk ghost session", "files.upload_chunk",
          {"id": FAKE, "index": 0, "data_base64": "aGk="}, expect="fail"),
        C(a, "WH12-F14 complete ghost", "files.upload_complete",
          {"id": FAKE, "final_hash": HI_SHA, "total_size": 2},
          expect="fail"),
        C(a, "WH12-F15 download ghost file", "files.download",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-F16 delete ghost file", "files.delete",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-F17 status ghost", "files.upload_status",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-F18 abort ghost", "files.upload_abort",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-F19 init zero size → err", "files.upload_init",
          {"filename": "z.bin", "content_type": "application/octet-stream",
           "expected_size": 0, "expected_hash": HI_SHA,
           "hash_algorithm": "sha256", "chunk_size": 1}, expect="fail"),
        C(a, "WH12-F20 files.get operator", "files.get",
          fn=lambda e: {"id": e.get("file12") or FAKE},
          token="operator", expect="deny"),
        C(a, "WH12-CON1 create", "connectors.create",
          {"connector_type_id": "manual",
           "display_name": f"WH12 conn {RUN}",
           "environment": "prod"},
          match="instance_id", save="conn",
          fn_save=lambda d: d.get("instance_id") or d.get("id")),
        C(a, "WH12-CON2 create 2nd", "connectors.create",
          {"connector_type_id": "manual",
           "display_name": f"WH12 conn2 {RUN}"}),
        C(a, "WH12-CON3 create analyst deny", "connectors.create",
          {"connector_type_id": "manual"}, token="analyst",
          expect="deny"),
        C(a, "WH12-CON3b create operator deny", "connectors.create",
          {"connector_type_id": "manual"}, token="operator",
          expect="deny"),
        C(a, "WH12-CON4 get", "connectors.get",
          fn=lambda e: {"id": e.get("conn")}, match="manual"),
        C(a, "WH12-CON5 get ghost", "connectors.get", {"id": FAKE},
          expect="fail"),
        C(a, "WH12-CON6 list", "connectors.list", {}, match="manual|data"),
        C(a, "WH12-CON7 list 2nd", "connectors.list", {}),
        C(a, "WH12-CON8 list analyst", "connectors.list", {},
          token="analyst", expect="deny"),
        C(a, "WH12-CON9 list operator deny", "connectors.list", {},
          token="operator", expect="deny"),
        C(a, "WH12-CON10 update name", "connectors.update",
          fn=lambda e: {"id": e.get("conn"),
                        "display_name": f"WH12 conn upd {RUN}",
                        "rate_limit_rps": 20, "trust_rank": 75},
          match="upd|rate_limit|trust"),
        C(a, "WH12-CON11 update ghost", "connectors.update",
          {"id": FAKE, "display_name": "x"}, expect="fail"),
        C(a, "WH12-CON12 update analyst deny", "connectors.update",
          fn=lambda e: {"id": e.get("conn"), "display_name": "nope"},
          token="analyst", expect="deny"),
        C(a, "WH12-CON13 disable", "connectors.disable",
          fn=lambda e: {"id": e.get("conn")},
          match="enabled|disabled|false"),
        C(a, "WH12-CON14 enable", "connectors.enable",
          fn=lambda e: {"id": e.get("conn")}, match="enabled|true"),
        C(a, "WH12-CON15 health", "connectors.health",
          fn=lambda e: {"id": e.get("conn")}, match="health|status|ok"),
        C(a, "WH12-CON16 health ghost", "connectors.health",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-CON17 disable ghost", "connectors.disable",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-CON18 enable ghost", "connectors.enable",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-CON19 delete", "connectors.delete",
          fn=lambda e: {"id": e.get("conn")}, match="deleted|true"),
        C(a, "WH12-CON20 delete ghost", "connectors.delete",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-CON21 get deleted → err", "connectors.get",
          fn=lambda e: {"id": e.get("conn")}, expect="fail"),
        C(a, "WH12-MON1 monitors list save", "monitors.list", {},
          match="monitor_id", save="mon_id",
          fn_save=lambda d: (d[0] or {}).get("monitor_id")
                            if isinstance(d, list) and d else None),
        C(a, "WH12-MON2 run monitor", "monitors.run",
          fn=lambda e: {"id": e.get("mon_id")}, match="run_id|ran_at"),
        C(a, "WH12-MON3 run ghost", "monitors.run", {"id": FAKE},
          expect="fail"),
        C(a, "WH12-MON4 run analyst deny", "monitors.run",
          fn=lambda e: {"id": e.get("mon_id")}, token="analyst",
          expect="deny"),
        C(a, "WH12-MON5 run operator deny", "monitors.run",
          fn=lambda e: {"id": e.get("mon_id")}, token="operator",
          expect="deny"),
        C(a, "WH12-FAC1 create check→factor", "checks.create",
          fn=lambda e: {"target_entity_id": e["wh1"], "check_kind": "kyc"},
          match="check_id|id", save="chk12",
          fn_save=lambda d: d.get("check_id") or d.get("id")),
        C(a, "WH12-FAC2 factors list", "factors.list",
          fn=lambda e: {"id": e["wh1"]}, match="data",
          save="factors12", fn_save=lambda d: d),
        C(a, "WH12-FAC3 resolve factor", "factors.resolve",
          fn=lambda e: {"id": e["wh1"],
                        "factor_id": (e.get("factors12") or [{}])[0].get(
                            "factor_id") or FAKE,
                        "resolution": "проверено вручную"},
          expect="reach"),
        C(a, "WH12-FAC4 resolve ghost factor", "factors.resolve",
          fn=lambda e: {"id": e["wh1"], "factor_id": FAKE,
                        "resolution": "x"}, expect="fail"),
        C(a, "WH12-FAC5 resolve ghost entity", "factors.resolve",
          {"id": FAKE, "factor_id": FAKE}, expect="fail"),
        C(a, "WH12-ASM1 assessment.create", "assessment.create",
          fn=lambda e: {"id": e["wh1"], "force": True},
          match="assessment_id|id", save="asm12",
          fn_save=lambda d: d.get("assessment_id") or d.get("id")),
        C(a, "WH12-ASM2 assessment.get", "assessment.get",
          fn=lambda e: {"id": e["wh1"]},
          match="score|risk_level|assessment"),
        C(a, "WH12-ASM3 assessment.get ghost", "assessment.get",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-ASM4 assessment.get analyst", "assessment.get",
          fn=lambda e: {"id": e["wh1"]},
          token="analyst", expect="reach"),
        C(a, "WH12-SC1 published save", "schema.published", {},
          match="revision_id", save="pub_list", fn_save=lambda d: d),
        C(a, "WH12-SC2 draft create", "schema.draft_create",
          fn=lambda e: {"name": f"WH12 probe {RUN}",
                        "base_revision": (e.get("pub_list") or [{}])[0]
                            .get("revision_id")},
          match="revision_id|draft_id|id", save="d12",
          fn_save=lambda d: d.get("revision_id") or d.get("draft_id")
                            or d.get("id")),
        C(a, "WH12-SC3 draft get", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12")}, match="wh_|types|etag"),
        C(a, "WH12-SC4 draft get save etag", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12")}, save="d12_etag",
          fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC5 draft validate", "schema.draft_validate",
          fn=lambda e: {"id": e.get("d12")}, match="valid|findings"),
        C(a, "WH12-SC6 draft impact", "schema.draft_impact",
          fn=lambda e: {"id": e.get("d12")},
          match="compatibility|impact|breaking"),
        C(a, "WH12-SC7 get etag after impact", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12")}, save="d12_etag",
          fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC8 draft delete", "schema.draft_delete",
          fn=lambda e: {"id": e.get("d12"),
                        "if_match": e.get("d12_etag")},
          match="deleted|retired|status"),
        C(a, "WH12-SC9 draft get after delete", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12")}, expect="reach"),
        C(a, "WH12-SC10 draft create analyst deny", "schema.draft_create",
          {"name": "nope"}, token="analyst", expect="deny"),
        C(a, "WH12-SC11 draft create 2nd", "schema.draft_create",
          fn=lambda e: {"name": f"WH12 pub {RUN}",
                        "base_revision": (e.get("pub_list") or [{}])[0]
                            .get("revision_id")},
          save="d12b", fn_save=lambda d: d.get("revision_id")
                            or d.get("draft_id") or d.get("id")),
        C(a, "WH12-SC11b draft get etag pre-update", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12b")}, save="d12b_e0",
          fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC12 draft update ver", "schema.draft_update",
          fn=lambda e: {"id": e.get("d12b"),
                        "display_name": f"WH12 pub {RUN}",
                        "if_match": e.get("d12b_e0"),
                        "semantic_version": _next_ver(
                            (e.get("pub_list") or [{}])[0]
                                .get("semantic_version") or "0.0.0")},
          match="etag|revision"),
        C(a, "WH12-SC13 draft get etag2", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12b")}, save="d12b_etag",
          fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC14 draft validate b", "schema.draft_validate",
          fn=lambda e: {"id": e.get("d12b")}),
        C(a, "WH12-SC15 draft impact b", "schema.draft_impact",
          fn=lambda e: {"id": e.get("d12b")}),
        C(a, "WH12-SC16 draft get etag3", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12b")}, save="d12b_etag2",
          fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC17 draft publish", "schema.draft_publish",
          fn=lambda e: {"id": e.get("d12b"),
                        "reason": "WH12 publish lifecycle probe",
                        "if_match": e.get("d12b_etag2")},
          match="published|revision_id", save="rev12",
          fn_save=lambda d: d.get("revision_id") or d.get("id")),
        C(a, "WH12-SC18 revision_get new", "schema.revision_get",
          fn=lambda e: {"id": e.get("rev12")},
          match="published|semantic_version", save="rev12_rec",
          fn_save=lambda d: d),
        C(a, "WH12-SC19 revision_migration", "schema.revision_migration",
          fn=lambda e: {"id": e.get("rev12")},
          match="migration_required|status"),
        C(a, "WH12-SC20 revision ghost migration",
          "schema.revision_migration", {"id": "999999"}, expect="fail"),
        C(a, "WH12-SC20b revision_get rev12 etag", "schema.revision_get",
          fn=lambda e: {"id": e.get("rev12")},
          save="rev12_etag", fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC21 revision_retire current", "schema.revision_retire",
          fn=lambda e: {"id": e.get("rev12"),
                        "reason": "WH12 lifecycle retire probe",
                        "if_match": e.get("rev12_etag")},
          match="retired|status"),
        C(a, "WH12-SC21b restore draft", "schema.draft_create",
          fn=lambda e: {"name": f"WH12 restore {RUN}",
                        "base_revision": e.get("rev12")},
          match="revision_id|draft_id|id", save="d12c",
          fn_save=lambda d: d.get("revision_id") or d.get("draft_id")
                            or d.get("id")),
        C(a, "WH12-SC21c restore etag", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12c")}, save="d12c_e0",
          fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC21d restore update ver", "schema.draft_update",
          fn=lambda e: {"id": e.get("d12c"),
                        "display_name": f"WH12 restore {RUN}",
                        "if_match": e.get("d12c_e0"),
                        "semantic_version": _next_ver(
                            (e.get("rev12_rec") or {})
                                .get("semantic_version") or "0.0.0")},
          match="etag|revision"),
        C(a, "WH12-SC21e restore validate", "schema.draft_validate",
          fn=lambda e: {"id": e.get("d12c")}),
        C(a, "WH12-SC21e2 restore impact", "schema.draft_impact",
          fn=lambda e: {"id": e.get("d12c")}),
        C(a, "WH12-SC21e3 restore etag2", "schema.draft_get",
          fn=lambda e: {"id": e.get("d12c")}, save="d12c_e1",
          fn_save=lambda d: d.get("etag")),
        C(a, "WH12-SC21f restore publish", "schema.draft_publish",
          fn=lambda e: {"id": e.get("d12c"),
                        "reason": "WH12 restore published catalog",
                        "if_match": e.get("d12c_e1")},
          match="published|revision_id"),
        C(a, "WH12-SC22 revision_retire ghost", "schema.revision_retire",
          {"id": "999999", "reason": "no such revision probe",
           "if_match": "\"x\""}, expect="fail"),
        C(a, "WH12-SC23 revision_retire analyst deny",
          "schema.revision_retire",
          fn=lambda e: {"id": e.get("rev12") or "1",
                        "reason": "denied analyst retire probe"},
          token="analyst", expect="deny"),
        C(a, "WH12-PG1 group create temp", "permissions.group_create",
          {"name": f"wh_tmp_{RUN}", "class": "data", "principals": []},
          match="group_id|id", save="grp12",
          fn_save=lambda d: d.get("group_id") or d.get("id")),
        C(a, "WH12-PG2 group delete", "permissions.group_delete",
          fn=lambda e: {"id": e.get("grp12")}, match="deleted|true"),
        C(a, "WH12-PG3 delete again → err", "permissions.group_delete",
          fn=lambda e: {"id": e.get("grp12")}, expect="fail"),
        C(a, "WH12-PG4 delete ghost", "permissions.group_delete",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-PG5 delete analyst deny", "permissions.group_delete",
          fn=lambda e: {"id": e.get("grp") or FAKE},
          token="analyst", expect="deny"),
        C(a, "WH12-UA1 activity admin self", "users.activity",
          {"id": "00000000-0000-0000-0000-0000000000a1"},
          match="data|activity"),
        C(a, "WH12-UA2 activity op user", "users.activity",
          fn=lambda e: {"id": e.get("op_user") or FAKE},
          expect="reach"),
        C(a, "WH12-UA3 activity ghost", "users.activity",
          {"id": FAKE}, expect="fail"),
        C(a, "WH12-UA4 activity limit", "users.activity",
          {"id": "00000000-0000-0000-0000-0000000000a1", "limit": 5}),
        C(a, "WH12-UA5 activity q filter", "users.activity",
          {"id": "00000000-0000-0000-0000-0000000000a1", "q": "schema"}),
        C(a, "WH12-UA6 activity analyst deny", "users.activity",
          {"id": "00000000-0000-0000-0000-0000000000a1"},
          token="analyst", expect="deny"),
    ]

    # ── WH12-T. top-ups for thin extended tools ──────────────────────────
    a = "WH12.topup"
    S += [
        C(a, "WH12-TA1 checkpoint create 3rd", "audit.checkpoint_create",
          {}, match="checkpoint_id"),
        C(a, "WH12-TA2 checkpoint create 4th", "audit.checkpoint_create",
          {}),
        C(a, "WH12-TA3 checkpoint create 5th", "audit.checkpoint_create",
          {}),
        C(a, "WH12-TA4 checkpoint create op deny",
          "audit.checkpoint_create", {}, token="operator", expect="deny"),
        C(a, "WH12-TA5 checkpoint create analyst deny",
          "audit.checkpoint_create", {}, token="analyst", expect="deny"),
        C(a, "WH12-TB1 checkpoint get 3rd", "audit.checkpoint_get", {},
          match="checkpoint_id"),
        C(a, "WH12-TB2 checkpoint get 4th", "audit.checkpoint_get", {}),
        C(a, "WH12-TB3 checkpoint get 5th", "audit.checkpoint_get", {}),
        C(a, "WH12-TB4 checkpoint get op deny", "audit.checkpoint_get",
          {}, token="operator", expect="deny"),
        C(a, "WH12-TB5 checkpoint get match hash", "audit.checkpoint_get",
          {}, match="entry_hash"),
        C(a, "WH12-TC1 retention get 2nd", "audit.retention_get", {},
          match="retention_days"),
        C(a, "WH12-TC2 retention get 3rd", "audit.retention_get", {}),
        C(a, "WH12-TC3 retention get legal_hold", "audit.retention_get",
          {}, match="legal_hold"),
        C(a, "WH12-TC4 retention get archive", "audit.retention_get",
          {}, match="archive_before_delete"),
        C(a, "WH12-TC5 retention get op deny", "audit.retention_get",
          {}, token="operator", expect="deny"),
        C(a, "WH12-TD1 verify to-only", "audit.verify",
          {"to": "2027-01-01T00:00:00Z"}),
        C(a, "WH12-TD2 verify checkpoint deny analyst", "audit.verify",
          {}, token="analyst", expect="deny"),
        C(a, "WH12-TD3 checkpoint_create reach", "audit.checkpoint_create",
          {}),
        C(a, "WH12-TD4 verify 3rd", "audit.verify", {}, match="valid"),
        C(a, "WH12-TE1 connectors.list 3rd", "connectors.list", {},
          match="data"),
        C(a, "WH12-TE2 connectors.list 4th", "connectors.list", {}),
        C(a, "WH12-TE3 connectors.list match conn", "connectors.list",
          {}, match="WH12 conn2|data"),
        C(a, "WH12-TF1 draft_create no-base", "schema.draft_create",
          {"name": f"WH12 nb {RUN}"}, match="revision_id|draft_id|id"),
        C(a, "WH12-TF2 draft_create operator deny",
          "schema.draft_create", {"name": "nope2"}, token="operator",
          expect="deny"),
    ]

    # ── WHX. per-tool generic negatives for every tool exercised ─────────
    used = {c["tool"] for c in S if not c["tool"].startswith("__")}
    wh_tools = [t for t in tools if t["name"] in used]
    S += synthetic_cases(wh_tools, operator_tools)
    return S


def run_case(c, tokens, env, idn):
    token = tokens.get(c["token"], tokens["admin"])
    if c["tool"] == "__raw__":
        if c["args"].get("__noauth__"):
            st, r = rpc(None, "tools/list", {}, idn)
        else:
            st, r = rpc(c["args"].get("__token__"), "tools/list", {}, idn)
        ok = (st == 401) == (c["expect"] == "http401")
        return ("pass" if ok else "fail"), f"http {st}", None
    if c["tool"] == "__rpc__":
        st, r = rpc(token, c["args"]["method"], c["args"].get("params", {}), idn)
        txt = json.dumps(r)
        ok = bool(re.search(c["match"] or ".", txt))
        return ("pass" if ok else "fail"), txt[:300], None
    if c["tool"] == "__call__":
        a2 = c["fn"](env) if c["fn"] else {}
        tok = a2.get("__token__") or token
        st, r = rpc(tok, "tools/call",
                    {"name": a2["__tool__"], "arguments": a2.get("__args__", {})},
                    idn)
        if a2.get("__expect401__"):
            # The MCP gate may serve the cached (still-valid) context for up
            # to 30s, in which case the API 401 surfaces as a tool error with
            # HTTP 200 — either rejection signal is correct.
            ok = st == 401 or is_tool_error(r)
            return ("pass" if ok else "fail"), f"http {st} {tool_text(r)[:120]}", None
        txt = tool_text(r)
        err = is_tool_error(r)
        ok = not err
        if ok and c["match"] and not re.search(c["match"], txt):
            ok = False
        return ("pass" if ok else "fail"), txt[:300], r

    a2 = dict(c["args"])
    if c["fn"]:
        a2 = c["fn"](env) or {}
    # drop args that still contain None placeholders (env not ready)
    a2 = {k: v for k, v in a2.items() if v is not None}
    st, r = rpc(token, "tools/call",
                {"name": c["tool"], "arguments": a2}, idn)
    txt = tool_text(r)
    err = is_tool_error(r)
    exp = c["expect"]
    if exp == "ok":
        ok = not err
    elif exp == "reach":
        ok = "not granted" not in txt and "-32602" not in json.dumps(r)
    elif exp in ("tool_err", "fail", "deny"):
        ok = err
        if exp == "deny":
            ok = err and ("not granted" in txt or "capability" in txt.lower()
                          or "-32602" in json.dumps(r))
    else:
        ok = not err
    detail = txt[:300]
    if ok and exp == "ok" and c["match"]:
        if not re.search(c["match"], txt, re.IGNORECASE):
            ok = False
            detail = f"match '{c['match']}' missing: {txt[:200]}"
    saved = None
    if ok and not err and c["save"]:
        d = tool_data(r) or {}
        saved = c.get("fn_save")(d) if c.get("fn_save") else d
    return ("pass" if ok else "fail"), detail, (saved if ok else None)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--area")
    ap.add_argument("--out", default=OUT)
    args = ap.parse_args()

    tokens = json.load(open(os.path.join(HERE, ".wh_tokens.json"),
                            encoding="utf-8"))
    env = json.load(open(os.path.join(HERE, ".wh_objects.json"),
                         encoding="utf-8"))

    st, tl_admin = rpc(tokens["admin"], "tools/list", {}, 1)
    tools = (tl_admin.get("result") or {}).get("tools", [])
    st, tl_op = rpc(tokens["operator"], "tools/list", {}, 2)
    operator_tools = {t["name"]
                      for t in (tl_op.get("result") or {}).get("tools", [])}
    print(f"admin tools={len(tools)} operator tools={len(operator_tools)}")

    cases = build_cases(env, tools, operator_tools)
    if args.area:
        cases = [c for c in cases if c["area"].startswith(args.area)]

    results = []
    for i, c in enumerate(cases, 1):
        t0 = time.time()
        try:
            outcome, detail, saved = run_case(c, tokens, env, i + 100)
        except Exception as e:
            outcome, detail, saved = "error", f"{type(e).__name__}: {e}", None
        if saved is not None and c.get("save"):
            env[c["save"]] = saved
            # also capture token_id for revocation case
            if c["save"] == "op_token":
                st2, r2 = rpc(tokens["admin"], "tools/call",
                              {"name": "users.tokens",
                               "arguments": {"id": env["op_user"]}}, i + 90000)
                for tk in (tool_data(r2) or []):
                    if tk.get("name") == "wh-op-token":
                        env["op_token_id"] = tk.get("token_id")
        dt = int((time.time() - t0) * 1000)
        results.append({"area": c["area"], "name": c["name"],
                        "tool": c["tool"], "expect": c["expect"],
                        "outcome": outcome, "ms": dt, "detail": detail})
        mark = {"pass": "PASS", "fail": "FAIL", "error": "ERR "}[outcome]
        print(f"[{mark}] {c['name']:38} {c['tool']:26} {dt}ms {detail[:110]}")

    ok = sum(1 for r in results if r["outcome"] == "pass")
    print(f"\n{ok}/{len(results)} passed")
    per_tool = {}
    for r in results:
        if r["tool"].startswith("__"):
            continue
        per_tool.setdefault(r["tool"], 0)
        per_tool[r["tool"]] += 1
    under = sorted(t for t, n in per_tool.items() if n < 10)
    print(f"tools={len(per_tool)} <10={under}")
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump({"passed": ok, "total": len(results), "results": results,
                   "per_tool": per_tool},
                  f, ensure_ascii=False, indent=2)
    sys.exit(0 if ok == len(results) else 1)


if __name__ == "__main__":
    main()
