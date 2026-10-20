"""Schema-only provisioning on a wiped DB — publishes the warehouse catalog
(base + v1 + v2) PLUS a contract/finance extension, then stops. No entities.

Extra types: wh_contract (supply / storage / rent / leasing / insurance /
credit agreements with legal entities).
Extra edges: contract parties/warehouse, inbound/outbound under contract,
payments against contracts, stock under storage contract, outbound carrier
and human receiver.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import provision_fresh as pf                      # noqa: E402

EXTRA_TYPES = [
    {"type_id": "wh_contract", "stable_alias": "wh_contract",
     "entity_class": "data", "label": "Договор",
     "fields": [
         {"kind": "text", "alias": "wh_contract_no", "label": "Номер договора",
          "identity": True, "required": True},
         {"kind": "text", "alias": "wh_contract_kind", "label": "Вид договора",
          "required": True},
         {"kind": "text", "alias": "name", "label": "Наименование"},
         {"kind": "date", "alias": "wh_doc_date", "label": "Дата"},
         {"kind": "date", "alias": "wh_until", "label": "Действует до"},
         {"kind": "decimal", "alias": "wh_amount", "label": "Сумма"},
         {"kind": "text", "alias": "wh_currency", "label": "Валюта"},
         {"kind": "text", "alias": "status_note", "label": "Статус"}]},
]

EXTRA_PREDICATES = [
    {"stable_alias": "wh_contract_no", "scalar_kind": "text",
     "label": "Номер договора", "participates_in_identity": True},
    {"stable_alias": "wh_contract_kind", "scalar_kind": "text",
     "label": "Вид договора"},
    {"stable_alias": "wh_contract_party", "is_edge": True,
     "domain": "wh_contract", "range": "legal_entity",
     "label": "Контрагент по договору"},
    {"stable_alias": "wh_contract_wh", "is_edge": True,
     "domain": "wh_contract", "range": "wh_warehouse",
     "label": "Договор по складу"},
    {"stable_alias": "wh_inbound_contract", "is_edge": True,
     "domain": "wh_inbound", "range": "wh_contract",
     "label": "Поставка по договору"},
    {"stable_alias": "wh_outbound_contract", "is_edge": True,
     "domain": "wh_outbound", "range": "wh_contract",
     "label": "Отгрузка по договору"},
    {"stable_alias": "wh_payment_contract", "is_edge": True,
     "domain": "wh_payment", "range": "wh_contract",
     "label": "Оплата по договору"},
    {"stable_alias": "wh_stock_contract", "is_edge": True,
     "domain": "wh_stock", "range": "wh_contract",
     "label": "Хранение по договору"},
    {"stable_alias": "wh_outbound_carrier", "is_edge": True,
     "domain": "wh_outbound", "range": "wh_contractor",
     "label": "Перевозчик отгрузки"},
    {"stable_alias": "wh_outbound_receiver", "is_edge": True,
     "domain": "wh_outbound", "range": "person",
     "label": "Получатель (физлицо)"},
]


def main():
    pf.rpc_raw("initialize", {"protocolVersion": "2025-06-18",
                              "capabilities": {},
                              "clientInfo": {"name": "provision-schema-only",
                                             "version": "1.0"}})
    try:
        pf.rpc_raw("notifications/initialized")
    except Exception:
        pass

    types = pf.TYPES + EXTRA_TYPES
    preds = pf.PREDICATES + EXTRA_PREDICATES

    pub = pf.call("schema.published")
    if (pub.get("data") or []):
        print("WARNING: published revision already exists:",
              json.dumps(pub["data"], ensure_ascii=False)[:300])

    d = pf.call("schema.draft_create", {"name": "warehouse-schema-only"})
    if pf.failed(d):
        print("draft_create FAILED", json.dumps(d, ensure_ascii=False)[:600])
        return 1
    draft = d["data"]
    did = str(draft["revision_id"])
    etag = draft["etag"]
    print("draft", did, "etag", etag)

    u = pf.call("schema.draft_update", {
        "id": did,
        "display_name": "Складской домен + договоры (fresh)",
        "semantic_version": "1.0.0",
        "description": "Warehouse domain + contracts/finance extension",
        "types": types,
        "predicates": preds,
        "changes": [{"kind": "add", "resource_kind": "schema",
                     "resource_id": "warehouse-schema-only",
                     "summary": "Полный складской домен с договорами"}],
        "if_match": etag})
    if pf.failed(u):
        print("draft_update FAILED", json.dumps(u, ensure_ascii=False)[:1200])
        return 1
    etag = (u.get("data") or {}).get("etag") or etag
    print("draft updated:", len(types), "types,", len(preds), "predicates")

    v = pf.call("schema.draft_validate", {"id": did})
    print("validate:", json.dumps(v, ensure_ascii=False)[:800])
    pf.call("schema.draft_impact", {"id": did})
    g = pf.call("schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag

    p = pf.call("schema.draft_publish", {
        "id": did, "reason": "Schema-only publish on wiped DB",
        "if_match": etag})
    if pf.failed(p):
        print("publish FAILED", json.dumps(p, ensure_ascii=False)[:1200])
        return 1
    rev = str((p.get("data") or {}).get("revision_id", did))
    print("published revision", rev)
    with open(os.path.join(HERE, ".published_rev.json"), "w") as f:
        json.dump({"revision": rev}, f)
    return 0


if __name__ == "__main__":
    sys.exit(main())
