"""Fresh-DB warehouse provisioning — EVERYTHING through the MCP endpoint.

The DB was wiped (empty schema catalog), so provision.py / provision_v2.py
cannot run (they need a published base revision). This script:

  1. schema.draft_create (no base_revision) — empty catalog draft.
  2. schema.draft_update — full catalog: base types (person, legal_entity),
     shared scalars, warehouse v1 + v2 types/scalars/edges.
  3. draft_validate -> draft_impact -> draft_get (fresh etag) -> draft_publish.
  4. entities.create — realistic warehouse domain dataset.
  5. relations.create — edges per the warehouse model.

Persists public ids to .wh_objects.json.
"""
import json
import os
import sys
import time
import urllib.request
import urllib.error

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from provision import WH_TYPES, WH_SCALARS, WH_EDGES          # noqa: E402
from provision_v2 import NEW_TYPES, NEW_SCALARS, NEW_EDGES    # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
MCP = os.environ.get("ENGINER_MCP", "http://localhost:8020/mcp")
TOK = json.load(open(os.path.join(HERE, ".wh_tokens.json")))["admin"]
HDR = {"Content-Type": "application/json",
       "Accept": "application/json, text/event-stream",
       "Authorization": "Bearer " + TOK}
_idn = [0]

# --- base types / shared scalars missing from the wiped catalog -------------
BASE_TYPES = [
    {"type_id": "person", "stable_alias": "person",
     "entity_class": "data", "label": "Физическое лицо",
     "fields": [
         {"kind": "text", "alias": "full_name", "label": "ФИО",
          "identity": True, "required": True},
         {"kind": "text", "alias": "inn", "label": "ИНН"},
         {"kind": "text", "alias": "wh_phone", "label": "Телефон"}]},
    {"type_id": "legal_entity", "stable_alias": "legal_entity",
     "entity_class": "data", "label": "Юридическое лицо",
     "fields": [
         {"kind": "text", "alias": "inn", "label": "ИНН",
          "identity": True, "required": True},
         {"kind": "text", "alias": "name", "label": "Наименование",
          "required": True},
         {"kind": "text", "alias": "wh_phone", "label": "Телефон"},
         {"kind": "text", "alias": "wh_contact", "label": "Контактное лицо"}]},
]

BASE_SCALARS = [
    {"stable_alias": "name", "scalar_kind": "text", "label": "Наименование"},
    {"stable_alias": "full_name", "scalar_kind": "text", "label": "ФИО",
     "participates_in_identity": True},
    {"stable_alias": "inn", "scalar_kind": "text", "label": "ИНН",
     "participates_in_identity": True},
    {"stable_alias": "address", "scalar_kind": "text", "label": "Адрес"},
    {"stable_alias": "status_note", "scalar_kind": "text", "label": "Статус"},
]

TYPES = BASE_TYPES + WH_TYPES + NEW_TYPES
PREDICATES = BASE_SCALARS + WH_SCALARS + NEW_SCALARS + WH_EDGES + NEW_EDGES


def rpc_raw(method, params=None):
    _idn[0] += 1
    body = {"jsonrpc": "2.0", "id": _idn[0], "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(MCP, data=json.dumps(body).encode(),
                                 headers=HDR)
    try:
        raw = urllib.request.urlopen(req, timeout=60).read().decode()
    except urllib.error.HTTPError as e:
        return {"_http": e.code, "_body": e.read()[:600].decode("utf-8",
                                                               "replace")}
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
        out = {"_text": txt[:3000]}
    if r.get("isError") and isinstance(out, dict):
        out["_isError"] = True
    elif r.get("isError"):
        out = {"_isError": True, "_text": txt[:3000]}
    return out


def failed(resp):
    return bool(resp.get("_rpc_err") or resp.get("_http")
                or resp.get("_isError") or resp.get("error"))


def pub_id(resp):
    d = resp.get("data") or {}
    return d.get("public_id") or d.get("id") or d.get("entity_id")


def main():
    rpc_raw("initialize", {"protocolVersion": "2025-06-18",
                           "capabilities": {},
                           "clientInfo": {"name": "provision-fresh",
                                          "version": "1.0"}})
    rpc_raw("notifications/initialized")

    # --- schema pipeline -------------------------------------------------
    pub = call("schema.published")
    if (pub.get("data") or []):
        print("WARNING: a published revision already exists:",
              json.dumps(pub["data"], ensure_ascii=False)[:300])

    d = call("schema.draft_create", {"name": "warehouse-fresh"})
    if failed(d):
        print("draft_create FAILED", json.dumps(d, ensure_ascii=False)[:600])
        return 1
    draft = d["data"]
    did = str(draft["revision_id"])
    etag = draft["etag"]
    print("draft", did, "etag", etag)

    u = call("schema.draft_update", {
        "id": did,
        "display_name": "СКР-база + складской домен (fresh)",
        "semantic_version": "1.0.0",
        "description": "Fresh provisioning: person/legal_entity + warehouse "
                       "management domain (zones, stock, in/out docs, cargo, "
                       "waybills, payments, expenses, holds, inventory)",
        "types": TYPES,
        "predicates": PREDICATES,
        "changes": [{"kind": "add", "resource_kind": "schema",
                     "resource_id": "warehouse-fresh",
                     "summary": "Полный складской домен с нуля"}],
        "if_match": etag})
    if failed(u):
        print("draft_update FAILED", json.dumps(u, ensure_ascii=False)[:1200])
        return 1
    etag = (u.get("data") or {}).get("etag") or etag
    print("draft updated:", len(TYPES), "types,", len(PREDICATES),
          "predicates")

    v = call("schema.draft_validate", {"id": did})
    print("validate:", json.dumps(v, ensure_ascii=False)[:800])
    im = call("schema.draft_impact", {"id": did})
    print("impact:", json.dumps(im, ensure_ascii=False)[:500])
    g = call("schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag

    p = call("schema.draft_publish", {
        "id": did, "reason": "Initial schema publish on wiped DB",
        "if_match": etag})
    if failed(p):
        print("publish FAILED", json.dumps(p, ensure_ascii=False)[:1200])
        return 1
    pub_rev = str((p.get("data") or {}).get("revision_id", did))
    print("published revision", pub_rev)
    with open(os.path.join(HERE, ".published_rev.json"), "w") as f:
        json.dump({"revision": pub_rev}, f)

    # --- entities ---------------------------------------------------------
    ids = {}

    def ent(key, type_id, fields):
        r = call("entities.create", {"type_id": type_id, "fields": fields})
        pid = pub_id(r)
        if not pid:
            print(f"  entities.create {key} FAILED",
                  json.dumps(r, ensure_ascii=False)[:500])
            return None
        ids[key] = pid
        print(f"  {key} ({type_id}) -> {pid}")
        return pid

    print("entities:")
    # warehouses
    ent("wh_msk", "wh_warehouse", {
        "wh_code": "WH-MSK-01", "name": "Склад «Северный» (Москва)",
        "address": "г. Москва, Дмитровское ш., 71",
        "wh_capacity_m2": 12000})
    ent("wh_spb", "wh_warehouse", {
        "wh_code": "WH-SPB-01", "name": "Склад «Нева» (Санкт-Петербург)",
        "address": "г. Санкт-Петербург, Индустриальный пр., 44",
        "wh_capacity_m2": 6500})

    # zones
    zones = [("z_rcv",  "Z-RCV-01",  "приёмка",        800),
             ("z_stoa", "Z-STO-A",   "хранение",       4200),
             ("z_stob", "Z-STO-B",   "хранение",       3900),
             ("z_cold", "Z-COLD-01", "холодильная",    600),
             ("z_ship", "Z-SHIP-01", "отгрузка",       900),
             ("z_spb1", "Z-SPB-STO", "хранение (СПб)", 5000)]
    for key, code, kind, cap in zones:
        ent(key, "wh_zone", {"wh_zone_code": code, "wh_zone_kind": kind,
                             "wh_capacity_m2": cap})

    # people
    ent("keeper", "person", {"full_name": "Сидоров Иван Петрович",
                             "inn": "771234567890",
                             "wh_phone": "+7 916 555-01-23"})
    ent("keeper2", "person", {"full_name": "Кузнецова Ольга Андреевна",
                              "inn": "780987654321",
                              "wh_phone": "+7 921 555-44-55"})

    # counterparties: legal_entity (base type) + typed supplier/client/contractor
    ent("le_sup", "legal_entity", {
        "name": "ООО «МетизСнаб»", "inn": "7714012345",
        "wh_phone": "+7 495 111-22-33", "wh_contact": "Громов А.В."})
    ent("le_cust", "legal_entity", {
        "name": "АО «СтройМонтаж»", "inn": "7805567890",
        "wh_phone": "+7 812 777-88-99", "wh_contact": "Орлова Н.С."})
    ent("sup1", "wh_supplier", {
        "wh_partner_code": "SUP-0001", "name": "ООО «МетизСнаб»",
        "inn": "7714012345", "wh_phone": "+7 495 111-22-33",
        "wh_contact": "Громов А.В."})
    ent("sup2", "wh_supplier", {
        "wh_partner_code": "SUP-0002", "name": "ЗАО «Лакокраска Опт»",
        "inn": "5029876543", "wh_phone": "+7 498 222-33-44",
        "wh_contact": "Белкин П.И."})
    ent("cli1", "wh_client", {
        "wh_partner_code": "CLI-0001", "name": "АО «СтройМонтаж»",
        "inn": "7805567890", "wh_phone": "+7 812 777-88-99",
        "wh_contact": "Орлова Н.С."})
    ent("cli2", "wh_client", {
        "wh_partner_code": "CLI-0002", "name": "ИП Воробьёв Д.М.",
        "inn": "771599887766", "wh_phone": "+7 903 555-66-77",
        "wh_contact": "Воробьёв Д.М."})
    ent("carrier", "wh_contractor", {
        "wh_partner_code": "CON-0001", "name": "ООО «ТрансЛогист»",
        "inn": "7722114455", "wh_phone": "+7 495 999-00-11",
        "wh_contact": "Жуков Р.Э."})

    # products
    prods = [("p_nails", "SKU-10001", "Гвозди строительные 100мм", "кг",
              "метизы"),
             ("p_paint", "SKU-10002", "Краска белая водно-дисперсионная",
              "л", "ЛКМ"),
             ("p_board", "SKU-10003", "Доска обрезная 50х150х6000", "м³",
              "пиломатериалы"),
             ("p_glue",  "SKU-10004", "Клей ПВА универсальный", "кг",
              "химия"),
             ("p_milk",  "SKU-10005", "Молоко пастеризованное 3.2%", "л",
              "продукты")]
    for key, sku, name, unit, cat in prods:
        ent(key, "wh_product", {"wh_sku": sku, "name": name,
                                "wh_unit": unit, "wh_category": cat})

    # cargo
    for key, no, name, w, v_ in [
            ("cg1", "CRG-2026-001", "Паллета: гвозди (40 кор.)",
             820, 1.2),
            ("cg2", "CRG-2026-002", "Паллета: краска (60 банок)",
             540, 1.1),
            ("cg3", "CRG-2026-003", "Термоконтейнер: молоко",
             300, 0.9)]:
        ent(key, "wh_cargo", {"wh_cargo_no": no, "name": name,
                              "wh_weight_kg": w, "wh_volume_m3": v_})

    # stock
    for key, qty, batch in [("stk_nails", 1500, "B-2026-011"),
                            ("stk_paint", 320, "B-2026-012"),
                            ("stk_board", 12.5, "B-2026-013"),
                            ("stk_milk", 480, "B-2026-021")]:
        ent(key, "wh_stock", {"wh_qty": qty, "wh_batch": batch})

    # inbound / outbound
    ent("inb1", "wh_inbound", {"wh_doc_no": "INB-2026-0001",
                               "wh_doc_date": "2026-10-02",
                               "status_note": "проведена"})
    ent("inb2", "wh_inbound", {"wh_doc_no": "INB-2026-0002",
                               "wh_doc_date": "2026-10-12",
                               "status_note": "проведена"})
    ent("out1", "wh_outbound", {"wh_doc_no": "OUT-2026-0001",
                                "wh_doc_date": "2026-10-15",
                                "status_note": "отгружена"})
    ent("out2", "wh_outbound", {"wh_doc_no": "OUT-2026-0002",
                                "wh_doc_date": "2026-10-18",
                                "status_note": "зарезервирована"})

    # waybills
    ent("wb1", "wh_waybill", {"wh_waybill_no": "WB-2026-0101",
                              "wh_doc_date": "2026-10-02",
                              "wh_amount": 184500, "wh_currency": "RUB",
                              "wh_due_date": "2026-11-02",
                              "status_note": "оплачена"})
    ent("wb2", "wh_waybill", {"wh_waybill_no": "WB-2026-0102",
                              "wh_doc_date": "2026-10-15",
                              "wh_amount": 96200, "wh_currency": "RUB",
                              "wh_due_date": "2026-11-15",
                              "status_note": "выставлена"})
    ent("wb3", "wh_waybill", {"wh_waybill_no": "WB-2026-0103",
                              "wh_doc_date": "2026-10-18",
                              "wh_amount": 41700, "wh_currency": "RUB",
                              "wh_due_date": "2026-11-18",
                              "status_note": "черновик"})

    # payments
    ent("pay1", "wh_payment", {"wh_payment_no": "PAY-2026-0501",
                               "wh_doc_date": "2026-10-05",
                               "wh_amount": 184500, "wh_currency": "RUB",
                               "wh_direction": "outgoing"})
    ent("pay2", "wh_payment", {"wh_payment_no": "PAY-2026-0502",
                               "wh_doc_date": "2026-10-17",
                               "wh_amount": 50000, "wh_currency": "RUB",
                               "wh_direction": "incoming"})
    ent("pay3", "wh_payment", {"wh_payment_no": "PAY-2026-0503",
                               "wh_doc_date": "2026-10-18",
                               "wh_amount": 46200, "wh_currency": "RUB",
                               "wh_direction": "incoming"})

    # expenses
    ent("exp1", "wh_expense", {"wh_expense_no": "EXP-2026-0901",
                               "wh_doc_date": "2026-10-01",
                               "wh_amount": 210000,
                               "wh_category": "аренда"})
    ent("exp2", "wh_expense", {"wh_expense_no": "EXP-2026-0902",
                               "wh_doc_date": "2026-10-10",
                               "wh_amount": 38500,
                               "wh_category": "коммунальные услуги"})

    # hold + inventory
    ent("hold1", "wh_hold", {"wh_hold_no": "HLD-2026-0001",
                             "wh_amount": 12000,
                             "wh_reason": "претензия по качеству поставки",
                             "wh_until": "2026-11-30"})
    ent("inv1", "wh_inventory", {"wh_inv_no": "INV-2026-0007",
                                 "wh_doc_date": "2026-10-14",
                                 "wh_diff_qty": -3})

    # --- relations --------------------------------------------------------
    def rel(src, pred, dst):
        if src not in ids or dst not in ids:
            print(f"  rel {src}-[{pred}]->{dst} SKIPPED (missing id)")
            return
        r = call("relations.create", {"id": ids[src], "predicate": pred,
                                      "target_entity_id": ids[dst]})
        if failed(r):
            print(f"  rel {src}-[{pred}]->{dst} FAILED",
                  json.dumps(r, ensure_ascii=False)[:400])

    print("relations:")
    rel("z_rcv", "wh_located_in", "wh_msk")
    rel("z_stoa", "wh_located_in", "wh_msk")
    rel("z_stob", "wh_located_in", "wh_msk")
    rel("z_cold", "wh_located_in", "wh_msk")
    rel("z_ship", "wh_located_in", "wh_msk")
    rel("z_spb1", "wh_located_in", "wh_spb")

    rel("keeper", "wh_keeper", "wh_msk")
    rel("keeper2", "wh_keeper", "wh_spb")

    rel("stk_nails", "wh_stock_product", "p_nails")
    rel("stk_nails", "wh_stock_zone", "z_stoa")
    rel("stk_paint", "wh_stock_product", "p_paint")
    rel("stk_paint", "wh_stock_zone", "z_stob")
    rel("stk_board", "wh_stock_product", "p_board")
    rel("stk_board", "wh_stock_zone", "z_stob")
    rel("stk_milk", "wh_stock_product", "p_milk")
    rel("stk_milk", "wh_stock_zone", "z_cold")

    rel("p_nails", "wh_supplied_by", "le_sup")
    rel("p_nails", "wh_product_supplier", "sup1")
    rel("p_paint", "wh_supplied_by", "le_sup")
    rel("p_paint", "wh_product_supplier", "sup2")
    rel("p_board", "wh_product_supplier", "sup1")
    rel("p_glue", "wh_product_supplier", "sup2")
    rel("p_milk", "wh_product_supplier", "sup2")

    rel("inb1", "wh_inbound_to", "wh_msk")
    rel("inb1", "wh_inbound_from", "le_sup")
    rel("inb1", "wh_inbound_supplier", "sup1")
    rel("inb1", "wh_inbound_item", "p_nails")
    rel("inb1", "wh_inbound_item", "p_paint")
    rel("inb2", "wh_inbound_to", "wh_msk")
    rel("inb2", "wh_inbound_supplier", "sup2")
    rel("inb2", "wh_inbound_item", "p_glue")
    rel("inb2", "wh_inbound_item", "p_milk")

    rel("out1", "wh_outbound_from", "wh_msk")
    rel("out1", "wh_outbound_to", "le_cust")
    rel("out1", "wh_outbound_client", "cli1")
    rel("out1", "wh_outbound_item", "p_nails")
    rel("out1", "wh_outbound_item", "p_board")
    rel("out2", "wh_outbound_from", "wh_msk")
    rel("out2", "wh_outbound_client", "cli2")
    rel("out2", "wh_outbound_item", "p_milk")

    rel("cg1", "wh_cargo_in", "z_stoa")
    rel("cg1", "wh_cargo_on", "wb1")
    rel("cg2", "wh_cargo_in", "z_stob")
    rel("cg2", "wh_cargo_on", "wb1")
    rel("cg3", "wh_cargo_in", "z_cold")
    rel("cg3", "wh_cargo_on", "wb3")

    rel("wb1", "wh_waybill_inbound", "inb1")
    rel("wb1", "wh_waybill_carrier", "carrier")
    rel("wb2", "wh_waybill_outbound", "out1")
    rel("wb2", "wh_waybill_carrier", "carrier")
    rel("wb3", "wh_waybill_outbound", "out2")
    rel("wb3", "wh_waybill_carrier", "carrier")

    rel("pay1", "wh_payment_for", "wb1")
    rel("pay1", "wh_payment_supplier", "sup1")
    rel("pay2", "wh_payment_for", "wb2")
    rel("pay2", "wh_payment_client", "cli1")
    rel("pay3", "wh_payment_for", "wb2")
    rel("pay3", "wh_payment_client", "cli1")

    rel("exp1", "wh_expense_wh", "wh_msk")
    rel("exp2", "wh_expense_wh", "wh_msk")

    rel("hold1", "wh_hold_on", "pay3")
    rel("hold1", "wh_hold_party", "cli1")

    rel("inv1", "wh_counted_in", "wh_msk")

    ids["_published_revision"] = pub_rev
    with open(os.path.join(HERE, ".wh_objects.json"), "w",
              encoding="utf-8") as f:
        json.dump(ids, f, ensure_ascii=False, indent=1)
    print("objects saved:", len(ids) - 1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
