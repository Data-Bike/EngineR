"""Restore an EngineR dump produced by dump_all.py — entirely via MCP.

Reads either dump.json or dump.puml (extracts the base64 block between
DUMP64-BEGIN/DUMP64-END legend markers — unescaping is unnecessary because
the payload alphabet is PUML-safe).

Restore is idempotent: entity identity keys dedup (same public_id returns
as `deduplicated`), duplicate relations come back 409.

  python docs/testing/warehouse/restore_dump.py [dump.puml|dump.json]
       [--dry-run]
"""
import base64
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mcp_probe as m  # noqa: E402


def handshake():
    m.rpc_raw("initialize", {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "restore", "version": "1.0"}})
    m.rpc_raw("notifications/initialized")


def load_dump(path):
    raw = open(path, "rb").read().decode("utf-8")
    if path.endswith(".puml") or "DUMP64-BEGIN" in raw:
        begin = raw.index("DUMP64-BEGIN")
        begin = raw.index("==", begin)  # marker line tail
        # collect the line region between BEGIN/END markers
        block = re.search(
            r"DUMP64-BEGIN[^\n]*\n(.*?)== DUMP64-END", raw, re.S)
        if not block:
            sys.exit("no DUMP64 block found in " + path)
        b64 = re.sub(r"[^A-Za-z0-9+/=]", "", block.group(1))
        return json.loads(base64.b64decode(b64).decode("utf-8"))
    return json.loads(raw)


def type_alias(t):
    return t.get("stable_alias") or t.get("type_id") or t.get("id")


def ensure_schema(dumped_schema, dry):
    """Publish the dumped catalog if the live DB doesn't already have it."""
    pub = m.call("schema.published")
    live = (pub.get("data") or [None])[0]
    want = {type_alias(t) for t in dumped_schema.get("types") or []}
    if live:
        have = {type_alias(t) for t in live.get("types") or []}
        if want <= have:
            print(f"schema already covers {len(want)} types "
                  f"(rev {live.get('revision_id')}) — skipping publish")
            return
        base = live.get("revision_id")
    else:
        base = None
    if dry:
        print(f"[dry] would publish dumped catalog "
              f"({len(want)} types, base={base})")
        return

    d = m.call("schema.draft_create",
               {"name": "restore-dump",
                **({"base_revision": str(base)} if base else {})})
    if d.get("_isError") or d.get("_rpc_err") or d.get("_http"):
        sys.exit("draft_create failed: " + json.dumps(d)[:400])
    draft = d.get("data") or {}
    did = str(draft.get("revision_id") or draft.get("id"))
    g = m.call("schema.draft_get", {"id": did})
    draft = g.get("data") or draft
    etag = draft.get("etag")

    types = list(draft.get("types") or [])
    preds = list(draft.get("predicates") or [])
    have_t = {type_alias(t) for t in types}
    have_p = {p.get("stable_alias") for p in preds}
    for t in dumped_schema.get("types") or []:
        if type_alias(t) not in have_t:
            types.append(t)
    for p in dumped_schema.get("predicates") or []:
        if p.get("stable_alias") not in have_p:
            preds.append(p)

    u = m.call("schema.draft_update", {
        "id": did, "types": types, "predicates": preds, "if_match": etag,
        "display_name": dumped_schema.get("display_name") or "restored",
        "semantic_version": (dumped_schema.get("semantic_version")
                             or "1.0.0"),
        "changes": [{"kind": "add", "resource_kind": "type",
                     "resource_id": "*",
                     "summary": "restore dump"}]})
    if u.get("_isError") or u.get("_rpc_err"):
        sys.exit("draft_update failed: " + json.dumps(u)[:400])
    m.call("schema.draft_validate", {"id": did})
    m.call("schema.draft_impact", {"id": did})
    g = m.call("schema.draft_get", {"id": did})
    etag = (g.get("data") or {}).get("etag") or etag
    p = m.call("schema.draft_publish", {"id": did,
                                        "if_match": etag,
                                        "reason": "restore dump"})
    if p.get("_isError") or p.get("_rpc_err"):
        sys.exit("publish failed: " + json.dumps(p)[:400])
    print("published catalog revision",
          (p.get("data") or {}).get("revision_id"))


def restore(d, dry):
    ensure_schema(d["schema"], dry)

    # id map: dumped public_id -> live public_id (dedup may return identical)
    idmap = {}
    created = dedup = failed = 0
    for i, e in enumerate(d["entities"]):
        if i % 100 == 0:
            print(f"  entities {i}/{len(d['entities'])} "
                  f"(created={created} dedup={dedup} fail={failed})")
        if dry:
            idmap[e["public_id"]] = e["public_id"]
            continue
        r = m.call("entities.create", {"type_id": e["type"],
                                       "fields": e["fields"]})
        data = r.get("data") or {}
        npid = data.get("public_id")
        if not npid or r.get("_isError") or r.get("_rpc_err"):
            failed += 1
            print("  FAIL", e["public_id"], json.dumps(r)[:200])
            continue
        idmap[e["public_id"]] = npid
        if data.get("outcome") == "deduplicated":
            dedup += 1
        else:
            created += 1
    print(f"entities: created={created} dedup={dedup} failed={failed}")

    rel_ok = rel_dup = rel_fail = 0
    for i, r in enumerate(d["relations"]):
        if i % 200 == 0:
            print(f"  relations {i}/{len(d['relations'])}")
        s, o = idmap.get(r["subject"]), idmap.get(r["object"])
        if not s or not o:
            rel_fail += 1
            continue
        if dry:
            continue
        # relations.create only accepts id/predicate/target_entity_id;
        # qualifiers/validity are kept in dump.json for reference but the
        # current tool surface cannot replay them (unknown-argument).
        args = {"id": s, "predicate": r["predicate"],
                "target_entity_id": o}
        res = m.call("relations.create", args)
        if res.get("data"):
            rel_ok += 1
        else:
            err = json.dumps(res)
            if '"status": 409' in err or "409" in err:
                rel_dup += 1
            else:
                rel_fail += 1
                if rel_fail < 8:
                    print("  REL FAIL", r["subject"], "->", r["object"],
                          err[:200])
    print(f"relations: created={rel_ok} duplicate={rel_dup} "
          f"failed={rel_fail}")
    return created, dedup, failed, rel_ok, rel_dup, rel_fail


def verify(d):
    """Compare live counts to the dump."""
    want = {}
    for e in d["entities"]:
        want[e["type"]] = want.get(e["type"], 0) + 1
    print("\n== verification (dumped vs live) ==")
    for alias, n in sorted(want.items()):
        # entities.list clamps to 200 — count via entities.query paging
        live = 0
        after = None
        while True:
            page = {"size": 500}
            if after:
                page["after"] = after
            r = m.call("entities.query", {"query": {
                "from": {"type_id": alias}, "page": page}})
            data = r.get("data") or {}
            rows = data.get("rows") or data.get("items") or []
            live += len(rows)
            cur = data.get("next_cursor") or data.get("cursor")
            if not rows or not cur or cur == after:
                break
            after = cur
        flag = "OK" if live >= n else "MISMATCH"
        print(f"  {alias}: dump={n} live={live} {flag}")


def main():
    path = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") \
        else os.path.join(os.path.dirname(os.path.abspath(__file__)),
                          "dump.puml")
    dry = "--dry-run" in sys.argv
    d = load_dump(path)
    print(f"dump: {d['format']} from {d['created_at']}: "
          f"{len(d['entities'])} entities, {len(d['relations'])} relations")
    handshake()
    restore(d, dry)
    if not dry:
        verify(d)


if __name__ == "__main__":
    main()
