"""Catalog isolation e2e check (docs/28 §4).

Creates a data group `qa_wh_only` whose catalog grant is limited to `wh`,
a QA principal `qa_wh_viewer` in it (+ the `analysts` capability group for
login/entity capabilities), mints a token, and verifies:

  Part 1 — restricted principal:
    * /me/context.catalogs lists only `wh`
    * /api/v1/catalogs shows only `wh`
    * ?catalog=hr / ?catalog=qa_t / ?catalog=general listing is denied
    * wh entity via foreign catalog returns 404 (existence not leaked)
    * /schemas/types?catalog=<denied> is 403; ?catalog=wh is not
    * schema draft create / entity query in a denied catalog is rejected

  Part 2 — admin:
    * catalog CRUD: create, duplicate 409, invalid alias 422, update,
      system catalog retire 422, non-empty catalog retire 409,
      empty catalog retire/reactivate cycle
    * schema draft→validate→impact→publish inside a new catalog `qa_t`
    * alias collision: `person` type exists in `general` AND `qa_t`
    * type listing isolation: qa_t types don't leak into general
    * catalog-local dedup: same identity key inside one catalog
      deduplicates; entities created under qa_t stay invisible in general
    * wrong-catalog create: qa_t type under ?catalog=wh is rejected
    * cross-catalog edge: admin links a wh entity to a qa_t entity
      (allowed), but qa_wh_viewer cannot see the qa_t peer in relations
      and cannot create an edge into a denied catalog

  Part 5 — review regressions:
    * shared predicates (`name` in `general`) are writable on wh subjects
      via the catalog-first/tenant-fallback resolution
    * cross-catalog edge predicates (`wh_keeper` in `wh`) resolve on a
      general-catalog `person` subject
    * create in a retired catalog => 409
    * update/retire of a denied-catalog entity => 404 (no existence leak);
      update with a readable but mismatched ?catalog= => 409
    * entity audit events carry the owning catalog alias

Run:  python docs/testing/warehouse/catalog_isolation_check.py
"""
import json
import os
import uuid
import urllib.request
import urllib.error

HERE = os.path.dirname(os.path.abspath(__file__))
API = os.environ.get("ENGINER_API", "http://localhost:8010")

RESULTS = []


def check(name, ok, detail=""):
    RESULTS.append((name, ok))
    print(("PASS " if ok else "FAIL ") + name, detail)


def req(method, path, token, body=None, etag=None):
    headers = {"Authorization": f"Bearer {token}",
               "Content-Type": "application/json"}
    if etag:
        headers["If-Match"] = etag
    r = urllib.request.Request(
        API + path,
        data=json.dumps(body).encode() if body is not None else None,
        headers=headers,
        method=method,
    )
    try:
        res = urllib.request.urlopen(r, timeout=15)
        out = json.loads(res.read())
        out["_etag"] = res.headers.get("ETag")
        return res.status, out
    except urllib.error.HTTPError as e:
        try:
            return e.code, json.loads(e.read())
        except Exception:
            return e.code, {}


def req_raw(method, path, token, payload, content_type):
    headers = {"Authorization": f"Bearer {token}",
               "Content-Type": content_type}
    r = urllib.request.Request(API + path, data=payload,
                               headers=headers, method=method)
    try:
        res = urllib.request.urlopen(r, timeout=15)
        body = res.read()
        try:
            return res.status, json.loads(body)
        except Exception:
            return res.status, {"_raw": body.decode("utf-8", "replace")}
    except urllib.error.HTTPError as e:
        try:
            return e.code, json.loads(e.read())
        except Exception:
            return e.code, {}


def items(res):
    d = res[1].get("data", [])
    return d if isinstance(d, list) else d.get("items", [])


def type_names(res):
    return [t.get("id") or t.get("stable_alias") or t.get("alias")
            for t in items(res)]


def next_semver(admin, catalog):
    """Smallest semver above every published/retired revision in the
    catalog — publish rejects a version that is not higher than all
    previously published ones (re-runs must keep bumping)."""
    st, r = req("GET", f"/api/v1/schemas/published?catalog={catalog}",
                admin)
    best = (0, 0, 0)
    for rev in (r.get("data") or []):
        sv = str(rev.get("semantic_version") or "0.0.0").split(".")
        try:
            cur = tuple(int(x) for x in (sv + ["0", "0", "0"])[:3])
        except ValueError:
            continue
        best = max(best, cur)
    return f"{best[0]}.{best[1] + 1}.{best[2]}"


def ensure_catalog(alias, name, admin):
    st, _ = req("GET", f"/api/v1/catalogs/{alias}", admin)
    if st != 200:
        st, _ = req("POST", "/api/v1/catalogs", admin,
                    {"alias": alias, "name": name})
        return st in (200, 201)
    return True


def ensure_group(name, body, admin):
    """Create the group or update the canonical one with that name.

    POST /permissions/groups now rejects duplicate aliases with 409, but
    duplicates created earlier may still exist — converge on the OLDEST
    (the same one `resolve_group_ids` picks for memberships) and delete
    the rest so rules and membership can't split across dupes."""
    st, lst = req("GET", "/api/v1/permissions/groups", admin)
    existing = [g for g in (lst.get("data") or []) if g.get("name") == name]
    if existing:
        existing.sort(key=lambda g: g["id"])
        gid = existing[0]["id"]
        for extra in existing[1:]:
            st, r = req("DELETE",
                        f"/api/v1/permissions/groups/{extra['id']}", admin)
            assert st == 200, f"dup group delete failed: {st} {r}"
        st, r = req("PUT", f"/api/v1/permissions/groups/{gid}", admin, body)
        assert st == 200, f"group update {name} failed: {st} {r}"
        return gid
    st, r = req("POST", "/api/v1/permissions/groups", admin, body)
    assert st == 200, f"group create {name} failed: {st} {r}"
    return r["data"]["id"]


def main():
    admin = json.load(open(os.path.join(HERE, ".wh_tokens.json")))["admin"]

    # =====================================================================
    # Part 0 — restricted principal provisioning
    # =====================================================================
    group_body = {
        "name": "qa_wh_only",
        "description": "QA: wh-catalog only data scope (docs/28 check)",
        "class": "data",
        "rules": [
            {"effect": "allow", "resource_type": "catalog", "catalog": "wh",
             "action": "*"},
            {"effect": "allow", "resource_type": "type", "type_alias": "*",
             "action": "*"},
            {"effect": "allow", "resource_type": "field", "type_alias": "*",
             "field": "*", "action": "*"},
            {"effect": "allow", "resource_type": "relation", "predicate": "*",
             "action": "*"},
        ],
    }
    gid = ensure_group("qa_wh_only", group_body, admin)
    print("group:", gid)

    # Capability group: entity.edit/entity.retire so write requests reach
    # the handler — where the catalog gate, not the capability guard,
    # must produce the 404 (existence-leak regression checks). user.read
    # lets the viewer reach /users/<pid>/activity for the Part 9 leak
    # regression — data gates still apply inside the handler.
    caps_body = {
        "name": "qa_wh_caps",
        "description": "QA: entity edit/retire/activity capabilities",
        "class": "capability",
        "rules": [
            {"effect": "allow", "resource_type": "capability",
             "action": "entity.edit"},
            {"effect": "allow", "resource_type": "capability",
             "action": "entity.retire"},
            {"effect": "allow", "resource_type": "capability",
             "action": "user.read"},
        ],
    }
    ensure_group("qa_wh_caps", caps_body, admin)

    st, r = req("POST", "/api/v1/users", admin, {
        "login": "qa_wh_viewer", "display_name": "QA wh viewer",
        "is_service_account": True,
        "groups": ["analysts", "qa_wh_only", "qa_wh_caps"],
    })
    if st == 200:
        pid = r["data"]["principal_id"]
        print("user created:", pid)
    else:
        st, lst = req("GET", "/api/v1/users?q=qa_wh_viewer", admin)
        pid = next(u["principal_id"] for u in lst["data"]
                   if u["login"] == "qa_wh_viewer")
        st, r = req("PUT", f"/api/v1/users/{pid}", admin,
                    {"status": "active",
                     "groups": ["analysts", "qa_wh_only", "qa_wh_caps"]})
        assert st == 200, f"user update failed: {st} {r}"
        print("user updated:", pid)

    st, r = req("POST", f"/api/v1/users/{pid}/tokens", admin,
                {"name": "qa-wh-viewer"})
    assert st == 200, f"token mint failed: {st} {r}"
    tok = r["data"]["token"]
    tok_path = os.path.join(HERE, ".wh_token_qa_wh_viewer.json")
    json.dump({"token": tok}, open(tok_path, "w"))
    print("token saved to", tok_path)

    # =====================================================================
    # Part 1 — restricted principal assertions
    # =====================================================================
    st, r = req("GET", "/api/v1/me/context", tok)
    cats = [c["alias"] for c in r["data"].get("catalogs", [])]
    check("context.catalogs", cats == ["wh"], cats)

    st, r = req("GET", "/api/v1/catalogs", tok)
    cats = [c["alias"] for c in items((st, r))]
    check("/catalogs", cats == ["wh"], cats)

    st, r = req("GET", "/api/v1/entities?catalog=wh&limit=3", tok)
    wh_items = items((st, r))
    check("entities?catalog=wh", st == 200 and bool(wh_items),
          f"{st} {len(wh_items)}")

    for foreign in ("hr", "qa_t", "general"):
        st, r = req("GET", f"/api/v1/entities?catalog={foreign}&limit=3", tok)
        foreign_items = items((st, r))
        ok = st in (403, 404, 422) or (st == 200 and not foreign_items)
        check(f"entities?catalog={foreign} denied", ok,
              f"{st} {len(foreign_items)}")

    pid0 = wh_items[0]["public_id"] if wh_items else None
    if pid0:
        st, _ = req("GET", f"/api/v1/entities/{pid0}?catalog=hr", tok)
        check("wh entity via hr => 403/404", st in (403, 404), st)
        st, _ = req("GET", f"/api/v1/entities/{pid0}", tok)
        check("wh entity direct", st == 200, st)

    st, r = req("GET", "/api/v1/schemas/types?catalog=hr", tok)
    check("types?catalog=hr denied", st in (403, 404), st)

    st, r = req("GET", "/api/v1/schemas/types?catalog=wh", tok)
    check("types?catalog=wh", st == 200 and bool(items((st, r))), st)

    # Denied catalog → schema admin + query must also refuse.
    st, r = req("POST", "/api/v1/schemas/drafts?catalog=general", tok,
                {"name": "qa-should-fail"})
    check("draft create in denied catalog", st in (403, 404), st)

    st, r = req("POST", "/api/v1/entities/query?catalog=general", tok,
                {"where": [], "limit": 1})
    check("entities/query in denied catalog", st in (403, 404), st)

    # =====================================================================
    # Part 2 — admin: catalog CRUD lifecycle
    # =====================================================================
    check("catalog create qa_t", ensure_catalog("qa_t", "QA Catalog", admin))

    st, r = req("POST", "/api/v1/catalogs", admin,
                {"alias": "qa_t", "name": "dup"})
    check("catalog duplicate alias => 409", st == 409, st)

    st, r = req("POST", "/api/v1/catalogs", admin,
                {"alias": "QA Bad!", "name": "bad"})
    check("catalog invalid alias => 422", st == 422, st)

    st, r = req("PUT", "/api/v1/catalogs/qa_t", admin,
                {"name": "QA Catalog 2", "description": "isolation target"})
    check("catalog update", st == 200, st)

    st, r = req("GET", "/api/v1/catalogs/no_such_catalog", admin)
    check("catalog get missing => 404", st == 404, st)

    st, r = req("POST", "/api/v1/catalogs/sys/retire", admin)
    check("retire sys => 422", st == 422, st)

    st, r = req("POST", "/api/v1/catalogs/wh/retire", admin)
    check("retire non-empty wh => 409", st == 409, st)

    # Empty-catalog retire/reactivate cycle on a throwaway catalog.
    ensure_catalog("qa_empty", "QA Empty", admin)
    st, r = req("POST", "/api/v1/catalogs/qa_empty/retire", admin)
    check("retire empty catalog", st == 200, st)
    # Writes into a retired catalog are rejected regardless of type.
    st, r = req("POST", "/api/v1/entities?catalog=qa_empty", admin,
                {"type_id": "qa_item", "fields": {"qa_code": "X"}})
    check("create in retired catalog => 409", st == 409, st)
    st, r = req("PUT", "/api/v1/catalogs/qa_empty", admin,
                {"status": "active"})
    check("reactivate retired catalog", st == 200, st)

    # =====================================================================
    # Part 3 — admin: schema draft/publish inside qa_t (independent stream)
    # =====================================================================
    st, r = req("GET", "/api/v1/schemas/types?catalog=qa_t", admin)
    if "qa_item" not in type_names((st, r)):
        st, r = req("POST", "/api/v1/schemas/drafts?catalog=qa_t", admin,
                    {"name": "qa-t-schema",
                     "description": "isolation check schema"})
        check("draft create in qa_t", st in (200, 201), st)
        draft = r.get("data", {})
        did = str(draft.get("revision_id"))
        etag = draft.get("etag") or r.get("_etag") or ""

        qa_types = [
            {"type_id": "qa_item", "stable_alias": "qa_item",
             "entity_class": "data", "label": "QA Item",
             "fields": [
                 {"kind": "text", "alias": "qa_code", "label": "Код",
                  "identity": True, "required": True},
                 {"kind": "text", "alias": "name", "label": "Имя"}]},
            # Same alias as the SKR `person` in `general` — the collision
            # must be allowed: aliases are unique per catalog only.
            {"type_id": "person", "stable_alias": "person",
             "entity_class": "data", "label": "ФЛ (QA)",
             "fields": [
                 {"kind": "text", "alias": "qa_code", "label": "Код",
                  "identity": True, "required": True},
                 {"kind": "text", "alias": "full_name", "label": "ФИО"}]},
        ]
        qa_preds = [
            {"stable_alias": "qa_code", "scalar_kind": "text",
             "label": "Код", "participates_in_identity": True},
            {"stable_alias": "name", "scalar_kind": "text", "label": "Имя"},
            {"stable_alias": "full_name", "scalar_kind": "text",
             "label": "ФИО"},
            {"stable_alias": "qa_ref", "is_edge": True,
             "domain": "qa_item", "range": "qa_item", "label": "Связь"},
        ]
        st, r = req("PUT", f"/api/v1/schemas/drafts/{did}", admin, {
            "display_name": "QA isolation schema",
            "semantic_version": next_semver(admin, "qa_t"),
            "description": "qa_item + person alias collision",
            "types": qa_types,
            "predicates": qa_preds,
        }, etag=etag)
        check("draft update", st == 200, st)
        etag = r.get("_etag") or (r.get("data") or {}).get("etag") or etag

        st, r = req("POST", f"/api/v1/schemas/drafts/{did}/validate", admin)
        valid = (r.get("data") or {}).get("valid", st == 200)
        check("draft validate", st == 200 and valid, st)

        st, r = req("POST", f"/api/v1/schemas/drafts/{did}/impact", admin)
        check("draft impact", st == 200, st)
        etag = r.get("_etag") or (r.get("data") or {}).get("etag") or etag

        st, r = req("POST", f"/api/v1/schemas/drafts/{did}/publish", admin,
                    {"reason": "catalog isolation check"}, etag=etag)
        check("draft publish in qa_t", st == 200, st)

    st, r = req("GET", "/api/v1/schemas/types?catalog=qa_t", admin)
    qa_types_list = type_names((st, r))
    check("qa_t types listed", st == 200 and "qa_item" in qa_types_list,
          qa_types_list)

    st, r = req("GET", "/api/v1/schemas/types?catalog=general", admin)
    gen_types = type_names((st, r))
    check("qa_item not in general", "qa_item" not in gen_types,
          gen_types[:8])

    # Alias collision: `person` resolves differently per catalog.
    st_qa, r_qa = req("GET", "/api/v1/schemas/types/person?catalog=qa_t",
                      admin)
    st_gen, r_gen = req("GET", "/api/v1/schemas/types/person?catalog=general",
                        admin)
    qa_fields = {f.get("alias") for f in
                 (r_qa.get("data") or {}).get("fields", [])}
    gen_fields = {f.get("alias") for f in
                  (r_gen.get("data") or {}).get("fields", [])}
    check("person alias collision resolves per catalog",
          st_qa == 200 and st_gen == 200 and
          (r_qa.get("data") or {}).get("catalog") == "qa_t" and
          (r_gen.get("data") or {}).get("catalog") == "general" and
          "qa_code" in qa_fields and "qa_code" not in gen_fields,
          f"qa_fields={sorted(qa_fields)[:4]}")

    # =====================================================================
    # Part 4 — admin: entities, dedup isolation, cross-catalog edges
    # =====================================================================
    iso_code = "ISO-" + uuid.uuid4().hex[:8]
    st, r = req("POST", "/api/v1/entities", admin, {
        "type_id": "qa_item",
        "fields": {"qa_code": iso_code, "name": "Изоляция"},
    })
    item1 = (r.get("data") or {})
    check("qa_item created", st in (200, 201) and
          item1.get("outcome") == "created", f"{st} {item1}")
    item1_pid = item1.get("public_id")

    st, r = req("POST", "/api/v1/entities", admin, {
        "type_id": "qa_item",
        "fields": {"qa_code": iso_code, "name": "Дубликат"},
    })
    item2 = (r.get("data") or {})
    check("same identity deduplicated within catalog",
          st in (200, 201) and item2.get("outcome") == "deduplicated" and
          item2.get("public_id") == item1_pid, f"{st} {item2}")

    # Type belongs to qa_t — forcing ?catalog=wh must not let it resolve.
    st, r = req("POST", "/api/v1/entities?catalog=wh", admin, {
        "type_id": "qa_item",
        "fields": {"qa_code": "ISO-WRONG"},
    })
    check("qa_item via ?catalog=wh rejected", st in (404, 422), st)

    # Entities in qa_t stay invisible from the general stream.
    st, r = req("GET", "/api/v1/entities?catalog=qa_t", admin)
    qa_ent = items((st, r))
    check("entities?catalog=qa_t shows qa_item",
          st == 200 and any(e.get("public_id") == item1_pid
                            for e in qa_ent), f"{st} {len(qa_ent)}")
    st, r = req("GET", "/api/v1/entities?catalog=general&limit=200", admin)
    gen_ent = items((st, r))
    check("qa_t entity absent from general listing",
          st == 200 and all(e.get("public_id") != item1_pid
                            for e in gen_ent), f"{st} {len(gen_ent)}")

    # Cross-catalog edge: wh entity → qa_t entity. Admin sees both
    # catalogs, so the link is legal (docs/28 §5); the statement is owned
    # by the subject's catalog (wh).
    st, r = req("GET", "/api/v1/entities?catalog=wh&limit=1", admin)
    wh_subject = (items((st, r)) or [{}])[0].get("public_id")
    if wh_subject and item1_pid:
        st, r = req("POST", f"/api/v1/entities/{wh_subject}/relations",
                    admin, {"predicate": "wh_located_in",
                            "target_entity_id": item1_pid})
        # 200/201 on first create, 409 when a prior run already linked them.
        check("cross-catalog edge created (admin)", st in (200, 201, 409),
              st)

        # qa_wh_viewer sees the wh subject but the qa_t peer is hidden.
        st, r = req("GET", f"/api/v1/entities/{wh_subject}/relations", tok)
        rels = r.get("data") or []
        peers = [x.get("target_entity_id") for x in rels]
        check("cross-catalog peer hidden from restricted viewer",
              st == 200 and item1_pid not in peers, f"{st} {len(rels)}")

        # Restricted viewer cannot create an edge into a denied catalog —
        # target lookup must fail with 404, not leak existence.
        st, r = req("POST", f"/api/v1/entities/{wh_subject}/relations",
                    tok, {"predicate": "wh_located_in",
                          "target_entity_id": item1_pid})
        check("edge to denied catalog target => 404/403",
              st in (403, 404), st)

    # =====================================================================
    # Part 5 — review regressions
    # =====================================================================
    # Shared-predicate fallback: `name` lives in `general`, wh subjects may
    # still write it (docs/28 §5 fallback resolution).
    if wh_subject:
        st, r = req("PUT", f"/api/v1/entities/{wh_subject}", admin,
                    {"fields": {"name": "Shared-Predicate-Test"}})
        check("wh entity writes shared `name` predicate",
              st in (200, 204), st)
        st, r = req("GET", f"/api/v1/entities/{wh_subject}", admin)
        flds = (r.get("data") or {}).get("fields") or {}
        check("shared predicate value persisted",
              st == 200 and flds.get("name") == "Shared-Predicate-Test",
              f"{st} {list(flds)[:6]}")

    # Cross-catalog edge predicate: `wh_keeper` lives in `wh` but the
    # subject `person` lives in `general` — predicate resolution falls
    # back tenant-wide after checking the subject catalog.
    st, r = req("GET",
                "/api/v1/entities?catalog=general&type_id=person&limit=1",
                admin)
    person_pid = (items((st, r)) or [{}])[0].get("public_id")
    if person_pid and wh_subject:
        st, r = req("POST", f"/api/v1/entities/{person_pid}/relations",
                    admin, {"predicate": "wh_keeper",
                            "target_entity_id": wh_subject})
        check("cross-catalog edge predicate on general subject",
              st in (200, 201, 409), st)

    # Existence leak: writing a foreign-catalog entity with a readable
    # ?catalog= context must say 409; with no read access to the entity's
    # catalog it must say 404 — existence must not leak.
    if wh_subject and item1_pid:
        st, r = req("PUT", f"/api/v1/entities/{wh_subject}?catalog=qa_t",
                    admin, {"fields": {}})
        check("update readable entity, wrong catalog => 409",
              st == 409, st)
        st, r = req("PUT", f"/api/v1/entities/{item1_pid}?catalog=wh",
                    tok, {"fields": {}})
        check("update denied-catalog entity => 404 (no leak)",
              st == 404, st)
        st, r = req("POST", f"/api/v1/entities/{item1_pid}/retire",
                    tok, {"reason": "qa"})
        check("retire denied-catalog entity => 404 (no leak)",
              st == 404, st)

    # Audit: entity events carry the owning catalog alias (docs/28 §8).
    if item1_pid:
        st, r = req("GET", "/api/v1/audit?resource_kind=entity&limit=50",
                    admin)
        rows = r.get("data") or []
        tagged = [x for x in rows
                  if x.get("resource_id") == item1_pid]
        check("audit row tagged with entity catalog",
              st == 200 and any(x.get("catalog") == "qa_t"
                                for x in tagged),
              f"{st} {len(rows)} rows, {len(tagged)} for entity")

    # =====================================================================
    # Part 6 — deep-review regressions (files, revisions, ambiguity)
    # =====================================================================
    # ?catalog= vs body.catalog disagreement is ambiguous — both entity
    # create and advanced query must refuse instead of silently picking.
    st, r = req("POST", "/api/v1/entities?catalog=wh", admin,
                {"type_id": "qa_item", "catalog": "qa_t",
                 "fields": {"qa_code": "ISO-CONF"}})
    check("create catalog conflict => 422", st == 422, st)
    st, r = req("POST", "/api/v1/entities/query?catalog=wh", admin,
                {"catalog": "qa_t", "where": []})
    check("query catalog conflict => 422", st == 422, st)

    # Staged uploads are owned: a second principal must not see, chunk,
    # complete, or abort someone else's session.
    st, r = req("POST", "/api/v1/files/upload/init", admin, {
        "filename": "qa.txt", "content_type": "text/plain",
        "expected_size": 11, "chunk_size": 11,
        "expected_hash": __import__("hashlib")
            .sha256(b"hello world").hexdigest(),
        "hash_algorithm": "sha256"})
    up = (r.get("data") or {})
    upload_id = up.get("upload_id")
    check("upload init", st in (200, 201) and upload_id, f"{st} {up}")
    if upload_id:
        st, r = req("GET", f"/api/v1/files/upload/{upload_id}", tok)
        check("foreign upload status => 404", st == 404, st)
        st, r = req_raw("PUT",
                        f"/api/v1/files/upload/{upload_id}/chunks/0",
                        tok, b"hello world", "application/octet-stream")
        check("foreign upload chunk => 404", st == 404, st)
        st, r = req("POST", f"/api/v1/files/upload/{upload_id}/abort",
                    tok, {})
        check("foreign upload abort => 404", st == 404, st)
        # Owner completes the session — the object becomes readable by
        # its uploader only (unattached file).
        st, r = req_raw("PUT",
                        f"/api/v1/files/upload/{upload_id}/chunks/0",
                        admin, b"hello world", "application/octet-stream")
        check("owner chunk upload", st in (200, 201), st)
        st, r = req("POST",
                    f"/api/v1/files/upload/{upload_id}/complete", admin,
                    {"final_hash": __import__("hashlib")
                        .sha256(b"hello world").hexdigest(),
                     "total_size": 11})
        file_id = (r.get("data") or {}).get("object_id")
        check("upload complete", st == 200 and file_id, f"{st} {r}")
        if file_id:
            st, r = req("GET", f"/api/v1/files/{file_id}", admin)
            check("owner reads own file", st == 200, st)
            st, r = req("GET", f"/api/v1/files/{file_id}", tok)
            check("foreign file metadata => 404", st == 404, st)
            st, r = req("GET", f"/api/v1/files/{file_id}/download", tok)
            check("foreign file download => 404", st == 404, st)
            st, r = req("DELETE", f"/api/v1/files/{file_id}", tok)
            check("foreign file delete => 404", st == 404, st)
            st, r = req("DELETE", f"/api/v1/files/{file_id}", admin)
            check("owner deletes own file", st == 200, st)

    # Optimistic concurrency is enforced atomically: a stale revision is
    # a 409 even though the read-time check already ran.
    if wh_subject:
        st, r = req("PUT", f"/api/v1/entities/{wh_subject}", admin,
                    {"revision": 1, "fields": {}})
        check("stale revision => 409", st == 409, st)

    # Retiring the published schema revision withdraws the catalog's
    # registry definitions — type resolution must stop honouring them.
    st, r = req("GET", "/api/v1/schemas/published?catalog=qa_t", admin)
    revs = r.get("data") or []
    if isinstance(revs, dict):
        revs = revs.get("items", [])
    pub = next((x for x in revs
                if x.get("status") == "published"), None)
    check("published revision listed for qa_t", pub is not None,
          f"{st} {len(revs)}")
    if pub:
        rid = str(pub.get("revision_id") or pub.get("id"))
        st, r = req("GET", f"/api/v1/schemas/revisions/{rid}", admin)
        rev_etag = r.get("_etag") or (r.get("data") or {}).get("etag") \
            or pub.get("etag")
        st, r = req("POST", f"/api/v1/schemas/revisions/{rid}/retire",
                    admin, {"reason": "qa: withdraw published defs"},
                    etag=rev_etag)
        check("retire published revision", st == 200, st)
        st, r = req("POST", "/api/v1/entities?catalog=qa_t", admin, {
            "type_id": "qa_item",
            "fields": {"qa_code": "ISO-AFTER-RETIRE"}})
        check("create after schema retire => 422", st == 422, st)
        # Republish the same definitions so the check stays rerunnable.
        st, r = req("POST", "/api/v1/schemas/drafts?catalog=qa_t", admin,
                    {"name": "qa-t-schema-v2"})
        did2 = str((r.get("data") or {}).get("revision_id"))
        et2 = (r.get("data") or {}).get("etag") or r.get("_etag") or ""
        st, r = req("PUT", f"/api/v1/schemas/drafts/{did2}", admin, {
            "display_name": "QA isolation schema (restore)",
            "description": "republish after retire check",
            "semantic_version": next_semver(admin, "qa_t"),
            "types": [
                {"type_id": "qa_item", "stable_alias": "qa_item",
                 "entity_class": "data", "label": "QA Item",
                 "fields": [
                     {"kind": "text", "alias": "qa_code", "label": "Код",
                      "identity": True, "required": True},
                     {"kind": "text", "alias": "name", "label": "Имя"}]},
                {"type_id": "person", "stable_alias": "person",
                 "entity_class": "data", "label": "ФЛ (QA)",
                 "fields": [
                     {"kind": "text", "alias": "qa_code", "label": "Код",
                      "identity": True, "required": True},
                     {"kind": "text", "alias": "full_name",
                      "label": "ФИО"}]},
            ],
            "predicates": [
                {"stable_alias": "qa_code", "scalar_kind": "text",
                 "label": "Код", "participates_in_identity": True},
                {"stable_alias": "name", "scalar_kind": "text",
                 "label": "Имя"},
                {"stable_alias": "full_name", "scalar_kind": "text",
                 "label": "ФИО"},
                {"stable_alias": "qa_ref", "is_edge": True,
                 "domain": "qa_item", "range": "qa_item",
                 "label": "Связь"},
            ]}, etag=et2)
        check("republish draft update", st == 200,
              f"{st} {r.get('error', '')}")
        et2 = r.get("_etag") or (r.get("data") or {}).get("etag") or et2
        req("POST", f"/api/v1/schemas/drafts/{did2}/validate", admin)
        st, r = req("POST", f"/api/v1/schemas/drafts/{did2}/impact", admin)
        et2 = r.get("_etag") or (r.get("data") or {}).get("etag") or et2
        st, r = req("POST", f"/api/v1/schemas/drafts/{did2}/publish",
                    admin, {"reason": "qa: restore defs"}, etag=et2)
        check("republish revives defs", st == 200, st)
        st, r = req("GET", "/api/v1/schemas/types?catalog=qa_t", admin)
        check("republished qa_item resolvable",
              st == 200 and "qa_item" in type_names((st, r)), st)

    # =====================================================================
    # Part 7 — instance-level deny inside a readable catalog
    # =====================================================================
    # qa_wh_viewer may read the wh *catalog*, but a per-entity deny must
    # still hide the instance — including its display_name in domain
    # listings (/checks used to leak it through the catalog-only gate).
    st, r = req("GET", "/api/v1/entities?catalog=wh&limit=2", admin)
    wh_two = [e.get("public_id") for e in items((st, r))][:2]
    if len(wh_two) == 2:
        hidden_pid, visible_pid = wh_two
        # Deny-read on exactly one instance via a dedicated group —
        # separate from qa_wh_only so reruns can't lose the rule to
        # duplicate-name resolution.
        deny_gid = ensure_group("qa_wh_deny", {
            "name": "qa_wh_deny",
            "description": "QA: per-entity deny inside readable catalog",
            "class": "data",
            "rules": [
                {"effect": "deny", "resource_type": "entity",
                 "entity": hidden_pid, "action": "read"},
            ],
        }, admin)
        st, r = req("PUT", f"/api/v1/users/{pid}", admin, {
            "status": "active",
            "groups": ["analysts", "qa_wh_only", "qa_wh_caps",
                       "qa_wh_deny"],
        })
        check("instance deny rule installed", st == 200,
              f"{st} group={deny_gid}")

        # Direct read of the denied instance must 404 — no existence leak.
        st, r = req("GET", f"/api/v1/entities/{hidden_pid}", tok)
        check("denied instance GET => 404", st in (403, 404), st)
        st, r = req("GET", f"/api/v1/entities/{visible_pid}", tok)
        check("visible instance still readable", st == 200, st)

        # A domain row pointing at the denied instance must be filtered
        # out entirely (name/existence leak through /checks).
        st, r = req("POST", "/api/v1/checks", admin,
                    {"target_entity_id": hidden_pid, "check_kind": "kyc"})
        hid_check = (r.get("data") or {}).get("check_id")
        check("check on denied entity created", st in (200, 201), st)
        st, r = req("POST", "/api/v1/checks", admin,
                    {"target_entity_id": visible_pid, "check_kind": "kyc"})
        vis_check = (r.get("data") or {}).get("check_id")
        check("check on visible entity created", st in (200, 201), st)

        st, r = req("GET", "/api/v1/checks?limit=50", tok)
        listed = {x.get("check_id") for x in items((st, r))}
        check("denied-instance check hidden from listing",
              st == 200 and hid_check not in listed,
              f"{st} {len(listed)}")
        check("visible-instance check still listed",
              st == 200 and (vis_check in listed or not vis_check),
              f"{st} {len(listed)}")

        # Entity-scoped checks feed must also refuse the denied instance.
        st, r = req("GET",
                    f"/api/v1/entities/{hidden_pid}/checks", tok)
        check("denied-instance /entities/<pid>/checks => 404",
              st in (403, 404), st)
    else:
        check("two wh entities for instance-deny part", False,
              str(wh_two))

    # =====================================================================
    # Part 8 — audit catalog filter + dump hardening regressions
    # =====================================================================
    # `GET /audit?catalog=` slices events to one catalog on top of the
    # per-row visibility gate; an unknown ref is a 422, not a silent empty.
    st, r = req("GET", "/api/v1/audit?catalog=qa_t&limit=50", admin)
    cats = {x.get("catalog") for x in (r.get("data") or [])}
    check("audit ?catalog=qa_t filters rows",
          st == 200 and cats <= {"qa_t"} and bool(cats),
          f"{st} {cats}")
    st, r = req("GET", "/api/v1/audit?catalog=no_such_catalog_zz&limit=5",
                admin)
    check("audit ?catalog unknown => 422", st == 422, st)

    # dump_id validation must reject the root-self reference: with the
    # old validator DELETE /admin/dump/. would wipe the dump directory.
    st, r = req("GET", "/api/v1/admin/dump/%2E/status.json", admin)
    check("dump id '.' rejected", st == 404, st)
    st, r = req("DELETE", "/api/v1/admin/dump/%2E", admin)
    check("dump delete '.' rejected", st == 404, st)
    st, r = req("POST", "/api/v1/admin/dump/%2E/restore", admin,
                {"confirm": "RESTORE"})
    check("dump restore '.' rejected", st == 404, st)
    # Restore of a nonexistent dump must be 404 even with confirm.
    st, r = req("POST", "/api/v1/admin/dump/no_such_dump_zz/restore",
                admin, {"confirm": "RESTORE"})
    check("dump restore missing => 404", st == 404, st)
    # Restore without the confirm body is a 422 — destructive ops need it.
    st, r = req("POST", "/api/v1/admin/dump/no_such_dump_zz/restore",
                admin, {})
    check("dump restore no confirm => 422", st == 422, st)

    # =====================================================================
    # Part 9 — user-activity feed respects catalog/entity visibility
    # =====================================================================
    # GET /users/<pid>/activity used to list every entity the target
    # principal created/changed/viewed with NO visibility filter — the
    # requester could learn public_id + type of entities in catalogs they
    # cannot read. Rows are now gated like any other listing.
    # The bearer token's own principal — created_by rows are stamped
    # with it, not with the seeded `admin` persona's id.
    st, r = req("GET", "/api/v1/me/context", admin)
    admin_pid = (r.get("data") or {}).get("principal_id")
    check("admin principal resolved", st == 200 and bool(admin_pid), st)

    # A fresh entity in qa_t created BY admin — the activity target.
    st, r = req("POST", "/api/v1/entities", admin, {
        "type_id": "qa_item",
        "fields": {"qa_code": "ACT-" + uuid.uuid4().hex[:8],
                   "name": "Activity-Leak-Check"},
    })
    act_pid = (r.get("data") or {}).get("public_id")
    check("qa_t entity for activity check created",
          st in (200, 201) and bool(act_pid),
          f"{st} {r.get('error', '')}")

    # Also produces a catalog-tagged audit row for the log feed.
    req("GET", f"/api/v1/entities/{act_pid}", admin)

    if admin_pid and act_pid:
        st, r = req("GET",
                    f"/api/v1/users/{admin_pid}/activity?limit=200", admin)
        feed_pids = {
            e.get("public_id")
            for sec in ("created", "changed", "viewed")
            for e in (r.get("data") or {}).get(sec, [])
        }
        check("admin sees own activity incl. qa_t entity",
              st == 200 and act_pid in feed_pids,
              f"{st} present={act_pid in feed_pids}")

        st, r = req("GET",
                    f"/api/v1/users/{admin_pid}/activity?limit=200", tok)
        check("viewer may query activity feed", st == 200, st)
        feed_pids = {
            e.get("public_id")
            for sec in ("created", "changed", "viewed")
            for e in (r.get("data") or {}).get(sec, [])
        }
        log_actions = [
            e.get("action") or ""
            for e in (r.get("data") or {}).get("log", [])
        ]
        check("denied-catalog entity absent from activity feeds",
              st == 200 and act_pid not in feed_pids, f"{st}")
        check("denied-catalog entity absent from activity log",
              all(act_pid not in a for a in log_actions), st)

    # =====================================================================
    # Part 10 — report linkage + group-alias uniqueness
    # =====================================================================
    # A report row exposes the entity it was generated for
    # (`entity_public_id`) plus a download url into the rendered fields —
    # a report on a denied-catalog entity must vanish from the viewer's
    # listing, and its file must refuse to download.
    if act_pid:
        st, r = req("POST", "/api/v1/reports", admin,
                    {"entity_id": act_pid, "format": "json"})
        rep = r.get("data") or {}
        rep_id = rep.get("report_id")
        rep_url = rep.get("download_url") or ""
        rep_file = rep_url.rstrip("/").rsplit("/", 2)[-2] \
            if "/files/" in rep_url else ""
        check("report on qa_t entity created",
              st in (200, 201) and bool(rep_id), f"{st} {rep}")

        st, r = req("GET", "/api/v1/reports", tok)
        rep_ids = {x.get("report_id") for x in items((st, r))}
        check("denied-entity report hidden from listing",
              st == 200 and rep_id not in rep_ids, f"{st}")
        if rep_file:
            st, _ = req("GET", f"/api/v1/files/{rep_file}/download", tok)
            check("denied-entity report file not downloadable",
                  st in (403, 404), st)
        st, r = req("GET", "/api/v1/reports", admin)
        rep_ids = {x.get("report_id") for x in items((st, r))}
        check("admin sees the report", rep_id in rep_ids, f"{st}")

    # Duplicate group aliases split memberships from rules — POST must
    # now refuse them instead of spawning a divergent twin.
    st, r = req("POST", "/api/v1/permissions/groups", admin,
                {"name": "qa_wh_only", "class": "data",
                 "description": "dup", "rules": []})
    check("duplicate group alias => 409", st == 409, st)

    print()
    failed = [n for n, ok in RESULTS if not ok]
    print(f"{len(RESULTS) - len(failed)}/{len(RESULTS)} checks passed")
    if failed:
        print("FAILED:", ", ".join(failed))
        return 1
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())
