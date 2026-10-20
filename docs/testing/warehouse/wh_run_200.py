"""Driver for the 200-prompt warehouse-manager scenario (wh_agent_prompts_200.md).

Executes every prompt in order through EngineR MCP, keeps code -> public_id
map, logs every tool call. Writes:
  .wh_objects_200.json   code -> public_id
  .wh_run_200_log.json   per-prompt structured log for the report
"""
import json
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import wh_call as wc  # noqa: E402

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

IDS = {}        # code -> public_id
PID2CODE = {}   # public_id -> code
LOG = []        # [{n, title, ops, ok, note}]
CUR = None      # current log record


def short_result(r):
    """Compact a tool response for the log."""
    if not isinstance(r, dict):
        return str(r)[:300]
    if wc.failed(r):
        return {"ERR": r.get("_rpc_err") or r.get("_http")
                or r.get("error") or r.get("_text") or r}
    d = r.get("data")
    if isinstance(d, dict):
        out = {}
        for k in ("public_id", "outcome", "revision", "predicate",
                  "relation_id", "entity_id"):
            if k in d:
                out[k] = d[k]
        if "fields" in d:
            out["fields"] = d["fields"]
        if "status" in d:
            out["status"] = d["status"]
        return out or {"data_keys": list(d.keys())}
    if isinstance(d, list):
        return {"count": len(d),
                "items": [item.get("public_id") or item.get("relation_id")
                          for item in d[:60]]}
    return r


def T(tool, args):
    """Tool call + append to current prompt log."""
    r = wc.call(tool, args)
    ok = not wc.failed(r)
    CUR["ops"].append({"tool": tool, "args": args, "ok": ok,
                       "result": short_result(r)})
    if not ok:
        CUR["ok"] = False
        print(f"  !! {tool} FAILED: {json.dumps(r, ensure_ascii=False)[:300]}")
    return r


def store(code, pid):
    IDS[code] = pid
    PID2CODE[pid] = code


def E(code, type_id, fields):
    """entities.create; registers code->public_id. Returns public_id."""
    r = T("entities.create", {"type_id": type_id, "fields": fields})
    pid = wc.pub_id(r)
    if pid:
        store(code, pid)
        oc = (r.get("data") or {}).get("outcome")
        print(f"  {code} ({type_id}) -> {pid} [{oc}]")
    return pid


def RE(a, pred, b, note=None):
    """relations.create between two registered codes (or raw pids)."""
    sa = IDS.get(a, a)
    sb = IDS.get(b, b)
    r = T("relations.create", {"id": sa, "predicate": pred,
                               "target_entity_id": sb})
    ok = not wc.failed(r)
    tag = "" if ok else "  <-- FAILED"
    print(f"  rel {a} -[{pred}]-> {b}{tag}")
    return ok


def UP(code, fields):
    """entities.update by code or pid."""
    pid = IDS.get(code, code)
    r = T("entities.update", {"id": pid, "fields": fields})
    if not wc.failed(r):
        print(f"  upd {code}: {json.dumps(fields, ensure_ascii=False)[:120]}")
    return r


def LS(type_alias, **kw):
    args = {"type": type_alias, "limit": 500}
    args.update(kw)
    r = T("entities.list", args)
    data = r.get("data") or []
    print(f"  list {type_alias}: {len(data)} entities")
    return data


def GET(code):
    pid = IDS.get(code, code)
    r = T("entities.get", {"id": pid})
    return (r.get("data") or {})


def RELS(code, direction="both"):
    pid = IDS.get(code, code)
    r = T("relations.list", {"id": pid, "direction": direction})
    data = r.get("data") or []
    print(f"  rels {code} [{direction}]: {len(data)} edges")
    return data


def HIST(code):
    pid = IDS.get(code, code)
    r = T("entities.history", {"id": pid})
    return r.get("data")


def Q(query):
    r = T("entities.query", {"query": query})
    return r


def name_of(rel_item):
    pv = {p["field"]: p["value"] for p in rel_item.get("preview") or []}
    for k in ("wh_code", "wh_zone_code", "wh_partner_code", "wh_sku",
              "wh_doc_no", "wh_waybill_no", "wh_payment_no", "wh_expense_no",
              "wh_hold_no", "wh_inv_no", "wh_contract_no", "wh_cargo_no",
              "wh_batch", "full_name", "inn", "name"):
        if k in pv:
            return pv[k]
    return rel_item.get("target_display_name")


def code_of(rel_item):
    return PID2CODE.get(rel_item.get("target_entity_id"),
                        rel_item.get("target_display_name"))


def step(n, title):
    global CUR
    CUR = {"n": n, "title": title, "ops": [], "ok": True, "note": ""}
    LOG.append(CUR)
    print(f"[{n}] {title}")


def note(s):
    CUR["note"] = (CUR["note"] + " " + s).strip()
    print(f"  .. {s}")


def find_by(data, key, val):
    for e in data:
        if str((e.get("fields") or {}).get(key)) == str(val):
            return e
    return None


# =====================================================================
def run():
    wc.init()
    print("== MCP session ready ==")

    # ---- Phase 1 -----------------------------------------------------
    step(1, "Склад СК-Центральный WH-01, Москва, 12000 м²")
    E("WH-01", "wh_warehouse",
      {"wh_code": "WH-01", "name": "СК-Центральный",
       "address": "г. Москва, ул. Складская 12", "wh_capacity_m2": 12000})

    step(2, "Зона приёмки Z-RCV-01 на WH-01")
    E("Z-RCV-01", "wh_zone", {"wh_zone_code": "Z-RCV-01",
                              "wh_zone_kind": "приёмка",
                              "wh_capacity_m2": 800})
    RE("Z-RCV-01", "wh_located_in", "WH-01")

    step(3, "Зона хранения Z-STO-A 4000 м²")
    E("Z-STO-A", "wh_zone", {"wh_zone_code": "Z-STO-A",
                             "wh_zone_kind": "хранение",
                             "wh_capacity_m2": 4000})
    RE("Z-STO-A", "wh_located_in", "WH-01")

    step(4, "Зона хранения Z-STO-B 3500 м²")
    E("Z-STO-B", "wh_zone", {"wh_zone_code": "Z-STO-B",
                             "wh_zone_kind": "хранение",
                             "wh_capacity_m2": 3500})
    RE("Z-STO-B", "wh_located_in", "WH-01")

    step(5, "Холодильная зона Z-COLD 600 м²")
    E("Z-COLD", "wh_zone", {"wh_zone_code": "Z-COLD",
                            "wh_zone_kind": "холодильная",
                            "wh_capacity_m2": 600})
    RE("Z-COLD", "wh_located_in", "WH-01")

    step(6, "Карантинная зона Z-QA 300 м²")
    E("Z-QA", "wh_zone", {"wh_zone_code": "Z-QA",
                          "wh_zone_kind": "карантин",
                          "wh_capacity_m2": 300})
    RE("Z-QA", "wh_located_in", "WH-01")

    step(7, "Зона отгрузки Z-SHIP 900 м²")
    E("Z-SHIP", "wh_zone", {"wh_zone_code": "Z-SHIP",
                            "wh_zone_kind": "отгрузка",
                            "wh_capacity_m2": 900})
    RE("Z-SHIP", "wh_located_in", "WH-01")

    step(8, "Склад СК-Север WH-02, СПб, 5000 м²")
    E("WH-02", "wh_warehouse",
      {"wh_code": "WH-02", "name": "СК-Север",
       "address": "г. Санкт-Петербург, Индустриальный пр. 44",
       "wh_capacity_m2": 5000})

    step(9, "Зоны WH-02: Z-N-RCV, Z-N-STO-A 2500 м², Z-N-SHIP")
    E("Z-N-RCV", "wh_zone", {"wh_zone_code": "Z-N-RCV",
                             "wh_zone_kind": "приёмка"})
    RE("Z-N-RCV", "wh_located_in", "WH-02")
    E("Z-N-STO-A", "wh_zone", {"wh_zone_code": "Z-N-STO-A",
                               "wh_zone_kind": "хранение",
                               "wh_capacity_m2": 2500})
    RE("Z-N-STO-A", "wh_located_in", "WH-02")
    E("Z-N-SHIP", "wh_zone", {"wh_zone_code": "Z-N-SHIP",
                              "wh_zone_kind": "отгрузка"})
    RE("Z-N-SHIP", "wh_located_in", "WH-02")

    step(10, "Все зоны WH-01")
    rels = RELS("WH-01", "incoming")
    zones = [code_of(x) for x in rels if x["predicate"] == "wh_located_in"]
    note("Зоны WH-01: " + ", ".join(zones))

    # ---- Phase 2 -----------------------------------------------------
    people = [
        (11, "P-KOZLOV", "Козлов Андрей Викторович", "772501010101", "WH-01"),
        (12, "P-IVANOV", "Иванов Пётр Сергеевич", "772455667788", "WH-01"),
        (13, "P-SMIRNOVA", "Смирнова Ольга Николаевна", "772511223344", "WH-01"),
        (14, "P-GROMOV", "Громов Денис Олегович", "772599887766", "WH-01"),
        (15, "P-PETROVA", "Петрова Марина Игоревна", "781234567890", "WH-02"),
        (16, "P-SOKOLOV", "Соколов Игорь Павлович", "781209876543", "WH-02"),
    ]
    for n, code, fio, inn, wh in people:
        step(n, f"Приём на работу {fio} -> {wh}")
        E(code, "person", {"full_name": fio, "inn": inn})
        RE(code, "wh_keeper", wh)

    step(17, "Дубликат person Иванов П.С. — проверка дедупликации")
    r = T("entities.create", {"type_id": "person",
                              "fields": {"full_name": "Иванов Пётр Сергеевич",
                                         "inn": "772455667788"}})
    pid = wc.pub_id(r)
    oc = (r.get("data") or {}).get("outcome")
    note(f"outcome={oc}, public_id={pid} (совпадает с P-IVANOV: "
         f"{pid == IDS.get('P-IVANOV')})")

    step(18, "Телефон Иванова +7 926 111-22-33")
    UP("P-IVANOV", {"wh_phone": "+7 926 111-22-33"})

    step(19, "Карточка завскладом Козлова")
    GET("P-KOZLOV")
    RELS("P-KOZLOV", "both")

    step(20, "Все сотрудники WH-01")
    rels = RELS("WH-01", "incoming")
    staff = [code_of(x) for x in rels if x["predicate"] == "wh_keeper"]
    note("Персонал WH-01: " + ", ".join(staff))

    # ---- Phase 3 -----------------------------------------------------
    step(21, "Юрлицо ООО МетизСнаб 7701123456")
    E("LE-METIZ", "legal_entity",
      {"name": "ООО МетизСнаб", "inn": "7701123456",
       "wh_phone": "+7 495 111-22-33", "wh_contact": "Смирнов А."})

    step(22, "Юрлицо АО ЛКМ-Групп 7702234567")
    E("LE-LKM", "legal_entity", {"name": "АО ЛКМ-Групп", "inn": "7702234567"})

    step(23, "Юрлицо ИП ЛесТорг 5003345678")
    E("LE-LES", "legal_entity", {"name": "ИП ЛесТорг", "inn": "5003345678"})

    step(24, "Юрлицо АО Молочный Дом 7703456789")
    E("LE-MOL", "legal_entity",
      {"name": "АО Молочный Дом", "inn": "7703456789"})

    step(25, "Карточка поставщика SUP-001 МетизСнаб")
    E("SUP-001", "wh_supplier",
      {"wh_partner_code": "SUP-001", "name": "ООО МетизСнаб",
       "inn": "7701123456", "wh_phone": "+7 495 111-22-33",
       "wh_contact": "Смирнов А."})

    step(26, "Карточка поставщика SUP-002 ЛКМ-Групп")
    E("SUP-002", "wh_supplier",
      {"wh_partner_code": "SUP-002", "name": "АО ЛКМ-Групп",
       "inn": "7702234567"})

    step(27, "Карточка поставщика SUP-003 ЛесТорг")
    E("SUP-003", "wh_supplier",
      {"wh_partner_code": "SUP-003", "name": "ИП ЛесТорг",
       "inn": "5003345678"})

    step(28, "Карточка поставщика SUP-004 Молочный Дом")
    E("SUP-004", "wh_supplier",
      {"wh_partner_code": "SUP-004", "name": "АО Молочный Дом",
       "inn": "7703456789"})

    step(29, "Юрлицо ООО СтройМонтаж 7713456789")
    E("LE-STROY", "legal_entity",
      {"name": "ООО СтройМонтаж", "inn": "7713456789"})

    step(30, "Юрлицо АО Ритейл Сити 7804567890")
    E("LE-RETAIL", "legal_entity",
      {"name": "АО Ритейл Сити", "inn": "7804567890"})

    step(31, "Юрлицо ООО ПромСклад 7709876543")
    E("LE-PROMSKLAD", "legal_entity",
      {"name": "ООО ПромСклад", "inn": "7709876543"})

    step(32, "Карточки клиентов CLI-001/002/003")
    E("CLI-001", "wh_client",
      {"wh_partner_code": "CLI-001", "name": "ООО СтройМонтаж",
       "inn": "7713456789"})
    E("CLI-002", "wh_client",
      {"wh_partner_code": "CLI-002", "name": "АО Ритейл Сити",
       "inn": "7804567890"})
    E("CLI-003", "wh_client",
      {"wh_partner_code": "CLI-003", "name": "ООО ПромСклад",
       "inn": "7709876543"})

    step(33, "Юрлицо-перевозчик ООО ТрансЛайн 7722001122")
    E("LE-TRANSLINE", "legal_entity",
      {"name": "ООО ТрансЛайн", "inn": "7722001122",
       "wh_phone": "+7 812 555-66-77"})

    step(34, "Юрлицо-перевозчик ИП Грузовоз 5033445566")
    E("LE-GRUZ", "legal_entity",
      {"name": "ИП Грузовоз", "inn": "5033445566"})

    step(35, "Карточки подрядчиков CON-001, CON-002")
    E("CON-001", "wh_contractor",
      {"wh_partner_code": "CON-001", "name": "ООО ТрансЛайн",
       "inn": "7722001122", "wh_phone": "+7 812 555-66-77"})
    E("CON-002", "wh_contractor",
      {"wh_partner_code": "CON-002", "name": "ИП Грузовоз",
       "inn": "5033445566"})

    step(36, "Юрлицо-арендодатель АО ПромНедвижимость")
    E("LE-PROMNED", "legal_entity",
      {"name": "АО ПромНедвижимость", "inn": "7705001234"})

    step(37, "Юрлицо-лизингодатель ООО ТехноЛизинг")
    E("LE-TEHNLIZ", "legal_entity",
      {"name": "ООО ТехноЛизинг", "inn": "7706002345"})

    step(38, "Юрлицо-страховщик АО Страховая Гарантия")
    E("LE-STRAH", "legal_entity",
      {"name": "АО Страховая Гарантия", "inn": "7707003456"})

    step(39, "Юрлицо-банк АО КредитПромБанк")
    E("LE-BANK", "legal_entity",
      {"name": "АО КредитПромБанк", "inn": "7708004567"})

    step(40, "Повторная регистрация SUP-001 — дедупликация")
    r = T("entities.create", {"type_id": "wh_supplier",
                              "fields": {"wh_partner_code": "SUP-001",
                                         "name": "ООО МетизСнаб",
                                         "inn": "7701123456"}})
    note(f"outcome={(r.get('data') or {}).get('outcome')}, "
         f"public_id={wc.pub_id(r)} (== SUP-001: "
         f"{wc.pub_id(r) == IDS.get('SUP-001')})")

    step(41, "Карточка юрлица МетизСнаб")
    GET("LE-METIZ")
    RELS("LE-METIZ", "both")

    step(42, "Все контрагенты с «ООО» в названии")
    data = LS("legal_entity", search="ООО")
    note("Найдено: " + ", ".join(e["display_name"] for e in data))

    # ---- Phase 4 -----------------------------------------------------
    def contract(code, kind, name, party, whs, date=None, until=None,
                 amount=None, status="действует"):
        f = {"wh_contract_no": code, "wh_contract_kind": kind,
             "name": name, "wh_currency": "RUB", "status_note": status}
        if date:
            f["wh_doc_date"] = date
        if until:
            f["wh_until"] = until
        if amount is not None:
            f["wh_amount"] = amount
        E(code, "wh_contract", f)
        RE(code, "wh_contract_party", party)
        for w in whs:
            RE(code, "wh_contract_wh", w)

    step(43, "Договор поставки CTR-P-001 МетизСнаб 12 000 000")
    contract("CTR-P-001", "поставка", "Договор поставки с ООО МетизСнаб",
             "LE-METIZ", ["WH-01"], "2026-10-01", "2027-09-30", 12000000)

    step(44, "Договор поставки CTR-P-002 ЛКМ-Групп 8 500 000")
    contract("CTR-P-002", "поставка", "Договор поставки с АО ЛКМ-Групп",
             "LE-LKM", ["WH-01"], "2026-10-01", "2027-09-30", 8500000)

    step(45, "Договор поставки CTR-P-003 ЛесТорг 6 000 000")
    contract("CTR-P-003", "поставка", "Договор поставки с ИП ЛесТорг",
             "LE-LES", ["WH-01"], "2026-10-05", None, 6000000)

    step(46, "Договор поставки CTR-P-004 Молочный Дом 3 200 000 (WH-01+WH-02)")
    contract("CTR-P-004", "поставка", "Договор поставки с АО Молочный Дом",
             "LE-MOL", ["WH-01", "WH-02"], "2026-10-05", None, 3200000)

    step(47, "Договор ответхранения CTR-S-001 ПромСклад 180 000/мес")
    contract("CTR-S-001", "хранение",
             "Ответственное хранение для ООО ПромСклад, 180000 RUB/мес",
             "LE-PROMSKLAD", ["WH-01"], "2026-10-10", "2027-10-09", 180000,
             "действует — тариф 180000 RUB/мес")

    step(48, "Договор хранения CTR-S-002 Ритейл Сити 95 000/мес на WH-02")
    contract("CTR-S-002", "хранение",
             "Сезонное хранение для АО Ритейл Сити, 95000 RUB/мес",
             "LE-RETAIL", ["WH-02"], "2026-10-10", None, 95000,
             "действует — тариф 95000 RUB/мес")

    step(49, "Договор аренды CTR-R-001 ПромНедвижимость WH-01 950 000/мес")
    contract("CTR-R-001", "аренда",
             "Аренда помещений WH-01, 950000 RUB/мес",
             "LE-PROMNED", ["WH-01"], "2026-10-01", "2030-12-31", 950000)

    step(50, "Договор аренды CTR-R-002 WH-02 380 000/мес")
    contract("CTR-R-002", "аренда", "Аренда помещений WH-02, 380000 RUB/мес",
             "LE-PROMNED", ["WH-02"], "2026-10-01", None, 380000)

    step(51, "Договор лизинга CTR-L-001 ТехноЛизинг 5 400 000")
    contract("CTR-L-001", "лизинг",
             "Лизинг двух вилочных погрузчиков, 150000 RUB/мес",
             "LE-TEHNLIZ", ["WH-01"], "2026-10-01", None, 5400000)

    step(52, "Договор страхования CTR-I-001 Страховая Гарантия 60 000 000")
    contract("CTR-I-001", "страхование",
             "Страхование товарных запасов WH-01, премия 480000 RUB/год",
             "LE-STRAH", ["WH-01"], "2026-10-01", None, 60000000)

    step(53, "Кредитный договор CTR-C-001 КредитПромБанк 20 000 000, 16%")
    contract("CTR-C-001", "кредит",
             "Кредитная линия 20000000 RUB на закупку товара, 16% годовых",
             "LE-BANK", ["WH-01"], "2026-10-01", None, 20000000)

    step(54, "Договор транспорта CTR-T-001 ТрансЛайн")
    contract("CTR-T-001", "транспорт",
             "Транспортные услуги по поставкам и отгрузкам WH-01",
             "LE-TRANSLINE", ["WH-01"], "2026-10-01")

    step(55, "Все договоры склада WH-01")
    rels = RELS("WH-01", "incoming")
    ctrs = [code_of(x) for x in rels if x["predicate"] == "wh_contract_wh"]
    note("Договоры WH-01: " + ", ".join(ctrs))

    step(56, "Карточка договора CTR-S-001")
    GET("CTR-S-001")
    rels = RELS("CTR-S-001", "both")
    note("Связи: " + "; ".join(
        f"{x['predicate']}->{code_of(x)}" for x in rels))

    # ---- Phase 5 -----------------------------------------------------
    prods = [
        (57, "SKU-101", "Гвозди строительные 120мм", "кг", "метизы"),
        (58, "SKU-102", "Саморезы по дереву 4х50", "кг", "метизы"),
        (59, "SKU-103", "Краска фасадная белая", "литры", "ЛКМ"),
        (60, "SKU-104", "Эмаль ПФ-115", "литры", "ЛКМ"),
        (61, "SKU-105", "Доска обрезная 40х150х6000", "м³", "пиломатериалы"),
        (62, "SKU-106", "Брус 100х100х6000", "м³", "пиломатериалы"),
        (63, "SKU-107", "Клей монтажный универсальный", "кг", "химия"),
        (64, "SKU-108", "Плёнка стретч 500мм", "рулоны", "упаковка"),
        (65, "SKU-109", "Молоко 3.2% 1л", "штуки", "молочная продукция"),
        (66, "SKU-110", "Творог 9% 200г", "штуки", "молочная продукция"),
    ]
    for n, sku, name, unit, cat in prods:
        step(n, f"Товар {name} ({sku})")
        E(sku, "wh_product", {"wh_sku": sku, "name": name,
                              "wh_unit": unit, "wh_category": cat})

    step(67, "Привязка поставщиков к товарам")
    for sku in ("SKU-101", "SKU-102", "SKU-108"):
        RE(sku, "wh_supplied_by", "LE-METIZ")
        RE(sku, "wh_product_supplier", "SUP-001")
    for sku in ("SKU-103", "SKU-104", "SKU-107"):
        RE(sku, "wh_supplied_by", "LE-LKM")
        RE(sku, "wh_product_supplier", "SUP-002")
    for sku in ("SKU-105", "SKU-106"):
        RE(sku, "wh_supplied_by", "LE-LES")
        RE(sku, "wh_product_supplier", "SUP-003")
    for sku in ("SKU-109", "SKU-110"):
        RE(sku, "wh_supplied_by", "LE-MOL")
        RE(sku, "wh_product_supplier", "SUP-004")

    step(68, "Все товары категории «метизы»")
    r = Q({"from": {"type_id": "wh_product"},
           "filters": {"wh_category": "метизы"},
           "page": {"size": 100}})
    note(json.dumps(short_result(r), ensure_ascii=False)[:500])

    step(69, "Товар SKU-105 и его поставщики")
    data = LS("wh_product", search="SKU-105")
    if data:
        store("SKU-105", data[0]["public_id"])
    GET("SKU-105")
    rels = RELS("SKU-105", "both")
    note("Поставщики: " + "; ".join(
        f"{x['predicate']}->{code_of(x)}" for x in rels))

    step(70, "Полный товарный каталог")
    data = LS("wh_product")
    note("Товаров: " + str(len(data)) + " — " + ", ".join(
        (e.get("fields") or {}).get("wh_sku", "?") for e in data))

    # ---- Phase 6 -----------------------------------------------------
    def inbound(code, le, sup, ctr, wh, date, status):
        E(code, "wh_inbound", {"wh_doc_no": code, "wh_doc_date": date,
                               "status_note": status})
        RE(code, "wh_inbound_to", wh)
        if le:
            RE(code, "wh_inbound_from", le)
        if sup:
            RE(code, "wh_inbound_supplier", sup)
        if ctr:
            RE(code, "wh_inbound_contract", ctr)

    def waybill(code, date, amount, target, direction, carrier=None,
                due=None, status=None):
        f = {"wh_waybill_no": code, "wh_doc_date": date,
             "wh_amount": amount, "wh_currency": "RUB"}
        if due:
            f["wh_due_date"] = due
        if status:
            f["status_note"] = status
        E(code, "wh_waybill", f)
        RE(code, direction, target)
        if carrier:
            RE(code, "wh_waybill_carrier", carrier)

    def stock(code, sku, qty, zone, ctr=None):
        E(code, "wh_stock", {"wh_batch": code, "wh_qty": qty})
        RE(code, "wh_stock_product", sku)
        RE(code, "wh_stock_zone", zone)
        if ctr:
            RE(code, "wh_stock_contract", ctr)

    step(71, "Поставка INB-001 МетизСнаб -> WH-01 по CTR-P-001, ожидается")
    inbound("INB-001", "LE-METIZ", "SUP-001", "CTR-P-001", "WH-01",
            "2026-10-15", "ожидается")

    step(72, "Позиции INB-001: гвозди, саморезы, плёнка")
    RE("INB-001", "wh_inbound_item", "SKU-101")
    RE("INB-001", "wh_inbound_item", "SKU-102")
    RE("INB-001", "wh_inbound_item", "SKU-108")

    step(73, "Накладная WB-001 на INB-001, 450000, перевозчик ТрансЛайн")
    waybill("WB-001", "2026-10-15", 450000, "INB-001", "wh_waybill_inbound",
            carrier="CON-001", due="2026-10-29")

    step(74, "Грузовые места CG-001, CG-002 по WB-001")
    E("CG-001", "wh_cargo", {"wh_cargo_no": "CG-001",
                             "name": "Паллета гвоздей",
                             "wh_weight_kg": 1200, "wh_volume_m3": 1.6})
    RE("CG-001", "wh_cargo_on", "WB-001")
    E("CG-002", "wh_cargo", {"wh_cargo_no": "CG-002",
                             "name": "Рулоны плёнки",
                             "wh_weight_kg": 180, "wh_volume_m3": 0.9})
    RE("CG-002", "wh_cargo_on", "WB-001")

    step(75, "INB-001 принята; грузы в зону приёмки Z-RCV-01")
    UP("INB-001", {"status_note": "принята"})
    RE("CG-001", "wh_cargo_in", "Z-RCV-01")
    RE("CG-002", "wh_cargo_in", "Z-RCV-01")

    step(76, "Размещение: B-101 гвозди 5000 кг, B-102 саморезы 3200 кг, "
             "B-103 плёнка 400 рул. — Z-STO-A")
    stock("B-101", "SKU-101", 5000, "Z-STO-A")
    stock("B-102", "SKU-102", 3200, "Z-STO-A")
    stock("B-103", "SKU-108", 400, "Z-STO-A")

    step(77, "Поставка INB-002 ЛКМ по CTR-P-002: краска, эмаль -> Z-STO-B")
    inbound("INB-002", "LE-LKM", "SUP-002", "CTR-P-002", "WH-01",
            "2026-10-16", "принята")
    RE("INB-002", "wh_inbound_item", "SKU-103")
    RE("INB-002", "wh_inbound_item", "SKU-104")
    waybill("WB-002", "2026-10-16", 380000, "INB-002", "wh_waybill_inbound",
            carrier="CON-001")
    stock("B-104", "SKU-103", 900, "Z-STO-B")
    stock("B-105", "SKU-104", 600, "Z-STO-B")

    step(78, "Поставка INB-003 ЛесТорг по CTR-P-003: доска, брус -> Z-STO-B")
    inbound("INB-003", "LE-LES", "SUP-003", "CTR-P-003", "WH-01",
            "2026-10-17", "принята")
    RE("INB-003", "wh_inbound_item", "SKU-105")
    RE("INB-003", "wh_inbound_item", "SKU-106")
    waybill("WB-003", "2026-10-17", 720000, "INB-003", "wh_waybill_inbound",
            carrier="CON-002")
    stock("B-106", "SKU-105", 45, "Z-STO-B")
    stock("B-107", "SKU-106", 20, "Z-STO-B")

    step(79, "Поставка INB-004 Молочный Дом по CTR-P-004: молоко, творог "
             "-> Z-COLD")
    inbound("INB-004", "LE-MOL", "SUP-004", "CTR-P-004", "WH-01",
            "2026-10-18", "принята")
    RE("INB-004", "wh_inbound_item", "SKU-109")
    RE("INB-004", "wh_inbound_item", "SKU-110")
    waybill("WB-004", "2026-10-18", 150000, "INB-004", "wh_waybill_inbound",
            carrier="CON-001")
    stock("B-108", "SKU-109", 2000, "Z-COLD")
    stock("B-109", "SKU-110", 1500, "Z-COLD")

    step(80, "Поставка INB-005 ПромСклад (грузовладелец) по CTR-S-001 — "
             "ответхранение")
    inbound("INB-005", "LE-PROMSKLAD", None, "CTR-S-001", "WH-01",
            "2026-10-18", "принята на хранение")

    step(81, "Товар SKU-201 «Линия упаковки ЛУ-200»")
    E("SKU-201", "wh_product", {"wh_sku": "SKU-201",
                                "name": "Линия упаковки ЛУ-200",
                                "wh_unit": "штуки",
                                "wh_category": "оборудование — ответхранение"})
    RE("INB-005", "wh_inbound_item", "SKU-201")

    step(82, "Остаток B-PS-01: 2 шт ЛУ-200 в Z-STO-B под CTR-S-001")
    stock("B-PS-01", "SKU-201", 2, "Z-STO-B", ctr="CTR-S-001")

    step(83, "Поставка INB-006 Молочный Дом -> WH-02 по CTR-P-004")
    inbound("INB-006", "LE-MOL", "SUP-004", "CTR-P-004", "WH-02",
            "2026-10-19", "принята")
    RE("INB-006", "wh_inbound_item", "SKU-109")
    waybill("WB-006", "2026-10-19", 72000, "INB-006", "wh_waybill_inbound",
            carrier="CON-002")
    stock("B-N-101", "SKU-109", 1200, "Z-N-STO-A")

    step(84, "Поставка INB-007 МетизСнаб по CTR-P-001 — повторный завоз "
             "саморезов")
    inbound("INB-007", "LE-METIZ", "SUP-001", "CTR-P-001", "WH-01",
            "2026-10-19", "ожидается")
    RE("INB-007", "wh_inbound_item", "SKU-102")
    waybill("WB-007", "2026-10-19", 210000, "INB-007", "wh_waybill_inbound")

    step(85, "INB-007 принята частично: 2600 кг вместо 3000 — претензия")
    stock("B-110", "SKU-102", 2600, "Z-STO-A")
    UP("INB-007", {"status_note": "принята частично, недостача 400 кг — "
                                  "претензия поставщику"})

    step(86, "Груз CG-003 «Паллета эмали» по WB-002 -> Z-STO-B")
    E("CG-003", "wh_cargo", {"wh_cargo_no": "CG-003",
                             "name": "Паллета эмали",
                             "wh_weight_kg": 950, "wh_volume_m3": 1.1})
    RE("CG-003", "wh_cargo_on", "WB-002")
    RE("CG-003", "wh_cargo_in", "Z-STO-B")

    step(87, "Поставка INB-008 ЛКМ по CTR-P-002 — в пути, задержка")
    inbound("INB-008", "LE-LKM", "SUP-002", "CTR-P-002", "WH-01",
            "2026-10-20", "в пути — задержка перевозчика")
    RE("INB-008", "wh_inbound_item", "SKU-104")

    step(88, "Все поставки WH-01 со статусами")
    rels = RELS("WH-01", "incoming")
    inb = [x for x in rels if x["predicate"] == "wh_inbound_to"]
    for x in inb:
        d = GET(x["target_entity_id"])
        note(f"{name_of(x)}: {(d.get('fields') or {}).get('status_note')}")

    step(89, "Карточка поставки INB-001")
    GET("INB-001")
    rels = RELS("INB-001", "both")
    note("Связи: " + "; ".join(
        f"{x['predicate']}->{code_of(x)}" for x in rels))

    step(90, "Все грузовые места и их расположение")
    data = LS("wh_cargo")
    for e in data:
        pid = e["public_id"]
        store((e.get("fields") or {}).get("wh_cargo_no", pid), pid)
        rs = T("relations.list", {"id": pid, "direction": "outgoing"})
        locs = [name_of(x) for x in (rs.get("data") or [])
                if x["predicate"] == "wh_cargo_in"]
        on = [name_of(x) for x in (rs.get("data") or [])
              if x["predicate"] == "wh_cargo_on"]
        note(f"{(e.get('fields') or {}).get('wh_cargo_no')}: "
             f"зона={locs}, накладная={on}")

    # ---- Phase 7 -----------------------------------------------------
    step(91, "Все остатки WH-01: товар, партия, количество, зона")
    data = LS("wh_stock")
    for e in data:
        pid = e["public_id"]
        store((e.get("fields") or {}).get("wh_batch", pid), pid)
        rs = T("relations.list", {"id": pid, "direction": "outgoing"})
        prod = [name_of(x) for x in (rs.get("data") or [])
                if x["predicate"] == "wh_stock_product"]
        zone = [name_of(x) for x in (rs.get("data") or [])
                if x["predicate"] == "wh_stock_zone"]
        note(f"{(e.get('fields') or {}).get('wh_batch')}: "
             f"{prod} x {(e.get('fields') or {}).get('wh_qty')} в {zone}")

    step(92, "Остатки по зоне Z-STO-A")
    rels = RELS("Z-STO-A", "incoming")
    note("Партии: " + ", ".join(name_of(x) for x in rels
                                if x["predicate"] == "wh_stock_zone"))

    step(93, "Остатки по зоне Z-COLD")
    rels = RELS("Z-COLD", "incoming")
    note("Партии: " + ", ".join(name_of(x) for x in rels
                                if x["predicate"] == "wh_stock_zone"))

    def payment(code, date, amount, direction, wb=None, client=None,
                sup=None, ctr=None, status=None):
        f = {"wh_payment_no": code, "wh_doc_date": date,
             "wh_amount": amount, "wh_currency": "RUB",
             "wh_direction": direction}
        if status:
            f["status_note"] = status
        E(code, "wh_payment", f)
        if wb:
            RE(code, "wh_payment_for", wb)
        if client:
            RE(code, "wh_payment_client", client)
        if sup:
            RE(code, "wh_payment_supplier", sup)
        if ctr:
            RE(code, "wh_payment_contract", ctr)

    step(94, "Начисление хранения октябрь CTR-S-001: PAY-S-001 180000 вх.")
    payment("PAY-S-001", "2026-10-31", 180000, "входящая",
            client="CLI-003", ctr="CTR-S-001",
            status="начислено за хранение, октябрь")

    step(95, "Начисление хранения октябрь CTR-S-002: PAY-S-002 95000 вх.")
    payment("PAY-S-002", "2026-10-31", 95000, "входящая",
            client="CLI-002", ctr="CTR-S-002",
            status="начислено за хранение, октябрь")

    def expense(code, date, amount, cat, wh):
        E(code, "wh_expense", {"wh_expense_no": code, "wh_doc_date": date,
                               "wh_amount": amount, "wh_category": cat})
        RE(code, "wh_expense_wh", wh)

    step(96, "Издержка EXP-001 коммуналка WH-01 42000")
    expense("EXP-001", "2026-10-31", 42000,
            "коммунальные услуги WH-01, октябрь", "WH-01")

    step(97, "Издержка EXP-002 электроэнергия холодильной зоны 18500")
    expense("EXP-002", "2026-10-31", 18500,
            "электроэнергия холодильной зоны", "WH-01")

    step(98, "Издержка EXP-003 ФОТ складского персонала 640000")
    expense("EXP-003", "2026-10-31", 640000,
            "ФОТ складского персонал, октябрь", "WH-01")

    step(99, "Все издержки WH-01 за месяц")
    rels = RELS("WH-01", "incoming")
    exps = [name_of(x) for x in rels if x["predicate"] == "wh_expense_wh"]
    note("Издержки WH-01: " + ", ".join(exps))

    step(100, "Оплаты по договорам хранения (вх./исх.)")
    for c in ("CTR-S-001", "CTR-S-002"):
        rels = RELS(c, "incoming")
        pays = [name_of(x) for x in rels
                if x["predicate"] == "wh_payment_contract"]
        note(f"{c}: платежи {pays}")

    # ---- Phase 8 -----------------------------------------------------
    def outbound(code, wh, le, cli, date, status):
        E(code, "wh_outbound", {"wh_doc_no": code, "wh_doc_date": date,
                                "status_note": status})
        RE(code, "wh_outbound_from", wh)
        if le:
            RE(code, "wh_outbound_to", le)
        if cli:
            RE(code, "wh_outbound_client", cli)

    step(101, "Отгрузка OUT-001 WH-01 -> СтройМонтаж: гвозди 2000 кг, "
              "доска 15 м³; WB-101 390000")
    outbound("OUT-001", "WH-01", "LE-STROY", "CLI-001", "2026-10-19",
             "отгружена")
    RE("OUT-001", "wh_outbound_item", "SKU-101")
    RE("OUT-001", "wh_outbound_item", "SKU-105")
    waybill("WB-101", "2026-10-19", 390000, "OUT-001", "wh_waybill_outbound")

    step(102, "OUT-001 увозит ТрансЛайн — carrier на отгрузке и накладной")
    RE("OUT-001", "wh_outbound_carrier", "CON-001")
    RE("WB-101", "wh_waybill_carrier", "CON-001")

    step(103, "Отгрузка OUT-002 -> Ритейл Сити: молоко 800, творог 500; "
              "WB-102 96000; Грузовоз")
    outbound("OUT-002", "WH-01", "LE-RETAIL", "CLI-002", "2026-10-19",
             "отгружена")
    RE("OUT-002", "wh_outbound_item", "SKU-109")
    RE("OUT-002", "wh_outbound_item", "SKU-110")
    waybill("WB-102", "2026-10-19", 96000, "OUT-002", "wh_waybill_outbound",
            carrier="CON-002")
    RE("OUT-002", "wh_outbound_carrier", "CON-002")

    step(104, "Отгрузка OUT-003 -> СтройМонтаж: краска 300 л, клей 120 кг; "
              "WB-103; получатель физлицо Титов")
    outbound("OUT-003", "WH-01", "LE-STROY", "CLI-001", "2026-10-20",
             "отгружена")
    RE("OUT-003", "wh_outbound_item", "SKU-103")
    RE("OUT-003", "wh_outbound_item", "SKU-107")
    waybill("WB-103", "2026-10-20", 128000, "OUT-003", "wh_waybill_outbound")
    E("P-TITOV", "person", {"full_name": "Титов Сергей Иванович",
                            "inn": "772977665544"})
    RE("OUT-003", "wh_outbound_receiver", "P-TITOV")

    step(105, "Отгрузка OUT-004 WH-02 -> Ритейл Сити: молоко 600 шт; WB-104")
    outbound("OUT-004", "WH-02", "LE-RETAIL", "CLI-002", "2026-10-20",
             "отгружена")
    RE("OUT-004", "wh_outbound_item", "SKU-109")
    waybill("WB-104", "2026-10-20", 43000, "OUT-004", "wh_waybill_outbound",
            carrier="CON-002")
    RE("OUT-004", "wh_outbound_carrier", "CON-002")

    step(106, "Списание: B-101 -> 3000 кг, B-106 -> 30 м³")
    UP("B-101", {"wh_qty": 3000})
    UP("B-106", {"wh_qty": 30})

    step(107, "Списание: B-108 -> 1200, B-109 -> 1000, B-N-101 -> 600")
    UP("B-108", {"wh_qty": 1200})
    UP("B-109", {"wh_qty": 1000})
    UP("B-N-101", {"wh_qty": 600})

    step(108, "Списание: B-104 -> 600; остаток клея B-111 350 кг -> 230 кг")
    UP("B-104", {"wh_qty": 600})
    stock("B-111", "SKU-107", 350, "Z-STO-A")
    UP("B-111", {"wh_qty": 230})
    note("Клей заведён отдельным остатком B-111 (приход от ЛКМ по "
         "регистрации, без отдельной поставки INB-009 — упрощено сценарием).")

    step(109, "Отгрузка OUT-005 -> ПромСклад: выдача 1 шт ЛУ-200 с "
              "ответхранения; WB-105 0 RUB; получатель Орлов")
    outbound("OUT-005", "WH-01", "LE-PROMSKLAD", "CLI-003", "2026-10-20",
             "отгружена — выдача с ответхранения")
    RE("OUT-005", "wh_outbound_item", "SKU-201")
    waybill("WB-105", "2026-10-20", 0, "OUT-005", "wh_waybill_outbound",
            status="акт выдачи")
    RE("OUT-005", "wh_outbound_carrier", "CON-001")
    RE("WB-105", "wh_waybill_carrier", "CON-001")
    E("P-ORLOV", "person", {"full_name": "Орлов К. Д.",
                            "inn": "772900112233"})
    RE("OUT-005", "wh_outbound_receiver", "P-ORLOV")

    step(110, "Остаток B-PS-01 -> 1 шт")
    UP("B-PS-01", {"wh_qty": 1})

    step(111, "Отгрузка OUT-006 -> СтройМонтаж, запланирована; WB-106 черновик")
    outbound("OUT-006", "WH-01", "LE-STROY", "CLI-001", "2026-10-21",
             "запланирована")
    RE("OUT-006", "wh_outbound_item", "SKU-106")
    RE("OUT-006", "wh_outbound_item", "SKU-102")
    waybill("WB-106", "2026-10-21", 0, "OUT-006", "wh_waybill_outbound",
            status="черновик")

    step(112, "Отгрузка OUT-007 -> Ритейл Сити задержана: не подан транспорт")
    outbound("OUT-007", "WH-01", "LE-RETAIL", "CLI-002", "2026-10-19",
             "задержана: не подан транспорт")
    RE("OUT-007", "wh_outbound_item", "SKU-104")
    waybill("WB-107", "2026-10-19", 0, "OUT-007", "wh_waybill_outbound",
            status="не выставлена — отгрузка задержана")

    step(113, "Грузы отгрузок CG-101, CG-102 (WB-101), CG-103 (WB-102) "
              "-> Z-SHIP")
    for cg, wb in (("CG-101", "WB-101"), ("CG-102", "WB-101"),
                   ("CG-103", "WB-102")):
        nm = {"CG-101": "Паллета гвоздей (отгрузка)",
              "CG-102": "Паллета гвоздей (отгрузка)",
              "CG-103": "Термобокс молочки"}[cg]
        E(cg, "wh_cargo", {"wh_cargo_no": cg, "name": nm})
        RE(cg, "wh_cargo_on", wb)
        RE(cg, "wh_cargo_in", "Z-SHIP")

    step(114, "Все отгрузки WH-01 за период")
    rels = RELS("WH-01", "incoming")
    outs = [x for x in rels if x["predicate"] == "wh_outbound_from"]
    for x in outs:
        pid = x["target_entity_id"]
        d = GET(pid)
        rs = T("relations.list", {"id": pid, "direction": "outgoing"})
        car = [name_of(y) for y in (rs.get("data") or [])
               if y["predicate"] == "wh_outbound_carrier"]
        rcv = [name_of(y) for y in (rs.get("data") or [])
               if y["predicate"] == "wh_outbound_receiver"]
        note(f"{name_of(x)}: {(d.get('fields') or {}).get('status_note')}; "
             f"перевозчик={car}; получатель={rcv}")

    step(115, "Карточка отгрузки OUT-003")
    GET("OUT-003")
    rels = RELS("OUT-003", "both")
    note("Связи: " + "; ".join(
        f"{x['predicate']}->{code_of(x)}" for x in rels))

    # ---- Phase 9 -----------------------------------------------------
    step(116, "Претензия по INB-007: PAY-007 210000 исх., удержание "
              "HLD-001 28000")
    payment("PAY-007", "2026-10-19", 210000, "исходящая",
            wb="WB-007", sup="SUP-001", ctr="CTR-P-001",
            status="оплата поставщику по WB-007")
    E("HLD-001", "wh_hold",
      {"wh_hold_no": "HLD-001", "wh_amount": 28000,
       "wh_reason": "недопоставка по накладной WB-007 (сторона: поставщик "
                    "SUP-001 МетизСнаб)", "wh_until": "2026-11-15"})
    RE("HLD-001", "wh_hold_on", "PAY-007")
    r = T("relations.create",
          {"id": IDS["HLD-001"], "predicate": "wh_hold_party",
           "target_entity_id": IDS["SUP-001"]})
    if wc.failed(r):
        note("wh_hold_party -> wh_supplier отклонено схемой "
             "(range=wh_client) — ожидаемое ограничение; сторона указана "
             "в тексте причины.")

    step(117, "WB-101 просрочена — напоминание СтройМонтажу")
    UP("WB-101", {"status_note": "просрочена, звонок 2026-10-20"})

    step(118, "Карантин: 150 шт молока B-108 -> B-108-Q в Z-QA; "
              "основной -> 1050")
    stock("B-108-Q", "SKU-109", 150, "Z-QA")
    UP("B-108", {"wh_qty": 1050})

    step(119, "Претензия по браку: PAY-004 150000 исх., CLI-004 для "
              "Молочный Дом, удержание HLD-002 9000")
    payment("PAY-004", "2026-10-18", 150000, "исходящая",
            wb="WB-004", sup="SUP-004", ctr="CTR-P-004",
            status="оплата поставщику по WB-004")
    E("CLI-004", "wh_client",
      {"wh_partner_code": "CLI-004", "name": "АО Молочный Дом",
       "inn": "7703456789"})
    E("HLD-002", "wh_hold",
      {"wh_hold_no": "HLD-002", "wh_amount": 9000,
       "wh_reason": "брак упаковки, возврат (карантин B-108-Q)",
       "wh_until": "2026-11-15"})
    RE("HLD-002", "wh_hold_on", "PAY-004")
    RE("HLD-002", "wh_hold_party", "CLI-004")

    step(120, "INB-008 — второй день задержки, претензия перевозчику")
    UP("INB-008", {"status_note": "задержка, новая дата 2026-10-22, "
                                  "претензия перевозчику ТрансЛайн по "
                                  "CTR-T-001"})

    step(121, "Штраф перевозчику: PAY-T-001 60000 исх. по CTR-T-001, "
              "HLD-003 15000")
    payment("PAY-T-001", "2026-10-20", 60000, "исходящая",
            ctr="CTR-T-001",
            status="оплата транспортных услуг по CTR-T-001")
    E("HLD-003", "wh_hold",
      {"wh_hold_no": "HLD-003", "wh_amount": 15000,
       "wh_reason": "штраф за опоздание INB-008; сторона — перевозчик "
                    "ТрансЛайн (CON-001), фиксируется на платеже по "
                    "договору CTR-T-001", "wh_until": "2026-11-15"})
    RE("HLD-003", "wh_hold_on", "PAY-T-001")
    note("wh_hold_party не использовано: перевозчик (wh_contractor) вне "
         "range wh_client; сторона отражена в причине.")

    step(122, "Акт инвентаризации INV-001: недостача гвоздей -60 кг, WH-01")
    E("INV-001", "wh_inventory",
      {"wh_inv_no": "INV-001", "wh_doc_date": "2026-10-20",
       "wh_diff_qty": -60})
    RE("INV-001", "wh_counted_in", "WH-01")

    step(123, "Недостача списана на кладовщика Иванова; EXP-004 4800")
    UP("INV-001", {"status_note": "недостача 60 кг гвоздей (B-101, Z-STO-A) "
                                  "списана на МОЛ — кладовщик Иванов П.С."})
    expense("EXP-004", "2026-10-20", 4800,
            "недостача к удержанию из ФОТ (Иванов П.С.)", "WH-01")

    step(124, "Возврат 100 шт молока от Ритейл Сити: B-108 -> 1150, "
              "WB-102 частичный возврат")
    UP("B-108", {"wh_qty": 1150})
    UP("WB-102", {"status_note": "частичный возврат 100 шт"})

    step(125, "Частичная оплата WB-103: PAY-103 100000 из 128000")
    payment("PAY-103", "2026-10-20", 100000, "входящая",
            wb="WB-103", client="CLI-001",
            status="частичная оплата, долг 28000")
    UP("WB-103", {"status_note": "оплачена частично, долг 28000"})

    step(126, "Дебиторка СтройМонтаж: PAY-101-EXP черновик, HLD-004 390000")
    payment("PAY-101-EXP", "2026-10-20", 390000, "входящая (ожидается)",
            wb="WB-101", client="CLI-001", status="черновик — ожидается")
    E("HLD-004", "wh_hold",
      {"wh_hold_no": "HLD-004", "wh_amount": 390000,
       "wh_reason": "дебиторка по WB-101, отгрузки приостановлены"})
    RE("HLD-004", "wh_hold_on", "PAY-101-EXP")
    RE("HLD-004", "wh_hold_party", "CLI-001")

    step(127, "OUT-006 заблокирована до оплаты WB-101")
    UP("OUT-006", {"status_note": "заблокирована до оплаты WB-101"})

    step(128, "Допоставка INB-010: саморезы 400 кг (B-112), HLD-001 закрыто")
    inbound("INB-010", "LE-METIZ", "SUP-001", "CTR-P-001", "WH-01",
            "2026-10-21", "принята (допоставка по претензии INB-007)")
    RE("INB-010", "wh_inbound_item", "SKU-102")
    stock("B-112", "SKU-102", 400, "Z-STO-A")
    UP("HLD-001", {"wh_reason": "недопоставка по накладной WB-007 — ЗАКРЫТО "
                                "2026-10-21: поставщик довёз 400 кг "
                                "(INB-010, партия B-112)"})

    step(129, "Карантин B-108-Q решён: утилизация EXP-005 9000, остаток -> 0")
    expense("EXP-005", "2026-10-21", 9000,
            "утилизация брака (молоко B-108-Q, 150 шт)", "WH-01")
    UP("B-108-Q", {"wh_qty": 0})
    UP("HLD-002", {"wh_reason": "брак упаковки, возврат (B-108-Q) — ЗАКРЫТО "
                                "2026-10-21: товар утилизирован поставщиком, "
                                "издержка EXP-005"})

    step(130, "Все открытые удержания и претензии")
    data = LS("wh_hold")
    for e in data:
        f = e.get("fields") or {}
        store(f.get("wh_hold_no", e["public_id"]), e["public_id"])
        closed = "ЗАКРЫТО" in (f.get("wh_reason") or "")
        note(f"{f.get('wh_hold_no')}: {f.get('wh_amount')} RUB — "
             f"{'закрыто' if closed else 'ОТКРЫТО'}; {f.get('wh_reason')}")

    step(131, "Проблемный реестр: задержано/просрочено/частично")
    for t in ("wh_inbound", "wh_outbound", "wh_waybill"):
        data = LS(t)
        bad = [e for e in data
               if any(k in ((e.get("fields") or {}).get("status_note") or "")
                      for k in ("задерж", "просроч", "частичн", "заблокир"))]
        note(f"{t}: проблемных {len(bad)} — " + ", ".join(
            f"{(e.get('fields') or {}).get('wh_doc_no') or (e.get('fields') or {}).get('wh_waybill_no')}"
            f" [{(e.get('fields') or {}).get('status_note')}]"
            for e in bad))

    # ---- Phase 10 ----------------------------------------------------
    step(132, "Оплата МетизСнаб по WB-001: PAY-001 450000 исх.")
    payment("PAY-001", "2026-10-20", 450000, "исходящая",
            wb="WB-001", sup="SUP-001", ctr="CTR-P-001",
            status="оплачено в срок")

    step(133, "Оплата ЛКМ-Групп по WB-002: PAY-002 380000 исх.")
    payment("PAY-002", "2026-10-21", 380000, "исходящая",
            wb="WB-002", sup="SUP-002", ctr="CTR-P-002")

    step(134, "Оплата ЛесТорг по WB-003: PAY-003 720000 исх., +3 дня")
    payment("PAY-003", "2026-10-24", 720000, "исходящая",
            wb="WB-003", sup="SUP-003", ctr="CTR-P-003",
            status="проведена с опозданием на 3 дня")

    step(135, "PAY-004 — статус «проведена с удержанием претензии 9000»")
    UP("PAY-004", {"status_note": "проведена с удержанием претензии 9000 "
                                  "(HLD-002)"})

    step(136, "Входящая оплата Ритейл Сити по WB-102: PAY-102 90000")
    payment("PAY-102", "2026-10-21", 90000, "входящая",
            wb="WB-102", client="CLI-002",
            status="96000 за минусом возврата 100 шт молока")

    step(137, "Сверка PAY-103 (частичная оплата WB-103)")
    d = GET("PAY-103")
    note(f"PAY-103: {(d.get('fields') or {}).get('wh_amount')} RUB — "
         f"{(d.get('fields') or {}).get('status_note')}")

    step(138, "Аренда октябрь CTR-R-001: PAY-R-001 950000 исх.")
    payment("PAY-R-001", "2026-10-31", 950000, "исходящая",
            ctr="CTR-R-001",
            status="аренда WH-01, октябрь; получатель АО ПромНедвижимость")
    note("Ребра payment->legal_entity нет в схеме — арендодатель указан в "
         "статусе, привязка через договор.")

    step(139, "Аренда WH-02 CTR-R-002: PAY-R-002 380000 исх.")
    payment("PAY-R-002", "2026-10-31", 380000, "исходящая",
            ctr="CTR-R-002", status="аренда WH-02, октябрь")

    step(140, "Лизинговый платёж CTR-L-001: PAY-L-001 150000 исх.")
    payment("PAY-L-001", "2026-10-31", 150000, "исходящая",
            ctr="CTR-L-001",
            status="лизинговый платёж; получатель ООО ТехноЛизинг")

    step(141, "Страховая премия CTR-I-001: PAY-I-001 480000 исх.")
    payment("PAY-I-001", "2026-10-20", 480000, "исходящая",
            ctr="CTR-I-001",
            status="годовая страховая премия; АО Страховая Гарантия")

    step(142, "Платёж по кредиту CTR-C-001: PAY-C-001 550000 исх.")
    payment("PAY-C-001", "2026-10-31", 550000, "исходящая",
            ctr="CTR-C-001",
            status="тело 280000 + проценты 270000; АО КредитПромБанк")

    step(143, "PAY-T-001 — «проведена за вычетом штрафа 15000»")
    UP("PAY-T-001", {"status_note": "проведена за вычетом штрафа 15000 "
                                    "(HLD-003); фактически 45000"})
    UP("HLD-003", {"wh_reason": "штраф за опоздание INB-008 — ЗАКРЫТО: "
                                "удержано из оплаты PAY-T-001"})

    step(144, "Издержка EXP-006 ГСМ и обслуживание погрузчиков 33000")
    expense("EXP-006", "2026-10-31", 33000,
            "ГСМ и обслуживание погрузчиков", "WH-01")

    step(145, "Издержка EXP-007 охрана склада 95000")
    expense("EXP-007", "2026-10-31", 95000, "охрана склада", "WH-01")

    step(146, "Издержка EXP-008 коммуналка WH-02 26000")
    expense("EXP-008", "2026-10-31", 26000, "аренда WH-02 коммуналка",
            "WH-02")

    step(147, "Все оплаты октября")
    data = LS("wh_payment")
    inc = [(e.get("fields") or {}) for e in data
           if "вход" in ((e.get("fields") or {}).get("wh_direction") or "")]
    out = [(e.get("fields") or {}) for e in data
           if "исход" in ((e.get("fields") or {}).get("wh_direction") or "")]
    note(f"входящие: {len(inc)} на {sum(float(x['wh_amount']) for x in inc)}; "
         f"исходящие: {len(out)} на {sum(float(x['wh_amount']) for x in out)}")

    step(148, "Оплаты по договору CTR-P-001")
    rels = RELS("CTR-P-001", "incoming")
    pays = [name_of(x) for x in rels
            if x["predicate"] == "wh_payment_contract"]
    note("Платежи по CTR-P-001: " + ", ".join(pays))

    step(149, "Дебиторка: неоплаченные и частично оплаченные накладные")
    data = LS("wh_waybill")
    debt = []
    for e in data:
        f = e.get("fields") or {}
        st = f.get("status_note") or ""
        if any(k in st for k in ("просроч", "долг", "частичн")):
            debt.append(f"{f.get('wh_waybill_no')} [{st}]")
    note("Дебиторка: " + "; ".join(debt))

    step(150, "Все издержки по обоим складам")
    data = LS("wh_expense")
    note("Издержек: " + str(len(data)) + " — " + ", ".join(
        (e.get("fields") or {}).get("wh_expense_no", "?") for e in data))

    # ---- Phase 11 ----------------------------------------------------
    step(151, "Инвентаризация Z-STO-A: акт INV-002, расхождение 0")
    E("INV-002", "wh_inventory",
      {"wh_inv_no": "INV-002", "wh_doc_date": "2026-10-21",
       "wh_diff_qty": 0})
    RE("INV-002", "wh_counted_in", "WH-01")
    UP("INV-002", {"status_note": "плановая, зона Z-STO-A — чисто"})

    step(152, "Инвентаризация Z-COLD: акт INV-003, +12 шт творога")
    E("INV-003", "wh_inventory",
      {"wh_inv_no": "INV-003", "wh_doc_date": "2026-10-21",
       "wh_diff_qty": 12})
    RE("INV-003", "wh_counted_in", "WH-01")
    UP("INV-003", {"status_note": "зона Z-COLD — пересортица, излишек "
                                  "12 шт творога (B-109)"})

    step(153, "Оприходование излишка: B-109 -> 1012 шт")
    UP("B-109", {"wh_qty": 1012})

    step(154, "Инвентаризация Z-N-STO-A (WH-02): акт INV-004, расхождение 0")
    E("INV-004", "wh_inventory",
      {"wh_inv_no": "INV-004", "wh_doc_date": "2026-10-21",
       "wh_diff_qty": 0})
    RE("INV-004", "wh_counted_in", "WH-02")
    UP("INV-004", {"status_note": "зона Z-N-STO-A — чисто"})

    step(155, "Сверка с МетизСнаб по CTR-P-001")
    rels = RELS("CTR-P-001", "incoming")
    inbs = [name_of(x) for x in rels
            if x["predicate"] == "wh_inbound_contract"]
    pays = [name_of(x) for x in rels
            if x["predicate"] == "wh_payment_contract"]
    note(f"поставки={inbs}; оплаты={pays}")

    step(156, "Сверка со СтройМонтаж: отгрузки, накладные, оплаты, долги")
    rels = RELS("CLI-001", "incoming")
    note("Связи CLI-001: " + "; ".join(
        f"{x['predicate']}<-{name_of(x)}" for x in rels))

    step(157, "История изменений остатка B-101")
    h = HIST("B-101")
    CUR["ops"][-1]["result"] = {"history": h}

    step(158, "Все акты инвентаризации")
    data = LS("wh_inventory")
    note("Акты: " + ", ".join(
        (e.get("fields") or {}).get("wh_inv_no", "?") for e in data))

    # ---- Phase 12 ----------------------------------------------------
    step(159, "Все поставки от Молочный Дом")
    rels = RELS("LE-MOL", "incoming")
    inbs = [name_of(x) for x in rels
            if x["predicate"] == "wh_inbound_from"]
    note("Поставки Молочный Дом: " + ", ".join(inbs))

    step(160, "Все документы за 2026-10-19")
    for t in ("wh_inbound", "wh_outbound", "wh_waybill"):
        data = LS(t)
        day = [e for e in data
               if (e.get("fields") or {}).get("wh_doc_date") == "2026-10-19"]
        note(f"{t} 2026-10-19: " + ", ".join(
            (e.get("fields") or {}).get("wh_doc_no")
            or (e.get("fields") or {}).get("wh_waybill_no") or "?"
            for e in day))

    step(161, "Товары, проходившие через карантин Z-QA")
    rels = RELS("Z-QA", "incoming")
    qa = [x for x in rels if x["predicate"] == "wh_stock_zone"]
    prods_qa = []
    for x in qa:
        rs = T("relations.list",
               {"id": x["target_entity_id"], "direction": "outgoing"})
        prods_qa += [name_of(y) for y in (rs.get("data") or [])
                     if y["predicate"] == "wh_stock_product"]
    note(f"Партии в карантине: {[name_of(x) for x in qa]}; "
         f"товары: {prods_qa}")

    step(162, "Физлица, забиравшие грузы лично")
    data = LS("wh_outbound")
    rcv_people = []
    for e in data:
        rs = T("relations.list",
               {"id": e["public_id"], "direction": "outgoing"})
        rcv_people += [name_of(y) for y in (rs.get("data") or [])
                       if y["predicate"] == "wh_outbound_receiver"]
    note("Получатели-физлица: " + ", ".join(rcv_people))

    step(163, "Перевозчики и количество накладных")
    for con in ("CON-001", "CON-002"):
        rels = RELS(con, "incoming")
        wb = [name_of(x) for x in rels
              if x["predicate"] == "wh_waybill_carrier"]
        ob = [name_of(x) for x in rels
              if x["predicate"] == "wh_outbound_carrier"]
        note(f"{con}: накладные={wb} ({len(wb)}), отгрузки={ob}")

    step(164, "Договоры, истекающие до конца 2027")
    data = LS("wh_contract")
    exp = [e for e in data
           if (e.get("fields") or {}).get("wh_until")
           and (e.get("fields") or {}).get("wh_until") <= "2027-12-31"]
    note("Истекают до 2027-12-31: " + ", ".join(
        f"{(e.get('fields') or {}).get('wh_contract_no')} "
        f"(до {(e.get('fields') or {}).get('wh_until')})" for e in exp))

    step(165, "Реестр договоров с контрагентами")
    data = LS("wh_contract")
    for e in data:
        rs = T("relations.list",
               {"id": e["public_id"], "direction": "outgoing"})
        party = [name_of(x) for x in (rs.get("data") or [])
                 if x["predicate"] == "wh_contract_party"]
        whs = [name_of(x) for x in (rs.get("data") or [])
               if x["predicate"] == "wh_contract_wh"]
        f = e.get("fields") or {}
        note(f"{f.get('wh_contract_no')} [{f.get('wh_contract_kind')}] "
             f"{f.get('status_note')}: контрагент={party}, склады={whs}")

    step(166, "Зоны площадью больше 3000 м²")
    r = Q({"from": {"type_id": "wh_zone"},
           "where": {"comparison": {"field": "wh_capacity_m2",
                                    "op": "gt", "value": 3000}},
           "page": {"size": 100}})
    note(json.dumps(short_result(r), ensure_ascii=False)[:500])

    step(167, "dashboard.stats — общая статистика")
    r = T("dashboard.stats", {})
    note(json.dumps(r.get("data"), ensure_ascii=False)[:800])

    step(168, "entities.query: товары категории «ЛКМ»")
    r = Q({"from": {"type_id": "wh_product"},
           "where": {"comparison": {"field": "wh_category",
                                    "op": "eq", "value": "ЛКМ"}},
           "page": {"size": 100}})
    note(json.dumps(short_result(r), ensure_ascii=False)[:500])

    step(169, "Оплаты > 200000 через фильтр по полю")
    r = Q({"from": {"type_id": "wh_payment"},
           "where": {"comparison": {"field": "wh_amount",
                                    "op": "gt", "value": 200000}},
           "page": {"size": 200}})
    note(json.dumps(short_result(r), ensure_ascii=False)[:800])

    step(170, "Карточка склада WH-01 целиком")
    GET("WH-01")
    rels = RELS("WH-01", "incoming")
    by_pred = {}
    for x in rels:
        by_pred.setdefault(x["predicate"], []).append(name_of(x))
    for p, items in by_pred.items():
        note(f"{p}: {items}")

    # ---- Phase 13 ----------------------------------------------------
    step(171, "Новый клиент ООО ДальСтрой 2501112233 + CLI-005")
    E("LE-DALSTROY", "legal_entity",
      {"name": "ООО ДальСтрой", "inn": "2501112233"})
    E("CLI-005", "wh_client",
      {"wh_partner_code": "CLI-005", "name": "ООО ДальСтрой",
       "inn": "2501112233"})

    step(172, "Договор ответхранения CTR-S-003 ДальСтрой WH-02 60000/мес")
    contract("CTR-S-003", "хранение",
             "Ответственное хранение для ООО ДальСтрой, 60000 RUB/мес",
             "LE-DALSTROY", ["WH-02"], "2026-10-21", None, 60000,
             "действует — тариф 60000 RUB/мес")

    step(173, "Завоз ДальСтрой на WH-02: INB-011, SKU-301 Металлочерепица, "
              "B-DS-01 800 м²")
    inbound("INB-011", "LE-DALSTROY", None, "CTR-S-003", "WH-02",
            "2026-10-21", "принята на хранение")
    E("SKU-301", "wh_product", {"wh_sku": "SKU-301",
                                "name": "Металлочерепица",
                                "wh_unit": "м²",
                                "wh_category": "кровля — ответхранение"})
    RE("INB-011", "wh_inbound_item", "SKU-301")
    stock("B-DS-01", "SKU-301", 800, "Z-N-STO-A", ctr="CTR-S-003")

    step(174, "СтройМонтаж погасил долг: PAY-101 390000 вх.; HLD-004 "
              "закрыто")
    payment("PAY-101", "2026-10-21", 390000, "входящая",
            wb="WB-101", client="CLI-001",
            status="погашение дебиторки по WB-101")
    UP("HLD-004", {"wh_reason": "дебиторка по WB-101 — ЗАКРЫТО 2026-10-21: "
                                "оплата PAY-101 390000 получена"})
    UP("WB-101", {"status_note": "оплачена полностью 2026-10-21 (PAY-101)"})

    step(175, "OUT-006 разблокирована и отгружена: B-107 -> 10, "
              "B-110 -> 1800")
    UP("OUT-006", {"status_note": "отгружена"})
    UP("B-107", {"wh_qty": 10})
    UP("B-110", {"wh_qty": 1800})
    RE("OUT-006", "wh_outbound_carrier", "CON-001")
    RE("WB-106", "wh_waybill_carrier", "CON-001")
    UP("WB-106", {"status_note": "выставлена"})

    step(176, "INB-008 прибыла с опозданием: эмаль 400 л B-113 -> Z-STO-B")
    UP("INB-008", {"status_note": "принята с опозданием, штраф применён "
                                  "(HLD-003)"})
    stock("B-113", "SKU-104", 400, "Z-STO-B")

    step(177, "CTR-S-001 расторгнут досрочно по запросу ПромСклад")
    UP("CTR-S-001", {"status_note": "расторгнут досрочно 2026-10-22 по "
                                    "соглашению сторон"})

    step(178, "Финальная отгрузка OUT-008: 1 шт ЛУ-200 ПромСклад, "
              "B-PS-01 -> 0")
    outbound("OUT-008", "WH-01", "LE-PROMSKLAD", "CLI-003", "2026-10-22",
             "отгружена — финальная выдача с ответхранения")
    RE("OUT-008", "wh_outbound_item", "SKU-201")
    waybill("WB-108", "2026-10-22", 0, "OUT-008", "wh_waybill_outbound",
            status="акт выдачи")
    RE("OUT-008", "wh_outbound_receiver", "P-ORLOV")
    UP("B-PS-01", {"wh_qty": 0})

    step(179, "Финальный расчёт CTR-S-001: PAY-S-001-FIN 90000 вх.; "
              "HLD-005 12000")
    payment("PAY-S-001-FIN", "2026-10-22", 90000, "входящая",
            client="CLI-003", ctr="CTR-S-001",
            status="финальный расчёт за неполный месяц (досрочное "
                   "расторжение)")
    E("HLD-005", "wh_hold",
      {"wh_hold_no": "HLD-005", "wh_amount": 12000,
       "wh_reason": "повреждение упаковки при вывозе",
       "wh_until": "2026-11-15"})
    RE("HLD-005", "wh_hold_on", "PAY-S-001-FIN")
    RE("HLD-005", "wh_hold_party", "CLI-003")

    step(180, "Страховой случай: протечка в Z-STO-B; EXP-009 45000; "
              "HLD-006 на страховую выплату")
    expense("EXP-009", "2026-10-22", 45000,
            "ущерб до страховой выплаты (протечка крыши Z-STO-B, краска)",
            "WH-01")
    UP("CTR-I-001", {"status_note": "заявлен страховой случай 2026-10-22"})
    E("HLD-006", "wh_hold",
      {"wh_hold_no": "HLD-006", "wh_amount": 45000,
       "wh_reason": "страховая выплата по случаю 2026-10-22 (ущерб EXP-009, "
                    "сторона — страховщик АО Страховая Гарантия)"})
    note("wh_hold_party не применимо (страховщик — не wh_client); "
         "удержание зафиксировано на будущую выплату.")

    step(181, "Страховая выплата PAY-I-002 40000 вх. по CTR-I-001; "
              "HLD-006 привязано")
    payment("PAY-I-002", "2026-10-22", 40000, "входящая",
            ctr="CTR-I-001",
            status="страховая выплата от АО Страховая Гарантия")
    RE("HLD-006", "wh_hold_on", "PAY-I-002")
    UP("HLD-006", {"wh_reason": "страховая выплата по случаю 2026-10-22 — "
                                "получено 40000 из 45000 (PAY-I-002), "
                                "остаток 5000 — открыто"})

    step(182, "Ставка по CTR-C-001 повышена до 18% с 2026-11")
    UP("CTR-C-001", {"name": "Кредитная линия 20000000 RUB на закупку "
                             "товара, 18% годовых с 2026-11",
                     "status_note": "действует — ставка повышена до 18% "
                                    "годовых с 2026-11"})

    step(183, "Договор CTR-L-002 выкупной платёж 900000, на согласовании")
    contract("CTR-L-002", "лизинг",
             "Выкупной платёж за погрузчики (предложение ТехноЛизинг)",
             "LE-TEHNLIZ", ["WH-01"], "2026-10-22", None, 900000,
             "на согласовании")

    step(184, "Ревизия персонала")
    data = LS("person")
    for e in data:
        rs = T("relations.list",
               {"id": e["public_id"], "direction": "outgoing"})
        whs = [name_of(x) for x in (rs.get("data") or [])
               if x["predicate"] == "wh_keeper"]
        note(f"{(e.get('fields') or {}).get('full_name')}: склады={whs}")

    step(185, "Все документы склада WH-02")
    rels = RELS("WH-02", "incoming")
    by_pred = {}
    for x in rels:
        by_pred.setdefault(x["predicate"], []).append(name_of(x))
    for p, items in by_pred.items():
        note(f"{p}: {items}")

    step(186, "Всё по перевозчику ТрансЛайн")
    rels = RELS("CON-001", "incoming")
    note("Связи CON-001: " + "; ".join(
        f"{x['predicate']}<-{name_of(x)}" for x in rels))
    rels2 = RELS("CTR-T-001", "incoming")
    note("Связи CTR-T-001: " + "; ".join(
        f"{x['predicate']}<-{name_of(x)}" for x in rels2))

    step(187, "Все удержания — итог претензионной работы")
    data = LS("wh_hold")
    for e in data:
        f = e.get("fields") or {}
        closed = "ЗАКРЫТО" in (f.get("wh_reason") or "")
        note(f"{f.get('wh_hold_no')}: {f.get('wh_amount')} — "
             f"{'закрыто' if closed else 'ОТКРЫТО'}")

    step(188, "Итоговая инвентарная ведомость по двум складам")
    data = LS("wh_stock")
    for e in data:
        f = e.get("fields") or {}
        rs = T("relations.list",
               {"id": e["public_id"], "direction": "outgoing"})
        prod = [name_of(x) for x in (rs.get("data") or [])
                if x["predicate"] == "wh_stock_product"]
        zone = [name_of(x) for x in (rs.get("data") or [])
                if x["predicate"] == "wh_stock_zone"]
        note(f"{f.get('wh_batch')}: {prod} = {f.get('wh_qty')} в {zone}")

    step(189, "История накладной WB-101")
    h = HIST("WB-101")
    CUR["ops"][-1]["result"] = {"history": h}

    step(190, "Повторное создание WH-01 — проверка дедупликации")
    r = T("entities.create", {"type_id": "wh_warehouse",
                              "fields": {"wh_code": "WH-01",
                                         "name": "СК-Центральный"}})
    note(f"outcome={(r.get('data') or {}).get('outcome')}, "
         f"public_id={wc.pub_id(r)} (== WH-01: "
         f"{wc.pub_id(r) == IDS.get('WH-01')})")

    # ---- Phase 14 ----------------------------------------------------
    step(191, "audit.query по событиям entity")
    r = T("audit.query", {"resource_kind": "entity", "limit": 500})
    d = r.get("data")
    cnt = len(d) if isinstance(d, list) else d
    note(f"audit events returned: {cnt}")

    step(192, "Сводная статистика по всем типам")
    ALL_TYPES = ["person", "legal_entity", "wh_warehouse", "wh_zone",
                 "wh_product", "wh_stock", "wh_inbound", "wh_outbound",
                 "wh_inventory", "wh_supplier", "wh_client",
                 "wh_contractor", "wh_cargo", "wh_waybill", "wh_payment",
                 "wh_expense", "wh_hold", "wh_contract"]
    for t in ALL_TYPES:
        data = LS(t)
        note(f"{t}: {len(data)}")

    step(193, "Топ-должники склада")
    data = LS("wh_waybill")
    for e in data:
        f = e.get("fields") or {}
        st = f.get("status_note") or ""
        if any(k in st for k in ("долг", "просроч", "частичн")):
            note(f"{f.get('wh_waybill_no')}: {f.get('wh_amount')} — {st}")

    step(194, "Контрагенты с действующими договорами")
    data = LS("wh_contract")
    for e in data:
        f = e.get("fields") or {}
        if "действует" in (f.get("status_note") or ""):
            rs = T("relations.list",
                   {"id": e["public_id"], "direction": "outgoing"})
            party = [name_of(x) for x in (rs.get("data") or [])
                     if x["predicate"] == "wh_contract_party"]
            note(f"{f.get('wh_contract_no')}: {party}")

    step(195, "Движение товара «молоко» (SKU-109)")
    rels = RELS("SKU-109", "incoming")
    for x in rels:
        note(f"{x['predicate']} <- {name_of(x)}")

    step(196, "Полный цикл по договору CTR-P-001")
    rels = RELS("CTR-P-001", "incoming")
    for x in rels:
        note(f"{x['predicate']} <- {name_of(x)}")

    step(197, "Полный цикл по договору CTR-S-001")
    rels = RELS("CTR-S-001", "incoming")
    for x in rels:
        note(f"{x['predicate']} <- {name_of(x)}")

    step(198, "Финансовый срез: входящие vs исходящие vs издержки")
    data = LS("wh_payment")
    inc = sum(float((e.get("fields") or {}).get("wh_amount", 0)) for e in data
              if "вход" in ((e.get("fields") or {}).get("wh_direction") or ""))
    out = sum(float((e.get("fields") or {}).get("wh_amount", 0)) for e in data
              if "исход" in ((e.get("fields") or {}).get("wh_direction") or ""))
    data = LS("wh_expense")
    exp = sum(float((e.get("fields") or {}).get("wh_amount", 0))
              for e in data)
    note(f"входящие={inc}; исходящие={out}; издержки={exp}; "
         f"сальдо={inc - out - exp}")

    step(199, "Финальный статус WH-01: месяц закрыт")
    data = LS("wh_hold")
    open_holds = [e for e in data
                  if "ЗАКРЫТО" not in
                  ((e.get("fields") or {}).get("wh_reason") or "")]
    n_open = len(open_holds)
    UP("WH-01", {"status_note": f"операционный месяц октябрь закрыт, "
                                f"претензий открыто: {n_open}"})
    note(f"Открытых удержаний: {n_open} — " + ", ".join(
        (e.get("fields") or {}).get("wh_hold_no", "?")
        for e in open_holds))

    step(200, "Итоговый отчёт управляющего — entities.list по типам")
    for t in ["wh_warehouse", "wh_zone", "person", "legal_entity",
              "wh_supplier", "wh_client", "wh_contractor", "wh_contract",
              "wh_product", "wh_stock", "wh_cargo", "wh_inbound",
              "wh_outbound", "wh_waybill", "wh_payment", "wh_expense",
              "wh_hold", "wh_inventory"]:
        data = LS(t)
        note(f"{t}: {len(data)}")

    # ---- persist ------------------------------------------------------
    with open(os.path.join(HERE, ".wh_objects_200.json"), "w",
              encoding="utf-8") as f:
        json.dump(IDS, f, ensure_ascii=False, indent=1)
    with open(os.path.join(HERE, ".wh_run_200_log.json"), "w",
              encoding="utf-8") as f:
        json.dump(LOG, f, ensure_ascii=False, indent=1)
    fails = [s["n"] for s in LOG if not s["ok"]]
    print(f"\nDONE. {len(LOG)} prompts, {len(IDS)} ids, "
          f"prompts with failed ops: {fails}")


if __name__ == "__main__":
    t0 = time.time()
    run()
    print(f"elapsed {time.time() - t0:.1f}s")
