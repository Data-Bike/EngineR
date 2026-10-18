"""Restore full content to kb_article entities truncated by the
statement_active_fact_unique btree cap. The index was replaced by a
hashed variant (migration 20261018000024_statement_fact_hash_index.sql),
so full-length kb_content now stores fine.

Everything goes through MCP tools/call — no raw SQL, no raw HTTP API.

Run:  python docs/testing/warehouse/kb_restore_truncated.py
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kb_provision import rpc, call, failed, show  # noqa: E402
from kb_docs_provision import ARTICLES  # noqa: E402

MARKER = "[truncated: btree cap"
FULL = {a["kb_id"]: a["content"] for a in ARTICLES}


def list_articles():
    rows = []
    cursor = None
    while True:
        args = {"type": "kb_article", "limit": 200}
        if cursor:
            args["cursor"] = cursor
        r = call("entities.list", args)
        if failed(r):
            print("entities.list FAILED:", show(r, 400))
            break
        data = r.get("data") or []
        rows.extend(data if isinstance(data, list) else data.get("rows", []))
        nxt = (r.get("meta") or {}).get("next_cursor") or r.get("next_cursor")
        if not nxt or nxt == cursor:
            break
        cursor = nxt
    return rows


def get_content(pid):
    g = call("entities.get", {"id": pid})
    if failed(g):
        return None, g
    fields = ((g.get("data") or {}).get("fields")) or {}
    return fields.get("kb_content"), g


def main():
    rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "kb-restore", "version": "1"}})
    rpc("notifications/initialized")

    rows = list_articles()
    print(f"entities.list kb_article -> {len(rows)} rows")

    truncated = []   # (kb_id, public_id)
    probes = []      # (kb_id, public_id)
    for row in rows:
        f = row.get("fields") or {}
        kb_id = f.get("kb_id") or ""
        pid = row.get("public_id") or row.get("id")
        if kb_id.startswith("kb.probe."):
            probes.append((kb_id, pid))
            continue
        content, g = get_content(pid)
        if content is None:
            print(f"GET FAILED {kb_id} ({pid}):", show(g, 300))
            continue
        if MARKER in content:
            truncated.append((kb_id, pid))

    print(f"truncated articles: {len(truncated)}")
    print(f"leftover kb.probe.*: {len(probes)}")

    restored, missing, failures = 0, [], []
    for kb_id, pid in truncated:
        full = FULL.get(kb_id)
        if full is None:
            missing.append(kb_id)
            print(f"NO DEFINITION for {kb_id} — needs manual rebuild")
            continue
        r = call("entities.update", {"id": pid,
                                     "fields": {"kb_content": full}})
        if failed(r):
            failures.append(kb_id)
            print(f"UPDATE FAILED {kb_id}:", show(r, 400))
            continue
        # verify
        content, g = get_content(pid)
        if content is None or MARKER in content or content != full:
            failures.append(kb_id)
            print(f"VERIFY FAILED {kb_id}: "
                  f"len={len(content or '')} expected={len(full)}")
            continue
        restored += 1
        print(f"{kb_id:>18} restored ({len(full)} chars)")

    retired = 0
    for kb_id, pid in probes:
        r = call("entities.retire", {"id": pid,
                                     "reason": "leftover probe article "
                                               "from btree-cap sizing; "
                                               "superseded by index fix"})
        if failed(r):
            print(f"RETIRE FAILED {kb_id}:", show(r, 300))
        else:
            retired += 1
            print(f"{kb_id:>18} retired")

    # final verification sweep
    remaining = 0
    rows = list_articles()
    still_probe = 0
    for row in rows:
        f = row.get("fields") or {}
        kb_id = f.get("kb_id") or ""
        pid = row.get("public_id") or row.get("id")
        if kb_id.startswith("kb.probe."):
            still_probe += 1
            continue
        content, _ = get_content(pid)
        if content and MARKER in content:
            remaining += 1
            print(f"STILL TRUNCATED: {kb_id}")

    print("\n===== SUMMARY =====")
    print(json.dumps({
        "truncated_found": len(truncated),
        "restored": restored,
        "missing_definition": missing,
        "update_or_verify_failures": failures,
        "probes_found": len(probes),
        "probes_retired": retired,
        "probes_remaining": still_probe,
        "markers_remaining": remaining,
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
