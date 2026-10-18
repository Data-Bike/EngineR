"""Warehouse «ТЕСТ» provisioning — EVERYTHING through the MCP endpoint.

Steps:
  1. schema.draft_create(base_revision=<current published>) — inherits catalog.
  2. schema.draft_update — appends warehouse types + predicates (full arrays).
  3. schema.draft_validate -> schema.draft_impact -> schema.draft_publish.
  4. entities.create — warehouses, zones, products, stock records,
     inbound/outbound deliveries, inventory check, supplier/customer, keeper.
  5. relations.create — edges per the warehouse model.

Persists created public ids to .wh_objects.json for the test runner.
Idempotent-ish: re-runs create fresh draft/objects (names carry NONCE).
"""
import json
import os
import sys
import time
import urllib.request
import urllib.error

HERE = os.path.dirname(os.path.abspath(__file__))
MCP = os.environ.get("ENGINER_MCP", "http://localhost:8020")
NONCE = os.environ.get("WH_NONCE") or hex(int(time.time()))[2:]


def load_tokens():
    with open(os.path.join(HERE, ".wh_tokens.json"), encoding="utf-8") as f:
        return json.load(f)


def rpc(token, tool, args, idn=1):
    body = {"jsonrpc": "2.0", "id": idn, "method": "tools/call",
            "params": {"name": tool, "arguments": args}}
    req = urllib.request.Request(
        MCP + "/mcp", data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json",
                 "Authorization": "Bearer " + token})
    try:
        resp = urllib.request.urlopen(req)
    except urllib.error.HTTPError as e:
        return {"_http": e.code, "_body": e.read()[:400].decode("utf-8", "replace")}
    d = json.loads(resp.read())
    if "error" in d:
        return {"_rpc_err": d["error"]}
    txt = d["result"]["content"][0]["text"]
    try:
        out = json.loads(txt)
    except Exception:
        return {"_text": txt}
    if isinstance(out, dict) and "error" in out:
        out["_api_err"] = out["error"]
    return out


def failed(resp):
    return bool(resp.get("_rpc_err") or resp.get("_http")
                or resp.get("_api_err") or resp.get("error"))


def pub_id(resp):
    d = resp.get("data") or {}
    return d.get("public_id") or d.get("id") or d.get("entity_id")


# --- warehouse schema extension -------------------------------------------
WH_TYPES = [
    {"type_id": "wh_warehouse", "stable_alias": "wh_warehouse",
     "entity_class": "data", "label": "Склад",
     "fields": [
         {"kind": "text", "alias": "wh_code", "label": "Код склада",
          "identity": True, "required": True},
         {"kind": "text", "alias": "name", "label": "Наименование",
          "required": True},
         {"kind": "text", "alias": "address", "label": "Адрес"},
         {"kind": "decimal", "alias": "wh_capacity_m2", "label": "Площадь, м²"}]},
    {"type_id": "wh_zone", "stable_alias": "wh_zone",
     "entity_class": "data", "label": "Зона склада",
     "fields": [
         {"kind": "text", "alias": "wh_zone_code", "label": "Код зоны",
          "identity": True, "required": True},
         {"kind": "text", "alias": "wh_zone_kind", "label": "Тип зоны"},
         {"kind": "decimal", "alias": "wh_capacity_m2", "label": "Площадь, м²"}]},
    {"type_id": "wh_product", "stable_alias": "wh_product",
     "entity_class": "data", "label": "Товар",
     "fields": [
         {"kind": "text", "alias": "wh_sku", "label": "Артикул (SKU)",
          "identity": True, "required": True},
         {"kind": "text", "alias": "name", "label": "Наименование",
          "required": True},
         {"kind": "text", "alias": "wh_unit", "label": "Ед. измерения"},
         {"kind": "text", "alias": "wh_category", "label": "Категория"}]},
    {"type_id": "wh_stock", "stable_alias": "wh_stock",
     "entity_class": "data", "label": "Складской остаток",
     "fields": [
         {"kind": "text", "alias": "wh_batch", "label": "Партия"},
         {"kind": "decimal", "alias": "wh_qty", "label": "Количество",
          "required": True}]},
    {"type_id": "wh_inbound", "stable_alias": "wh_inbound",
     "entity_class": "data", "label": "Приходная накладная",
     "fields": [
         {"kind": "text", "alias": "wh_doc_no", "label": "Номер накладной",
          "identity": True, "required": True},
         {"kind": "date", "alias": "wh_doc_date", "label": "Дата"},
         {"kind": "text", "alias": "status_note", "label": "Статус"}]},
    {"type_id": "wh_outbound", "stable_alias": "wh_outbound",
     "entity_class": "data", "label": "Расходная накладная",
     "fields": [
         {"kind": "text", "alias": "wh_doc_no", "label": "Номер накладной",
          "identity": True, "required": True},
         {"kind": "date", "alias": "wh_doc_date", "label": "Дата"},
         {"kind": "text", "alias": "status_note", "label": "Статус"}]},
    {"type_id": "wh_inventory", "stable_alias": "wh_inventory",
     "entity_class": "data", "label": "Инвентаризация",
     "fields": [
         {"kind": "text", "alias": "wh_inv_no", "label": "Номер акта",
          "identity": True, "required": True},
         {"kind": "date", "alias": "wh_doc_date", "label": "Дата"},
         {"kind": "decimal", "alias": "wh_diff_qty", "label": "Расхождение"}]},
]

WH_SCALARS = [
    {"stable_alias": "wh_code", "scalar_kind": "text", "label": "Код склада",
     "participates_in_identity": True},
    {"stable_alias": "wh_zone_code", "scalar_kind": "text", "label": "Код зоны",
     "participates_in_identity": True},
    {"stable_alias": "wh_zone_kind", "scalar_kind": "text", "label": "Тип зоны"},
    {"stable_alias": "wh_sku", "scalar_kind": "text", "label": "Артикул",
     "participates_in_identity": True},
    {"stable_alias": "wh_unit", "scalar_kind": "text", "label": "Ед. измерения"},
    {"stable_alias": "wh_category", "scalar_kind": "text", "label": "Категория"},
    {"stable_alias": "wh_qty", "scalar_kind": "decimal", "label": "Количество"},
    {"stable_alias": "wh_batch", "scalar_kind": "text", "label": "Партия"},
    {"stable_alias": "wh_doc_no", "scalar_kind": "text", "label": "Номер документа",
     "participates_in_identity": True},
    {"stable_alias": "wh_doc_date", "scalar_kind": "date", "label": "Дата документа"},
    {"stable_alias": "wh_inv_no", "scalar_kind": "text", "label": "Номер акта",
     "participates_in_identity": True},
    {"stable_alias": "wh_diff_qty", "scalar_kind": "decimal", "label": "Расхождение"},
    {"stable_alias": "wh_capacity_m2", "scalar_kind": "decimal",
     "label": "Площадь, м²"},
]

WH_EDGES = [
    {"stable_alias": "wh_located_in", "is_edge": True, "domain": "wh_zone",
     "range": "wh_warehouse", "label": "Расположена в",
     "inverse_label": "Имеет зону"},
    {"stable_alias": "wh_stock_product", "is_edge": True, "domain": "wh_stock",
     "range": "wh_product", "label": "Остаток товара",
     "inverse_label": "Остатки"},
    {"stable_alias": "wh_stock_zone", "is_edge": True, "domain": "wh_stock",
     "range": "wh_zone", "label": "Хранится в зоне",
     "inverse_label": "Хранит"},
    {"stable_alias": "wh_supplied_by", "is_edge": True, "domain": "wh_product",
     "range": "legal_entity", "label": "Поставляется",
     "inverse_label": "Поставляет"},
    {"stable_alias": "wh_inbound_from", "is_edge": True, "domain": "wh_inbound",
     "range": "legal_entity", "label": "Поставщик",
     "inverse_label": "Поставки"},
    {"stable_alias": "wh_inbound_to", "is_edge": True, "domain": "wh_inbound",
     "range": "wh_warehouse", "label": "На склад",
     "inverse_label": "Приходы"},
    {"stable_alias": "wh_inbound_item", "is_edge": True, "domain": "wh_inbound",
     "range": "wh_product", "label": "Позиция прихода",
     "inverse_label": "Приходы товара"},
    {"stable_alias": "wh_outbound_to", "is_edge": True, "domain": "wh_outbound",
     "range": "legal_entity", "label": "Получатель",
     "inverse_label": "Отгрузки"},
    {"stable_alias": "wh_outbound_from", "is_edge": True, "domain": "wh_outbound",
     "range": "wh_warehouse", "label": "Со склада",
     "inverse_label": "Расходы"},
    {"stable_alias": "wh_outbound_item", "is_edge": True, "domain": "wh_outbound",
     "range": "wh_product", "label": "Позиция расхода",
     "inverse_label": "Отгрузки товара"},
    {"stable_alias": "wh_counted_in", "is_edge": True, "domain": "wh_inventory",
     "range": "wh_warehouse", "label": "Проведена на складе",
     "inverse_label": "Инвентаризации"},
    {"stable_alias": "wh_keeper", "is_edge": True, "domain": "person",
     "range": "wh_warehouse", "label": "Кладовщик",
     "inverse_label": "Склады"},
]


def main():
    tok = load_tokens()["admin"]

    # --- schema pipeline ---
    pub = rpc(tok, "schema.published", {})
    revs = pub.get("data") or []
    base = revs[0]
    base_id = str(base["revision_id"])
    print("base revision", base_id, base["semantic_version"])

    d = rpc(tok, "schema.draft_create",
            {"name": f"warehouse-test-{NONCE}", "base_revision": base_id})
    draft = d.get("data") or {}
    did = str(draft["revision_id"])
    etag = draft["etag"]
    types = list(draft.get("types") or [])
    preds = list(draft.get("predicates") or [])
    existing_t = {t.get("stable_alias") or t.get("type_id") for t in types}
    existing_p = {p.get("stable_alias") for p in preds}
    for t in WH_TYPES:
        if t["stable_alias"] not in existing_t:
            types.append(t)
    for p in WH_SCALARS + WH_EDGES:
        if p["stable_alias"] not in existing_p:
            preds.append(p)

    parts = [int(x) for x in base["semantic_version"].split(".")]
    ver = f"{parts[0]}.{parts[1] + 1}.0"
    u = rpc(tok, "schema.draft_update", {
        "id": did, "display_name": f"Склад ТЕСТ {NONCE}",
        "semantic_version": ver,
        "description": "Складская предметная область для MCP-прогона",
        "types": types, "predicates": preds,
        "changes": [{"kind": "add", "resource_kind": "schema",
                     "resource_id": "warehouse", "summary": "Складская схема ТЕСТ"}],
        "if_match": etag})
    if failed(u):
        print("draft_update FAILED", json.dumps(u, ensure_ascii=False)[:600])
        return 1
    etag = (u.get("data") or {}).get("etag") or etag
    print("draft updated ->", did)

    v = rpc(tok, "schema.draft_validate", {"id": did})
    print("validate:", json.dumps(v, ensure_ascii=False)[:300])
    im = rpc(tok, "schema.draft_impact", {"id": did})
    print("impact:", json.dumps(im, ensure_ascii=False)[:300])
    # impact bumps record version — refetch a fresh etag before publish
    g = rpc(tok, "schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag

    p = rpc(tok, "schema.draft_publish", {
        "id": did, "reason": "Публикация складской схемы ТЕСТ для прогона MCP",
        "if_match": etag})
    if failed(p):
        print("publish FAILED", json.dumps(p, ensure_ascii=False)[:600])
        return 1
    pub_rev = (p.get("data") or {}).get("revision_id", did)
    print("published revision", pub_rev)

    # --- objects ---
    ids = {}
    def ent(key, type_id, fields, **kw):
        r = rpc(tok, "entities.create",
                {"type_id": type_id, "fields": fields, **kw})
        pid = pub_id(r)
        if not pid:
            print(f"entities.create {key} FAILED",
                  json.dumps(r, ensure_ascii=False)[:400])
            return None
        ids[key] = pid
        print(f"  {key} -> {pid}")
        return pid

    ent("wh1", "wh_warehouse", {
        "wh_code": f"WH-{NONCE}-01", "name": "Склад ТЕСТ Центральный",
        "address": "г. Москва, ул. Складская, 1", "wh_capacity_m2": 5000})
    ent("wh2", "wh_warehouse", {
        "wh_code": f"WH-{NONCE}-02", "name": "Склад ТЕСТ Северный",
        "address": "г. СПб, Лендорожная, 12", "wh_capacity_m2": 2500})

    for code, kind, wh in [("RCV", "приёмка", "wh1"), ("STO-A", "хранение", "wh1"),
                           ("SHIP", "отгрузка", "wh1"), ("COLD", "холод", "wh2")]:
        ent(f"zone_{code.lower().replace('-','_')}", "wh_zone",
            {"wh_zone_code": f"Z-{NONCE}-{code}", "wh_zone_kind": kind})

    prods = [("p_nails", f"SKU-{NONCE}-001", "Гвозди строительные 100мм", "кг",
              "метизы"),
             ("p_paint", f"SKU-{NONCE}-002", "Краска белая водно-дисперсионная",
              "л", "ЛКМ"),
             ("p_board", f"SKU-{NONCE}-003", "Доска обрезная 50х150х6000", "м³",
              "пиломатериалы"),
             ("p_glue", f"SKU-{NONCE}-004", "Клей ПВА универсальный", "кг",
              "химия")]
    for key, sku, name, unit, cat in prods:
        ent(key, "wh_product", {"wh_sku": sku, "name": name,
                                "wh_unit": unit, "wh_category": cat})

    ent("sup", "legal_entity",
        {"name": "ООО ТЕСТ Поставка", "inn": f"77{int(NONCE[:8],16)%100000000:08d}"})
    ent("cust", "legal_entity",
        {"name": "АО ТЕСТ Покупатель", "inn": f"78{int(NONCE[:8],16)%100000000:08d}"})
    ent("keeper", "person",
        {"full_name": "Сидоров Кладовщик Тестович", "inn": f"77{int(NONCE[:8],16)%10000000000:010d}"})

    ent("stk_nails", "wh_stock", {"wh_qty": 1500, "wh_batch": "B-1"})
    ent("stk_paint", "wh_stock", {"wh_qty": 320, "wh_batch": "B-2"})
    ent("stk_board", "wh_stock", {"wh_qty": 12.5, "wh_batch": "B-3"})

    ent("inb1", "wh_inbound", {"wh_doc_no": f"INB-{NONCE}-001",
                               "wh_doc_date": "2026-01-10",
                               "status_note": "проведена"})
    ent("inb2", "wh_inbound", {"wh_doc_no": f"INB-{NONCE}-002",
                               "wh_doc_date": "2026-01-15",
                               "status_note": "черновик"})
    ent("out1", "wh_outbound", {"wh_doc_no": f"OUT-{NONCE}-001",
                                "wh_doc_date": "2026-01-20",
                                "status_note": "зарезервирована"})
    ent("inv1", "wh_inventory", {"wh_inv_no": f"INV-{NONCE}-001",
                                 "wh_doc_date": "2026-02-01",
                                 "wh_diff_qty": -3})

    # --- relations ---
    def rel(src_key, predicate, dst_key):
        r = rpc(tok, "relations.create", {
            "id": ids[src_key], "predicate": predicate,
            "target_entity_id": ids[dst_key]})
        ok = not failed(r)
        if not ok:
            print(f"  rel {src_key}-[{predicate}]->{dst_key} FAILED",
                  json.dumps(r, ensure_ascii=False)[:300])
        return ok

    print("relations:")
    for z in ["zone_sto_a", "zone_ship"]:
        rel(z, "wh_located_in", "wh1")
    rel("zone_cold", "wh_located_in", "wh2")
    rel("stk_nails", "wh_stock_product", "p_nails")
    rel("stk_nails", "wh_stock_zone", "zone_sto_a")
    # stk_paint edges are intentionally left to the WH3 test cases
    rel("stk_board", "wh_stock_product", "p_board")
    rel("stk_board", "wh_stock_zone", "zone_rcv")
    rel("p_nails", "wh_supplied_by", "sup")
    rel("p_paint", "wh_supplied_by", "sup")
    rel("p_board", "wh_supplied_by", "sup")
    rel("inb1", "wh_inbound_from", "sup")
    rel("inb1", "wh_inbound_to", "wh1")
    rel("inb1", "wh_inbound_item", "p_nails")
    rel("inb1", "wh_inbound_item", "p_paint")
    rel("inb2", "wh_inbound_from", "sup")
    rel("inb2", "wh_inbound_to", "wh1")
    rel("inb2", "wh_inbound_item", "p_glue")
    rel("out1", "wh_outbound_to", "cust")
    rel("out1", "wh_outbound_from", "wh1")
    rel("out1", "wh_outbound_item", "p_nails")
    rel("out1", "wh_outbound_item", "p_board")
    rel("inv1", "wh_counted_in", "wh1")
    rel("keeper", "wh_keeper", "wh1")

    ids["_nonce"] = NONCE
    ids["_published_revision"] = str(pub_rev)
    with open(os.path.join(HERE, ".wh_objects.json"), "w", encoding="utf-8") as f:
        json.dump(ids, f, ensure_ascii=False, indent=1)
    print("objects saved:", len(ids))
    return 0


if __name__ == "__main__":
    sys.exit(main())
