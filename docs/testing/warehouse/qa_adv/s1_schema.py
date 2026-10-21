import json, threading, sys
sys.path.insert(0, ".")
from probe_lib import api, rec, brief, etag_of, sql_i, RESULTS

S = "schema-lifecycle"
TYPES = [{"type_id": "qa2_item", "stable_alias": "qa2_item", "entity_class": "data",
          "fields": [{"alias": "display_name", "kind": "text", "required": True}]}]
PREDS = [{"predicate_id": "display_name", "stable_alias": "display_name",
          "is_edge": False, "scalar_kind": "text", "min_cardinality": 0, "max_cardinality": 1}]
CHANGES = [{"kind": "add", "resource_kind": "type", "resource_id": "qa2_item",
            "summary": "add qa2_item type", "breaking": False}]


def publish(who, rid, reason):
    st, b, h = api(who, "GET", f"/schemas/drafts/{rid}")
    et = etag_of((st, b, h))
    return api(who, "POST", f"/schemas/drafts/{rid}/publish",
               {"reason": reason}, headers={"If-Match": et} if et else None)

def draft_flow(name, catalog="qa_empty", sv="0.1.0", who="admin"):
    st, b, h = api(who, "POST", f"/schemas/drafts?catalog={catalog}", {"name": name})
    rid = (b.get("data") or {}).get("revision_id") if isinstance(b, dict) else None
    return rid, (st, b, h)

# 1. create draft
rid, resp = draft_flow("qa2 schema adv")
rec(S, "create draft", "201 + revision_id", brief(resp), "PASS" if rid else "FAIL")
if not rid:
    print("abort"); sys.exit(1)

# 2. update draft content (PUT, If-Match)
et = etag_of(resp)
st, b, h = api("admin", "PUT", f"/schemas/drafts/{rid}",
               {"display_name": "QA2 Schema", "semantic_version": "0.1.0",
                "types": TYPES, "predicates": PREDS, "changes": CHANGES},
               headers={"If-Match": et} if et else None)
rec(S, "PUT draft body+If-Match", "200", brief((st, b, h)), "PASS" if st == 200 else "FAIL")

# PUT with stale etag -> 412
st2, b2, _ = api("admin", "PUT", f"/schemas/drafts/{rid}",
                 {"display_name": "x", "semantic_version": "0.1.0"},
                 headers={"If-Match": '"schema:999:999"'})
rec(S, "PUT stale If-Match", "412", brief((st2, b2, {})), "PASS" if st2 == 412 else "FAIL")

# 3. validate
st, b, h = api("admin", "POST", f"/schemas/drafts/{rid}/validate")
ok = isinstance(b, dict) and b.get("data", {}).get("valid") is True
rec(S, "validate draft", "valid=true", brief((st, b, h)), "PASS" if ok else "FAIL")

# validate again on non-draft status -> 422
st2, b2, _ = api("admin", "POST", f"/schemas/drafts/{rid}/validate")
rec(S, "re-validate non-draft", "422 state_invalid", brief((st2, b2, {})),
    "PASS" if st2 == 422 else "FAIL")

# 4. publish BEFORE impact -> 422
st2, b2, _ = publish("admin", rid, "premature publish attempt")
rec(S, "publish before impact", "422 schema.state_invalid", brief((st2, b2, {})),
    "PASS" if st2 == 422 else "FAIL")

# 5. publish by analyst (no schema.publish) -> 403
st2, b2, _ = api("analyst", "POST", f"/schemas/drafts/{rid}/publish",
                 {"reason": "analyst must not publish"})
rec(S, "analyst publish", "403", brief((st2, b2, {})), "PASS" if st2 == 403 else "FAIL")

# analyst creates own draft? lacks schema.draft cap -> 403
st2, b2, _ = api("analyst", "POST", "/schemas/drafts?catalog=qa_empty", {"name": "analyst draft"})
rec(S, "analyst create draft", "403", brief((st2, b2, {})), "PASS" if st2 == 403 else "FAIL")

# 6. impact
st, b, h = api("admin", "POST", f"/schemas/drafts/{rid}/impact")
rec(S, "impact", "200 impact_ready report", brief((st, b, h)), "PASS" if st == 200 else "FAIL")

# 7. publish with reason < 10 chars -> 400
st2, b2, _ = api("admin", "POST", f"/schemas/drafts/{rid}/publish", {"reason": "short"})
rec(S, "publish short reason", "400", brief((st2, b2, {})), "PASS" if st2 == 400 else "FAIL")

# 8. publish ok
st, b, h = publish("admin", rid, "qa2 schema publish for adversarial test")
pub_rev = b.get("data", {}).get("revision_id") if isinstance(b, dict) else None
rec(S, "publish draft", "200 status=published", brief((st, b, h)),
    "PASS" if st == 200 and pub_rev == rid else "FAIL")

# re-publish published revision -> 404 (not draft anymore)
st2, b2, _ = publish("admin", rid, "double publish attempt one")
rec(S, "re-publish same revision", "404/422 (no longer draft)", brief((st2, b2, {})),
    "PASS" if st2 in (404, 422, 409) else "FAIL")

# 9. type usable?
st, b, h = api("admin", "GET", "/schemas/types?catalog=qa_empty")
aliases = [t.get("stable_alias") or t.get("alias") for t in (b.get("data") or [])] if isinstance(b, dict) else []
rec(S, "type visible in catalog", "qa2_item listed", f"{st} aliases={aliases}",
    "PASS" if "qa2_item" in aliases else "FAIL")

st, b, h = api("admin", "POST", "/entities?catalog=qa_empty",
               {"type_id": "qa2_item", "fields": {"display_name": "QA2 Probe"}})
ent_pid = (b.get("data") or {}).get("public_id") if isinstance(b, dict) else None
rec(S, "entity of new type", "200/201 created", brief((st, b, h)),
    "PASS" if st in (200, 201) and ent_pid else "FAIL")

# 10. retire published revision
st, b, h = api("admin", "GET", f"/schemas/revisions/{rid}")
et2 = etag_of((st, b, h))
st, b, h = api("admin", "POST", f"/schemas/revisions/{rid}/retire",
               {"reason": "retire qa2 schema after test"},
               headers={"If-Match": et2} if et2 else None)
rec(S, "retire published revision", "200 retired", brief((st, b, h)),
    "PASS" if st == 200 else "FAIL")

# entity create after retire -> must fail
st, b, h = api("admin", "POST", "/entities?catalog=qa_empty",
               {"type_id": "qa2_item", "fields": {"display_name": "ghost"}})
rec(S, "create on retired schema", "error (type withdrawn)", brief((st, b, h)),
    "PASS" if st >= 400 else "FAIL")

# 11. concurrent publish race: build two drafts; make first one impact_ready,
# fire 2 publishes of same draft concurrently.
rid2, _ = draft_flow("qa2 race draft", sv="0.1.0")
api("admin", "PUT", f"/schemas/drafts/{rid2}",
    {"display_name": "QA2 Race", "semantic_version": "0.1.0",
     "types": TYPES, "predicates": PREDS, "changes": CHANGES})
api("admin", "POST", f"/schemas/drafts/{rid2}/validate")
api("admin", "POST", f"/schemas/drafts/{rid2}/impact")
outs = []
def pub():
    st, b, h = api("admin", "GET", f"/schemas/drafts/{rid2}")
    et = etag_of((st, b, h))
    outs.append(api("admin", "POST", f"/schemas/drafts/{rid2}/publish",
                    {"reason": "concurrent publish race test"},
                    headers={"If-Match": et} if et else None))
t1 = threading.Thread(target=pub); t2 = threading.Thread(target=pub)
t1.start(); t2.start(); t1.join(); t2.join()
codes = sorted(o[0] for o in outs)
rec(S, "two concurrent publishes", "exactly one 200, other 404/409/422",
    f"codes={codes}", "PASS" if codes.count(200) == 1 and len(codes) == 2 else "FAIL")
rc, out, err = sql_i(f"SELECT revision_id, content->>'status' FROM meta.schema_revision WHERE revision_id IN ({rid},{rid2}) ORDER BY revision_id")
print("      DB statuses:", out)
# cleanup rid2: published -> retire
st, b, h = api("admin", "GET", f"/schemas/revisions/{rid2}")
et3 = etag_of((st, b, h))
api("admin", "POST", f"/schemas/revisions/{rid2}/retire",
    {"reason": "cleanup race publish"}, headers={"If-Match": et3} if et3 else None)

# 12. impact on type with live entities: draft on qa_t removing qa_item
rid3, r3 = draft_flow("qa-t impact probe", catalog="qa_t", sv="0.19.0")
# fetch base types to mutate
st, b, h = api("admin", "GET", f"/schemas/drafts/{rid3}")
draft = (b.get("data") or {}) if isinstance(b, dict) else {}
types = draft.get("types", [])
kept = [t for t in types if (t.get("stable_alias") or t.get("alias")) != "qa_item"]
api("admin", "PUT", f"/schemas/drafts/{rid3}",
    {"display_name": "qa-t impact probe", "semantic_version": "0.19.0",
     "types": kept, "predicates": draft.get("predicates", []),
     "changes": [{"kind": "remove", "resource_kind": "type", "resource_id": "qa_item",
                  "summary": "remove qa_item with live entities", "breaking": True}]})
api("admin", "POST", f"/schemas/drafts/{rid3}/validate")
st, b, h = api("admin", "POST", f"/schemas/drafts/{rid3}/impact")
imp = (b.get("data") or {}) if isinstance(b, dict) else {}
live = sql_i("SELECT count(*) FROM core.entity e JOIN meta.type_definition t ON t.tenant_id=e.tenant_id AND t.type_id=e.type_id WHERE t.stable_alias='qa_item' AND e.retired_at IS NULL")[1]
note = f"removed qa_item; live entities={live}; affected_entities={imp.get('affected_entities')}"
verdict = "SUSPICIOUS" if imp.get("affected_entities") in (None, 0, 128) else "PASS"
rec(S, "impact on live-entity type removal", "real affected count / blocker", note, verdict)
st, b, h = publish("admin", rid3, "publish breaking removal")
rec(S, "publish breaking removal", "published (no live-entity blocker)", brief((st, b, h)),
    "SUSPICIOUS" if st == 200 and int(live or 0) > 0 else "PASS" if st != 200 else "FAIL")
if st == 200:
    st, b, h = api("admin", "GET", f"/schemas/revisions/{rid3}")
    et = etag_of((st, b, h))
    api("admin", "POST", f"/schemas/revisions/{rid3}/retire",
        {"reason": "cleanup breaking publish"}, headers={"If-Match": et} if et else None)
else:
    api("admin", "DELETE", f"/schemas/drafts/{rid3}")

# cleanup qa2_item entity
if ent_pid:
    st, b, _ = api("admin", "POST", f"/entities/{ent_pid}/retire",
                   {"reason": "qa cleanup retire"})
    rec(S, "cleanup: retire probe entity", "200", brief((st, b, {})),
        "PASS" if st == 200 else "FAIL")

print("\n=== S1 DONE ===")
