import json, lzma, subprocess, threading, time, uuid, sys
sys.path.insert(0, ".")
from probe_lib import api, rec, brief, sql, sql_i, tenant_id, RESULTS

S = "dump-lifecycle"
TID = tenant_id()
created_dumps = []

def dump_status(did):
    st, b, _ = api("admin", "GET", "/admin/dump")
    if not isinstance(b, dict):
        return None
    for d in b.get("data") or []:
        if d.get("dump_id") == did:
            return d.get("status")
    return None

def wait_done(did, tries=60):
    for _ in range(tries):
        s = dump_status(did)
        if s not in ("running", "restoring"):
            return s
        time.sleep(1)
    return "timeout"

# ---- 0. permission gate
st, b, h = api("analyst", "POST", "/admin/dump")
rec(S, "analyst export", "403", brief((st, b, h)), "PASS" if st == 403 else "FAIL")
st, b, h = api("operator", "POST", "/admin/dump/upload", raw=b"{}\n")
rec(S, "operator upload", "403", brief((st, b, h)), "PASS" if st == 403 else "FAIL")

# ---- 1. export
st, b, h = api("admin", "POST", "/admin/dump")
did = (b.get("data") or {}).get("dump_id") if isinstance(b, dict) else None
rec(S, "export start", "200 running", brief((st, b, h)), "PASS" if did else "FAIL")
if did:
    created_dumps.append(did)
    # concurrent export while running -> 409
    st2, b2, _ = api("admin", "POST", "/admin/dump")
    rec(S, "concurrent export", "409 busy", brief((st2, b2, {})),
        "PASS" if st2 == 409 else "FAIL")
    if st2 == 200:
        created_dumps.append((b2.get("data") or {}).get("dump_id"))
    st_final = wait_done(did)
    rec(S, "export finishes", "done", st_final, "PASS" if st_final == "done" else "FAIL")

    # ---- 2. download + secret scan
    st, content, h = api("admin", "GET", f"/admin/dump/{did}/dump.ndjson.xz")
    ok = st == 200 and isinstance(content, bytes)
    rec(S, "download archive", "200 xz bytes", f"{st} {len(content) if ok else ''}",
        "PASS" if ok else "FAIL")
    if ok:
        raw = lzma.decompress(content)
        tables = set()
        secret_hits = []
        for line in raw.split(b"\n"):
            if not line:
                continue
            try:
                rec_row = json.loads(line)
            except Exception:
                continue
            t = rec_row.get("table", "")
            tables.add(t)
            if t in ("sec.session", "sec.principal_credential", "sec.api_token"):
                secret_hits.append(t)
        rec(S, "secret tables excluded",
            "no sec.session/principal_credential/api_token lines",
            f"tables={len(tables)} secret_hits={secret_hits}",
            "PASS" if not secret_hits else "FAIL")
        hdr = json.loads(raw.split(b"\n")[0])
        rec(S, "header line", "_header w/ format", str(hdr)[:100],
            "PASS" if hdr.get("table") == "_header" else "FAIL")

    # traversal on file names -> 404
    for fid, fname in [(did, "..%2F..%2Fetc%2Fpasswd"), (did, "../../evil"),
                       (did, ".."), (did, "manifest.json/../../x"),
                       ("..%2F..", "status.json"), ("evil..id", "status.json"),
                       (".", "status.json")]:
        st2, b2, _ = api("admin", "GET", f"/admin/dump/{fid}/{fname}")
        rec(S, f"file download {fid}/{fname}", "404", str(st2),
            "PASS" if st2 == 404 else "FAIL")
    # legit file
    st2, b2, _ = api("admin", "GET", f"/admin/dump/{did}/manifest.json")
    rec(S, "manifest.json download", "200", str(st2), "PASS" if st2 == 200 else "FAIL")
    st2, b2, _ = api("admin", "GET", f"/admin/dump/{did}/status.json")
    rec(S, "status.json download", "200", str(st2), "PASS" if st2 == 200 else "FAIL")

    # tamper manifest.json on disk then restore still works (manifest not verified)
    subprocess.run(["docker", "exec", "-i", "enginer-api", "sh", "-c",
                    f"echo '{{\"format\":\"enginer-dump/99\",\"tables\":{{}}}}' > /app/dumps/{did}/manifest.json"],
                   capture_output=True)

# ---- 3. crafted uploads + restores
def upload_and_restore(tag, payload_bytes, expect_upload):
    st, b, h = api("admin", "POST", "/admin/dump/upload", raw=payload_bytes)
    uid = (b.get("data") or {}).get("dump_id") if isinstance(b, dict) else None
    rec(S, f"upload {tag}", f"expect {expect_upload}", brief((st, b, h)),
        "PASS" if (st == 200) == (expect_upload == 200) else "FAIL")
    if uid:
        created_dumps.append(uid)
    if uid and expect_upload == 200:
        st2, b2, _ = api("admin", "POST", f"/admin/dump/{uid}/restore",
                         {"confirm": "RESTORE"})
        rec(S, f"restore {tag} start", "200 restoring or 5xx", brief((st2, b2, {})),
            "PASS" if st2 == 200 else "FAIL")
        fin = wait_done(uid)
        return uid, fin
    return uid, None

# (a) tampered header format (manifest-equivalent tamper in-band)
hdr99 = b'{"table":"_header","row":{"format":"enginer-dump/99","created_at":"x"}}\n'
uid, fin = upload_and_restore("unknown-format-99", hdr99, 200)
rec(S, "restore unknown format", "warns, succeeds (documented)", f"final={fin}",
    "SUSPICIOUS" if fin == "restored" else "FAIL")

# (b) garbage upload
upload_and_restore("garbage", b"not a dump at all\n", 422)
upload_and_restore("empty", b"", 422)
upload_and_restore("gzip-blob", b"\x1f\x8b\x08\x00fakegzip\n", 422)

# (c) decompression bomb: valid first line then ~60MB of '0' lines in xz
bomb_raw = hdr99 + b"0\n" * (30 * 1024 * 1024)
bomb = lzma.compress(bomb_raw, preset=1)
uid, fin = upload_and_restore(f"xz-bomb {len(bomb)}B->{len(bomb_raw)//1024//1024}MB",
                              bomb, 200)
if uid:
    rec(S, "restore xz-bomb", "graceful failure, no hang/crash", f"final={fin}",
        "PASS" if fin in ("failed", "restored") else "FAIL")

# (d) SQL injection payloads inside row values + hostile table names
inj_uuid = str(uuid.uuid4())
qa_item_type = sql("SELECT type_id FROM meta.type_definition WHERE stable_alias='qa_item' AND catalog_id=6 AND retired_revision IS NULL").strip()
lines = [
    {"table": "_header", "row": {"format": "enginer-dump/2"}},
    # injection string in a free-text column
    {"table": "core.entity", "row": {
        "tenant_id": TID, "entity_id": 99000001, "public_id": inj_uuid,
        "entity_class": 1, "catalog_id": 6, "type_id": qa_item_type,
        "head_revision": 1, "retire_reason": "'); DROP TABLE core.entity;--"}},
    # hostile table name -> schema allowlist / table resolution must refuse
    {"table": "core.entity;DROP TABLE core.entity", "row": {"x": 1}},
    # secret table smuggle attempt -> must be skipped on restore
    {"table": "sec.api_token", "row": {
        "token_id": str(uuid.uuid4()), "tenant_id": TID,
        "principal_id": sql("SELECT principal_id FROM sec.principal LIMIT 1"),
        "name": "smuggled", "token_hash": "deadbeef"}},
    # (e) cross-tenant row injection (tenant that does not exist locally)
    {"table": "core.entity", "row": {
        "tenant_id": "00000000-dead-0000-0000-0000000000aa",
        "entity_id": 99000002, "public_id": str(uuid.uuid4()),
        "entity_class": 1, "catalog_id": 6, "type_id": qa_item_type,
        "head_revision": 1}},
    # privilege escalation: add operator principal to admins group
    {"table": "sec.principal_group_membership", "row": {
        "tenant_id": TID,
        "principal_id": "01a150b4-9a06-7813-8cee-ac89aeab275e",
        "group_id": "00000000-0000-0000-0000-0000000000b1",
        "groups_revision": 999}},
]
payload = ("\n".join(json.dumps(l) for l in lines) + "\n").encode()
tok_before = sql("SELECT count(*) FROM sec.api_token")
caps_before = json.dumps(api("operator", "GET", "/me/context")[1].get("data", {}).get("capabilities"))
uid, fin = upload_and_restore("crafted-rows", payload, 200)
rec(S, "restore crafted-rows", "restored or failed", f"final={fin}", "PASS")
tok_after = sql("SELECT count(*) FROM sec.api_token")
rec(S, "sec.api_token not injectable", "count unchanged",
    f"before={tok_before} after={tok_after}",
    "PASS" if tok_before == tok_after else "FAIL")
inj = sql(f"SELECT retire_reason FROM core.entity WHERE entity_id=99000001")
rec(S, "SQLi payload stored literally", "literal text row, table intact",
    f"retire_reason={inj[:60]}",
    "PASS" if "DROP TABLE" in (inj or "") else "FAIL")
xt = sql(f"SELECT count(*) FROM core.entity WHERE entity_id=99000002")
rec(S, "cross-tenant row injection", "row for fabricated tenant_id landed",
    f"count={xt}", "SUSPICIOUS" if xt == "1" else "INFO")
mem = sql("SELECT count(*) FROM sec.principal_group_membership WHERE principal_id='01a150b4-9a06-7813-8cee-ac89aeab275e' AND group_id='00000000-0000-0000-0000-0000000000b1'")
caps_after = json.dumps(api("operator", "GET", "/me/context")[1].get("data", {}).get("capabilities"))
rec(S, "priv-esc via dump (operator->admins)", "membership injected; caps widen",
    f"membership_rows={mem} caps_gained={caps_before != caps_after}",
    "SUSPICIOUS" if mem == "1" else "INFO")
# cleanup injected rows
sql_i("DELETE FROM core.entity WHERE entity_id IN (99000001,99000002)")
sql_i("DELETE FROM sec.principal_group_membership WHERE principal_id='01a150b4-9a06-7813-8cee-ac89aeab275e' AND group_id='00000000-0000-0000-0000-0000000000b1'")

# restore without confirm -> 422
if did:
    st, b, _ = api("admin", "POST", f"/admin/dump/{did}/restore", {"confirm": "nope"})
    rec(S, "restore w/o confirm", "422", brief((st, b, {})),
        "PASS" if st == 422 else "FAIL")

# restore of a real dump into same db (idempotent merge)
if did and st_final == "done":
    st, b, _ = api("admin", "POST", f"/admin/dump/{did}/restore", {"confirm": "RESTORE"})
    rec(S, "restore real dump", "200 restoring", brief((st, b, {})), "PASS" if st == 200 else "FAIL")
    if st == 200:
        fin = wait_done(did)
        # concurrent dump during restore -> 409
        rec(S, "real restore finishes", "restored", fin, "PASS" if fin == "restored" else "FAIL")

# cleanup created dump dirs
for d in created_dumps:
    if d:
        st, b, _ = api("admin", "DELETE", f"/admin/dump/{d}")
        print(f"      cleanup dump {d}: {st}")

print("\n=== S2 DONE ===")
