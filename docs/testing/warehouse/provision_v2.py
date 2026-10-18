"""Warehouse «ТЕСТ» v2 — расширение схемы до полного домена управляющего.

Добавляет типы: поставщик, клиент, подрядчик, груз, накладная, оплата,
издержка, удержание — и связи между ними, поставками, отгрузками, зонами.
Всё через MCP: draft_create -> draft_update -> validate -> impact -> publish.
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

NEW_TYPES = [
    {"type_id": "wh_supplier", "stable_alias": "wh_supplier",
     "entity_class": "data", "label": "Поставщик",
     "fields": [
         {"kind": "text", "alias": "wh_partner_code", "label": "Код контрагента",
          "identity": True, "required": True},
         {"kind": "text", "alias": "name", "label": "Наименование",
          "required": True},
         {"kind": "text", "alias": "inn", "label": "ИНН"},
         {"kind": "text", "alias": "wh_phone", "label": "Телефон"},
         {"kind": "text", "alias": "wh_contact", "label": "Контактное лицо"}]},
    {"type_id": "wh_client", "stable_alias": "wh_client",
     "entity_class": "data", "label": "Клиент",
     "fields": [
         {"kind": "text", "alias": "wh_partner_code", "label": "Код контрагента",
          "identity": True, "required": True},
         {"kind": "text", "alias": "name", "label": "Наименование",
          "required": True},
         {"kind": "text", "alias": "inn", "label": "ИНН"},
         {"kind": "text", "alias": "wh_phone", "label": "Телефон"},
         {"kind": "text", "alias": "wh_contact", "label": "Контактное лицо"}]},
    {"type_id": "wh_contractor", "stable_alias": "wh_contractor",
     "entity_class": "data", "label": "Подрядчик",
     "fields": [
         {"kind": "text", "alias": "wh_partner_code", "label": "Код контрагента",
          "identity": True, "required": True},
         {"kind": "text", "alias": "name", "label": "Наименование",
          "required": True},
         {"kind": "text", "alias": "inn", "label": "ИНН"},
         {"kind": "text", "alias": "wh_phone", "label": "Телефон"},
         {"kind": "text", "alias": "wh_contact", "label": "Контактное лицо"}]},
    {"type_id": "wh_cargo", "stable_alias": "wh_cargo",
     "entity_class": "data", "label": "Груз (место)",
     "fields": [
         {"kind": "text", "alias": "wh_cargo_no", "label": "Номер груза",
          "identity": True, "required": True},
         {"kind": "text", "alias": "name", "label": "Описание",
          "required": True},
         {"kind": "decimal", "alias": "wh_weight_kg", "label": "Вес, кг"},
         {"kind": "decimal", "alias": "wh_volume_m3", "label": "Объём, м³"}]},
    {"type_id": "wh_waybill", "stable_alias": "wh_waybill",
     "entity_class": "data", "label": "Накладная",
     "fields": [
         {"kind": "text", "alias": "wh_waybill_no", "label": "Номер накладной",
          "identity": True, "required": True},
         {"kind": "date", "alias": "wh_doc_date", "label": "Дата"},
         {"kind": "decimal", "alias": "wh_amount", "label": "Сумма"},
         {"kind": "text", "alias": "wh_currency", "label": "Валюта"},
         {"kind": "date", "alias": "wh_due_date", "label": "Срок оплаты"},
         {"kind": "text", "alias": "status_note", "label": "Статус"}]},
    {"type_id": "wh_payment", "stable_alias": "wh_payment",
     "entity_class": "data", "label": "Оплата",
     "fields": [
         {"kind": "text", "alias": "wh_payment_no", "label": "Номер платежа",
          "identity": True, "required": True},
         {"kind": "date", "alias": "wh_doc_date", "label": "Дата"},
         {"kind": "decimal", "alias": "wh_amount", "label": "Сумма",
          "required": True},
         {"kind": "text", "alias": "wh_currency", "label": "Валюта"},
         {"kind": "text", "alias": "wh_direction", "label": "Направление"}]},
    {"type_id": "wh_expense", "stable_alias": "wh_expense",
     "entity_class": "data", "label": "Издержка",
     "fields": [
         {"kind": "text", "alias": "wh_expense_no", "label": "Номер расхода",
          "identity": True, "required": True},
         {"kind": "date", "alias": "wh_doc_date", "label": "Дата"},
         {"kind": "decimal", "alias": "wh_amount", "label": "Сумма",
          "required": True},
         {"kind": "text", "alias": "wh_category", "label": "Категория"}]},
    {"type_id": "wh_hold", "stable_alias": "wh_hold",
     "entity_class": "data", "label": "Удержание",
     "fields": [
         {"kind": "text", "alias": "wh_hold_no", "label": "Номер удержания",
          "identity": True, "required": True},
         {"kind": "decimal", "alias": "wh_amount", "label": "Сумма",
          "required": True},
         {"kind": "text", "alias": "wh_reason", "label": "Причина"},
         {"kind": "date", "alias": "wh_until", "label": "Удерживать до"}]},
]

NEW_SCALARS = [
    {"stable_alias": "wh_partner_code", "scalar_kind": "text",
     "label": "Код контрагента", "participates_in_identity": True},
    {"stable_alias": "wh_phone", "scalar_kind": "text", "label": "Телефон"},
    {"stable_alias": "wh_contact", "scalar_kind": "text",
     "label": "Контактное лицо"},
    {"stable_alias": "wh_cargo_no", "scalar_kind": "text",
     "label": "Номер груза", "participates_in_identity": True},
    {"stable_alias": "wh_weight_kg", "scalar_kind": "decimal",
     "label": "Вес, кг"},
    {"stable_alias": "wh_volume_m3", "scalar_kind": "decimal",
     "label": "Объём, м³"},
    {"stable_alias": "wh_waybill_no", "scalar_kind": "text",
     "label": "Номер накладной", "participates_in_identity": True},
    {"stable_alias": "wh_amount", "scalar_kind": "decimal", "label": "Сумма"},
    {"stable_alias": "wh_currency", "scalar_kind": "text", "label": "Валюта"},
    {"stable_alias": "wh_due_date", "scalar_kind": "date",
     "label": "Срок оплаты"},
    {"stable_alias": "wh_direction", "scalar_kind": "text",
     "label": "Направление платежа"},
    {"stable_alias": "wh_payment_no", "scalar_kind": "text",
     "label": "Номер платежа", "participates_in_identity": True},
    {"stable_alias": "wh_expense_no", "scalar_kind": "text",
     "label": "Номер расхода", "participates_in_identity": True},
    {"stable_alias": "wh_hold_no", "scalar_kind": "text",
     "label": "Номер удержания", "participates_in_identity": True},
    {"stable_alias": "wh_reason", "scalar_kind": "text", "label": "Причина"},
    {"stable_alias": "wh_until", "scalar_kind": "date",
     "label": "Удерживать до"},
]

NEW_EDGES = [
    {"stable_alias": "wh_inbound_supplier", "is_edge": True,
     "domain": "wh_inbound", "range": "wh_supplier",
     "label": "Поставщик", "inverse_label": "Поставки"},
    {"stable_alias": "wh_outbound_client", "is_edge": True,
     "domain": "wh_outbound", "range": "wh_client",
     "label": "Клиент", "inverse_label": "Отгрузки"},
    {"stable_alias": "wh_waybill_inbound", "is_edge": True,
     "domain": "wh_waybill", "range": "wh_inbound",
     "label": "Накладная на приход", "inverse_label": "Накладные"},
    {"stable_alias": "wh_waybill_outbound", "is_edge": True,
     "domain": "wh_waybill", "range": "wh_outbound",
     "label": "Накладная на отгрузку", "inverse_label": "Накладные"},
    {"stable_alias": "wh_waybill_carrier", "is_edge": True,
     "domain": "wh_waybill", "range": "wh_contractor",
     "label": "Перевозчик", "inverse_label": "Перевозки"},
    {"stable_alias": "wh_cargo_in", "is_edge": True,
     "domain": "wh_cargo", "range": "wh_zone",
     "label": "Размещён в зоне", "inverse_label": "Грузы"},
    {"stable_alias": "wh_cargo_on", "is_edge": True,
     "domain": "wh_cargo", "range": "wh_waybill",
     "label": "Груз по накладной", "inverse_label": "Грузы"},
    {"stable_alias": "wh_payment_for", "is_edge": True,
     "domain": "wh_payment", "range": "wh_waybill",
     "label": "Оплата по накладной", "inverse_label": "Оплаты"},
    {"stable_alias": "wh_payment_client", "is_edge": True,
     "domain": "wh_payment", "range": "wh_client",
     "label": "Плательщик-клиент", "inverse_label": "Оплаты"},
    {"stable_alias": "wh_payment_supplier", "is_edge": True,
     "domain": "wh_payment", "range": "wh_supplier",
     "label": "Получатель-поставщик", "inverse_label": "Оплаты"},
    {"stable_alias": "wh_expense_wh", "is_edge": True,
     "domain": "wh_expense", "range": "wh_warehouse",
     "label": "Издержка склада", "inverse_label": "Издержки"},
    {"stable_alias": "wh_hold_on", "is_edge": True,
     "domain": "wh_hold", "range": "wh_payment",
     "label": "Удержание на платёж", "inverse_label": "Удержания"},
    {"stable_alias": "wh_hold_party", "is_edge": True,
     "domain": "wh_hold", "range": "wh_client",
     "label": "Удержание клиента", "inverse_label": "Удержания"},
    {"stable_alias": "wh_product_supplier", "is_edge": True,
     "domain": "wh_product", "range": "wh_supplier",
     "label": "Поставщик товара", "inverse_label": "Товары"},
]


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


def main():
    tok = load_tokens()["admin"]

    pub = rpc(tok, "schema.published", {})
    revs = pub.get("data") or []
    base = revs[0]
    base_id = str(base["revision_id"])
    print("base revision", base_id, base["semantic_version"])

    d = rpc(tok, "schema.draft_create",
            {"name": f"warehouse-mgr-{NONCE}", "base_revision": base_id})
    draft = d.get("data") or {}
    did = str(draft["revision_id"])
    etag = draft["etag"]
    types = list(draft.get("types") or [])
    preds = list(draft.get("predicates") or [])
    existing_t = {t.get("stable_alias") or t.get("type_id") for t in types}
    existing_p = {p.get("stable_alias") for p in preds}
    added_t = added_p = 0
    for t in NEW_TYPES:
        if t["stable_alias"] not in existing_t:
            types.append(t); added_t += 1
    for p in NEW_SCALARS + NEW_EDGES:
        if p["stable_alias"] not in existing_p:
            preds.append(p); added_p += 1
    print(f"adding {added_t} types, {added_p} predicates")

    parts = [int(x) for x in base["semantic_version"].split(".")]
    ver = f"{parts[0]}.{parts[1] + 1}.0"
    u = rpc(tok, "schema.draft_update", {
        "id": did, "display_name": f"Склад ТЕСТ управляющий {NONCE}",
        "semantic_version": ver,
        "description": "Расширенный складской домен: контрагенты, грузы, "
                       "накладные, оплаты, издержки, удержания",
        "types": types, "predicates": preds,
        "changes": [{"kind": "add", "resource_kind": "schema",
                     "resource_id": "warehouse-v2",
                     "summary": "Полный домен управляющего складом"}],
        "if_match": etag})
    if failed(u):
        print("draft_update FAILED", json.dumps(u, ensure_ascii=False)[:800])
        return 1
    etag = (u.get("data") or {}).get("etag") or etag

    v = rpc(tok, "schema.draft_validate", {"id": did})
    print("validate:", json.dumps(v, ensure_ascii=False)[:300])
    im = rpc(tok, "schema.draft_impact", {"id": did})
    print("impact:", json.dumps(im, ensure_ascii=False)[:300])
    g = rpc(tok, "schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag

    p = rpc(tok, "schema.draft_publish", {
        "id": did, "reason": "Публикация расширенной складской схемы",
        "if_match": etag})
    if failed(p):
        print("publish FAILED", json.dumps(p, ensure_ascii=False)[:800])
        return 1
    pub_rev = (p.get("data") or {}).get("revision_id", did)
    print("published revision", pub_rev)

    with open(os.path.join(HERE, ".published_rev.json"), "w") as f:
        json.dump({"revision": pub_rev, "nonce": NONCE}, f)
    return 0


if __name__ == "__main__":
    sys.exit(main())
