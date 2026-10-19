"""Full data dump of the EngineR database via MCP — writes:

  dump.json  — canonical machine-readable dump (schema + entities + relations)
  dump.puml  — human-readable PlantUML map of all entities/relations AND a
               self-contained restorable payload: the canonical JSON dump,
               base64-encoded between DUMP64-BEGIN/DUMP64-END legend lines.
               base64 uses only [A-Za-z0-9+/=] so nothing needs escaping for
               PlantUML; restore_dump.py reads the same block back verbatim.

Run:  python docs/testing/warehouse/dump_all.py [out_dir]
"""
import base64
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mcp_probe as m  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1] if len(sys.argv) > 1 else HERE


def handshake():
    m.rpc_raw("initialize", {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "dump", "version": "1.0"}})
    m.rpc_raw("notifications/initialized")


def type_alias(t):
    return t.get("stable_alias") or t.get("type_id") or t.get("id")


def all_entity_ids(alias):
    """Enumerate every active entity of a type via entities.query paging."""
    ids = []
    after = None
    while True:
        page = {"size": 500}
        if after:
            page["after"] = after
        r = m.call("entities.query", {"query": {
            "from": {"type_id": alias}, "page": page}})
        data = r.get("data") or {}
        rows = data.get("rows") or data.get("items") or []
        if not rows:
            break
        for row in rows:
            pid = row.get("public_id") or row.get("id")
            if pid:
                ids.append(pid)
        cur = data.get("next_cursor") or data.get("cursor")
        if not cur or cur == after:
            break
        after = cur
    return ids


def dump():
    pub = m.call("schema.published")
    schema = (pub.get("data") or [None])[0]
    if not schema:
        sys.exit("no published schema — nothing to dump")
    aliases = [type_alias(t) for t in schema.get("types") or []]
    print(f"published rev {schema.get('revision_id')}: {len(aliases)} types")

    entities = []
    for alias in aliases:
        ids = all_entity_ids(alias)
        print(f"  {alias}: {len(ids)} entities")
        for pid in ids:
            g = m.call("entities.get", {"id": pid})
            d = g.get("data") or {}
            entities.append({
                "public_id": pid,
                "type": alias,
                "status": d.get("status"),
                "revision": d.get("revision"),
                "display_name": d.get("display_name"),
                "fields": d.get("fields") or {},
            })

    known = {e["public_id"] for e in entities}
    relations = {}
    for i, e in enumerate(entities):
        if i % 100 == 0:
            print(f"  relations scan {i}/{len(entities)}")
        r = m.call("relations.list", {"id": e["public_id"],
                                      "direction": "outgoing"})
        for rel in r.get("data") or []:
            tgt = rel.get("target_entity_id")
            if tgt not in known:
                continue
            key = (e["public_id"], rel.get("predicate"), tgt,
                   rel.get("relation_id"))
            relations[key] = {
                "relation_id": rel.get("relation_id"),
                "subject": e["public_id"],
                "predicate": rel.get("predicate"),
                "object": tgt,
                "valid_from": rel.get("valid_from"),
                "valid_to": rel.get("valid_to"),
                "qualifiers": rel.get("qualifiers") or {},
            }
    rels = list(relations.values())
    print(f"  relations: {len(rels)}")

    return {
        "format": "enginer-dump/1",
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "schema": schema,
        "entities": entities,
        "relations": rels,
    }


# ------------------------------------------------------------------ puml

def esc(s, maxlen=90):
    """Escape a string for a PlantUML object/label slot.

    PlantUML labels cannot contain raw newlines or double quotes; non-ASCII
    is fine (file is UTF-8). We JSON-escape the unsafe chars so the visible
    text round-trips, and truncate for readability — the authoritative data
    is the base64 payload, the diagram is for humans."""
    if s is None:
        return ""
    s = str(s)
    s = (s.replace("\\", "\\\\")
          .replace('"', "\\\"")
          .replace("\r\n", "\\n").replace("\n", "\\n").replace("\r", "\\n")
          .replace("\t", "\\t"))
    if len(s) > maxlen:
        s = s[: maxlen - 1] + "…"
    return s


def node_id(pid):
    return "E_" + pid.replace("-", "")


def to_puml(d):
    lines = [
        "@startuml",
        f"title EngineR data dump — {d['created_at']} "
        f"({len(d['entities'])} entities, {len(d['relations'])} relations)",
        "skinparam objectAttributeFontSize 10",
        "skinparam linetype ortho",
        "",
        "' ==== Entities ====",
    ]
    for e in d["entities"]:
        nid = node_id(e["public_id"])
        label = f"{e['type']}: {e.get('display_name') or e['public_id']}"
        lines.append(f'object "{esc(label)}" as {nid} {{')
        for k, v in (e.get("fields") or {}).items():
            lines.append(f'  {esc(k, 40)} = "{esc(v)}"')
        lines.append("}")
    lines += ["", "' ==== Relations ===="]
    for r in d["relations"]:
        lines.append(
            f"{node_id(r['subject'])} --> {node_id(r['object'])} : "
            f"{esc(r['predicate'], 40)}")

    # Embedded canonical payload — machine-restore block.
    payload = base64.b64encode(
        json.dumps(d, ensure_ascii=False).encode("utf-8")).decode("ascii")
    lines += ["", "legend right",
              "== DUMP64-BEGIN enginer-dump/1 (base64 utf-8 json) =="]
    lines += [payload[i:i + 76] for i in range(0, len(payload), 76)]
    lines += ["== DUMP64-END ==", "endlegend", "@enduml"]
    return "\n".join(lines) + "\n"


def main():
    handshake()
    d = dump()
    js = os.path.join(OUT, "dump.json")
    pu = os.path.join(OUT, "dump.puml")
    with open(js, "w", encoding="utf-8") as f:
        json.dump(d, f, ensure_ascii=False, indent=1)
    with open(pu, "w", encoding="utf-8") as f:
        f.write(to_puml(d))
    print(f"wrote {js} ({len(d['entities'])} entities) + {pu}")


if __name__ == "__main__":
    main()
