"""Revision 3 of the warehouse schema — field-kind upgrade.

The boot seed overwrites revision 1 content (`ON CONFLICT DO UPDATE`), so
the wh types live in `meta.type_definition` but the published record shows
only the 7 seed types. This script publishes a new revision built from the
CURRENT published one:

  - keeps every seed type/predicate;
  - merges wh_* types in (person/legal_entity merged field-by-field);
  - upgrades declared field kinds to the rich set: money, measurement,
    enum, tags, phone — exercising apps/api/src/field_kinds.rs coercion;
  - adds wh_tags (tags) scalar + field on wh_product.

Run: python docs/testing/warehouse/provision_v3_kinds.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import provision_fresh as pf                                   # noqa: E402
from provision import WH_TYPES, WH_SCALARS, WH_EDGES           # noqa: E402
from provision_v2 import NEW_TYPES, NEW_SCALARS, NEW_EDGES     # noqa: E402
from provision_schema_only import EXTRA_TYPES, EXTRA_PREDICATES  # noqa: E402

# --- field-kind upgrades ----------------------------------------------------
# {type_alias: {field_alias: {"kind": ..., extra keys...}}}
UPGRADES = {
    "wh_warehouse": {
        "wh_capacity_m2": {"kind": "measurement", "unit": "m2"},
    },
    "wh_zone": {
        "wh_zone_kind": {
            "kind": "enum",
            "values": ["приёмка", "хранение", "холодильная", "карантин",
                       "отгрузка", "прочее"],
        },
        "wh_capacity_m2": {"kind": "measurement", "unit": "m2"},
    },
    "wh_supplier": {"wh_phone": {"kind": "phone"}},
    "wh_client": {"wh_phone": {"kind": "phone"}},
    "wh_contractor": {"wh_phone": {"kind": "phone"}},
    "person": {"wh_phone": {"kind": "phone"}},
    "legal_entity": {"wh_phone": {"kind": "phone"}},
    "wh_cargo": {
        "wh_weight_kg": {"kind": "measurement", "unit": "kg"},
        "wh_volume_m3": {"kind": "measurement", "unit": "m3"},
    },
    "wh_waybill": {"wh_amount": {"kind": "money", "currency": "RUB"}},
    "wh_payment": {
        "wh_amount": {"kind": "money", "currency": "RUB"},
        "wh_direction": {
            "kind": "enum",
            "values": ["входящая", "исходящая", "incoming", "outgoing"],
        },
    },
    "wh_expense": {"wh_amount": {"kind": "money", "currency": "RUB"}},
    "wh_hold": {"wh_amount": {"kind": "money", "currency": "RUB"}},
    "wh_contract": {
        "wh_amount": {"kind": "money", "currency": "RUB"},
        "wh_contract_kind": {
            "kind": "enum",
            "values": ["поставка", "хранение", "аренда", "лизинг",
                       "страхование", "кредит", "транспорт", "выкуп",
                       "прочее"],
        },
    },
    "wh_product": {},
}

# New fields appended to existing types: {type_alias: [field, ...]}
NEW_FIELDS = {
    "wh_product": [
        {"kind": "tags", "alias": "wh_tags", "label": "Теги"},
    ],
}

NEW_PREDICATES = [
    {"stable_alias": "wh_tags", "scalar_kind": "json", "label": "Теги"},
]


def upgraded(t):
    """Return a copy of type entry `t` with UPGRADES + NEW_FIELDS applied."""
    t = json.loads(json.dumps(t, ensure_ascii=False))
    alias = t.get("stable_alias") or t.get("type_id")
    fields = {f.get("alias"): f for f in t.get("fields", [])}
    for fa, up in UPGRADES.get(alias, {}).items():
        if fa in fields:
            fields[fa].update(up)
    for nf in NEW_FIELDS.get(alias, []):
        if nf["alias"] not in fields:
            t.setdefault("fields", []).append(nf)
    return t


def merge_type(existing, new):
    """Merge `new` wh type into `existing` type (same alias): keep existing
    fields, upgrade kinds, append missing field aliases."""
    out = json.loads(json.dumps(existing, ensure_ascii=False))
    have = {f.get("alias") for f in out.get("fields", [])}
    for f in new.get("fields", []):
        if f.get("alias") not in have:
            out.setdefault("fields", []).append(f)
    return upgraded(out)


def main():
    pf.rpc_raw("initialize", {"protocolVersion": "2025-06-18",
                              "capabilities": {},
                              "clientInfo": {"name": "provision-v3",
                                             "version": "1.0"}})
    try:
        pf.rpc_raw("notifications/initialized")
    except Exception:
        pass

    pub = pf.call("schema.published")
    revs = pub.get("data") or []
    if not revs:
        print("no published revision — run provision_schema_only first")
        return 1
    base_id = str(revs[0]["revision_id"])
    print("base revision", base_id, revs[0].get("semantic_version"))

    d = pf.call("schema.draft_create",
                {"name": "warehouse-kinds-v3", "base_revision": base_id})
    if pf.failed(d):
        print("draft_create FAILED", json.dumps(d, ensure_ascii=False)[:600])
        return 1
    draft = d["data"]
    did = str(draft["revision_id"])
    etag = draft["etag"]
    print("draft", did)

    types = list(draft.get("types") or [])
    preds = list(draft.get("predicates") or [])
    by_alias = {}
    for i, t in enumerate(types):
        by_alias[t.get("stable_alias") or t.get("type_id")] = i
    existing_p = {p.get("stable_alias") for p in preds}

    wh_types = pf.BASE_TYPES + WH_TYPES + NEW_TYPES + EXTRA_TYPES
    added_t = merged_t = 0
    for t in wh_types:
        alias = t["stable_alias"]
        if alias in by_alias:
            types[by_alias[alias]] = merge_type(types[by_alias[alias]], t)
            merged_t += 1
        else:
            types.append(upgraded(t))
            added_t += 1
    added_p = 0
    for p in (WH_SCALARS + NEW_SCALARS + WH_EDGES + NEW_EDGES
              + EXTRA_PREDICATES + NEW_PREDICATES):
        if p["stable_alias"] not in existing_p:
            preds.append(p)
            added_p += 1
    print(f"types: {added_t} added, {merged_t} merged; predicates: +{added_p}")

    u = pf.call("schema.draft_update", {
        "id": did,
        "display_name": "СКР + складской домен (field kinds v3)",
        "semantic_version": "1.2.0",
        "description": "Warehouse domain with upgraded field kinds: money, "
                       "measurement, enum, tags, phone (field_kinds.rs)",
        "types": types,
        "predicates": preds,
        "changes": [{"kind": "update", "resource_kind": "schema",
                     "resource_id": "warehouse-kinds-v3",
                     "summary": "Прокачка видов полей: деньги, величины, "
                                "enum, теги, телефон"}],
        "if_match": etag})
    if pf.failed(u):
        print("draft_update FAILED", json.dumps(u, ensure_ascii=False)[:1200])
        return 1
    etag = (u.get("data") or {}).get("etag") or etag

    v = pf.call("schema.draft_validate", {"id": did})
    print("validate:", json.dumps(v, ensure_ascii=False)[:900])
    pf.call("schema.draft_impact", {"id": did})
    g = pf.call("schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag

    p = pf.call("schema.draft_publish", {
        "id": did,
        "reason": "Field-kind upgrade: money/measurement/enum/tags/phone",
        "if_match": etag})
    if pf.failed(p):
        print("publish FAILED", json.dumps(p, ensure_ascii=False)[:1200])
        return 1
    print("published revision",
          (p.get("data") or {}).get("revision_id", did))
    return 0


if __name__ == "__main__":
    sys.exit(main())
