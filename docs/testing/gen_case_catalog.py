#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Generate docs/testing/mcp_test_cases.md from build_cases() in
mcp_test_runner.py and docs/testing/mcp_test_report.md from the last
run's mcp_report_raw.json."""
import collections
import importlib.util
import json
import sys

HERE = "docs/testing"


def load_cases():
    spec = importlib.util.spec_from_file_location(
        "mcp_test_runner", f"{HERE}/mcp_test_runner.py")
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m.build_cases()


def gen_cases_md(cases, raw):
    if raw:
        # Prefer the raw run report — it lists exactly what was executed,
        # including the generated X.* per-tool block.
        areas = collections.OrderedDict()
        for c in raw["results"]:
            tok = "analyst" if "analyst" in c["name"].lower() else "admin"
            areas.setdefault(c["area"], []).append(
                {"name": c["name"], "tool": c["tool"],
                 "expect": c["expect"], "token": tok})
        cases = [c for cs in areas.values() for c in cs]
    else:
        areas = collections.OrderedDict()
        for c in cases:
            areas.setdefault(c["area"], []).append(c)
    L = []
    L.append("# EngineR MCP — каталог тест-кейсов")
    L.append("")
    L.append("Автогенерация из `build_cases()` в `mcp_test_runner.py` "
             "(`python docs/testing/gen_case_catalog.py`).")
    L.append(f"Всего кейсов: **{len(cases)}** в {len(areas)} областях. "
             "Все выполняются через MCP endpoint `POST /mcp` "
             "(JSON-RPC `tools/call`, `tools/list`, `initialize`).")
    L.append("")
    L.append("## Способы выполнения")
    L.append("")
    L.append("```bash")
    L.append("# Весь каталог:")
    L.append("python docs/testing/mcp_test_runner.py")
    L.append("")
    L.append("# Одна область (префикс кода области):")
    L.append("python docs/testing/mcp_test_runner.py --area J.requests")
    L.append("")
    L.append("# Другой стенд / учётная запись:")
    L.append("python docs/testing/mcp_test_runner.py \\")
    L.append("    --mcp http://host:8020 --api http://host:8010 \\")
    L.append("    --login admin --password enginer-demo")
    L.append("")
    L.append("# Прямой JSON-RPC вызов:")
    L.append("curl -X POST http://localhost:8020/mcp \\")
    L.append('  -H "Authorization: Bearer egr_<token>" \\')
    L.append('  -H "Content-Type: application/json" \\')
    L.append('  -H "Accept: application/json, text/event-stream" \\')
    L.append('''  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call",''')
    L.append('''       "params":{"name":"entities.list","arguments":{"limit":5}}}\'''')
    L.append("```")
    L.append("")
    L.append("Раннер перед прогоном сам заводит две сервисные учётки "
             "(`qa-mcp-admin`: admins+all_data, `qa-mcp-analyst`: "
             "analysts+all_data), выпускает каждой bearer-токен и гоняет "
             "все кейсы через `tools/call`. Столбец «Токен» — какой "
             "принципал выполняет вызов.")
    L.append("")
    L.append("## Семантика ожиданий (expect)")
    L.append("")
    L.append("| expect | Смысл |")
    L.append("|---|---|")
    L.append("| `ok` | вызов успешен; `match` — подстрока, обязательная в ответе |")
    L.append("| `tool_err` | MCP-валидация отклонила вызов до обращения к API (isError) |")
    L.append("| `fail` | API вернул бизнес-ошибку 4xx |")
    L.append("| `deny` | нет capability у принципала (-32602 / not granted) |")
    L.append("| `http401` | транспорт отклонил bearer-токен |")
    L.append("")
    L.append("`fn:` — аргументы вычисляются из env (id, созданные ранними "
             "кейсами); `save:` — ответ сохраняется в env для цепочек.")
    for area, cs in areas.items():
        L += ["", f"## {area}", "", "| Кейс | Tool | Ожидание | Токен |",
              "|---|---|---|---|"]
        for c in cs:
            L.append(f"| {c['name']} | `{c['tool']}` | {c['expect']} "
                     f"| {c['token']} |")
    return "\n".join(L) + "\n"


def gen_report_md(cases):
    raw = json.load(open(f"{HERE}/mcp_report_raw.json", encoding="utf-8"))
    res = raw["results"]
    by_area = collections.OrderedDict()
    for r in res:
        by_area.setdefault(r["area"], []).append(r)
    L = []
    L.append("# EngineR MCP — отчёт о прогоне")
    L.append("")
    L.append(f"Результат: **{raw['passed']}/{raw['total']} passed**.")
    L.append("")
    L.append("## Найденные и исправленные дефекты (эта кампания)")
    L.append("")
    L.append("### Баги API")
    L.append("")
    L.append("1. **`PUT /schemas/drafts/{id}` затирал содержимое черновика** — "
             "`types`/`predicates`/`changes` имели `#[serde(default)]`: "
             "metadata-only апдейт (display_name + semantic_version) "
             "обнулял весь каталог типов. Опубликованная после этого "
             "ревизия отдавала пустой `/schemas/types`. Исправлено на "
             "`Option<Vec>`: отсутствующее поле сохраняет контент, "
             "явный `[]` — очищает. Регрессия покрыта тестом в "
             "`pg_scenarios` (metadata-only PUT не трогает types).")
    L.append("2. **`POST /workflows` принимал любой `definition_id`** — "
             "инстанс создавался для несуществующего определения. Добавлена "
             "таблица `meta.workflow_definition` (миграция 23), сид 4 "
             "определений из SKR-бандла, валидация на старте (404 на "
             "неизвестный id) и проверка легальности перехода "
             "`{from,to,trigger}` по определению в `transition_workflow` "
             "(422 на нелегальный переход).")
    L.append("3. **`POST /entities/query` молча игнорил невалидный `query`** — "
             "строка/число вместо объекта превращались в match-all выборку. "
             "Теперь 422 на не-объектном теле и не-объектном `query`.")
    L.append("4. **Рассинхрон capability на read-эндпоинтах схемы** — "
             "`GET /schemas/published` и `GET /schemas/revisions/{id}` "
             "попадали под catch-all `schema.draft`, хотя это read-only "
             "каталог того же уровня, что `/schemas/types` (`entity.read`). "
             "Analyst видел tool в `tools/list`, но получал 403. Маппинг "
             "исправлен на `entity.read`.")
    L.append("5. **Неоднородные ответы на ghost-parent в sub-list "
             "эндпоинтах** — `GET /entities/{id}/documents`, "
             "`/entities/{id}/checks`, `GET /users/{id}/tokens`, "
             "`/users/{id}/activity` возвращали `[]` на несуществующий "
             "родитель, тогда как `relations.list`/`tasks.list`/"
             "`factors.list`/`entities.history` отдавали 404. Добавлены "
             "проверки существования — поведение унифицировано к 404.")
    L.append("6. **`POST /connectors` молча клампил `trust_rank`** (101 → "
             "100) и принимал отрицательный `rate_limit_rps`. Теперь "
             "вне-диапазонные значения — 422, и в `PATCH /connectors/{id}` "
             "тоже.")
    L.append("")
    L.append("### Баги MCP-каталога/адаптера")
    L.append("")
    L.append("7. `requests.create`: `requested_product` не был required — "
             "API отвечал 422. Помечен обязательным.")
    L.append("8. `workflows.transition`: `to` и `trigger` помечены "
             "обязательными (API 422 без них).")
    L.append("9. `assessment.create`: не хватало `force`/`desired_level`/"
             "`decision` — добавлены.")
    L.append("10. `inputSchema` теперь несёт `x-location` "
             "(path/query/body) на параметр — позволяет клиентам и "
             "тест-генератору отличать path-аргументы от query/body.")
    L.append("")
    L.append("## Сводка по областям")
    L.append("")
    L.append("| Область | Кейсов | PASS | FAIL/ERR | Медиана, мс |")
    L.append("|---|---|---|---|---|")
    for area, rs in by_area.items():
        ok = sum(1 for r in rs if r["outcome"] == "pass")
        bad = len(rs) - ok
        ms = sorted(r["ms"] for r in rs)
        med = ms[len(ms) // 2]
        L.append(f"| {area} | {len(rs)} | {ok} | {bad or '—'} | {med} |")
    L.append("")
    L.append("## Детали по каждому кейсу")
    for area, rs in by_area.items():
        L += ["", f"### {area}", "",
              "| Кейс | Tool | Ожидание | Итог | мс |", "|---|---|---|---|---|"]
        for r in rs:
            mark = {"pass": "PASS", "fail": "**FAIL**",
                    "error": "**ERR**"}[r["outcome"]]
            L.append(f"| {r['name']} | `{r['tool']}` | {r['expect']} "
                     f"| {mark} | {r['ms']} |")
        for r in rs:
            if r["outcome"] != "pass":
                L.append("")
                det = r["detail"].replace("\n", " ").replace("|", "\\|")[:400]
                L.append(f"> **{r['name']}** — {det}")
    return "\n".join(L) + "\n"


if __name__ == "__main__":
    cases = load_cases()
    try:
        raw = json.load(open(f"{HERE}/mcp_report_raw.json", encoding="utf-8"))
    except FileNotFoundError:
        raw = None
        print("no mcp_report_raw.json — catalog from build_cases only")
    open(f"{HERE}/mcp_test_cases.md", "w", encoding="utf-8") \
        .write(gen_cases_md(cases, raw))
    if raw:
        open(f"{HERE}/mcp_test_report.md", "w", encoding="utf-8") \
            .write(gen_report_md(cases))
        print("report written")
    n = sum(len(v) for v in collections.Counter()) or "?"
    print("catalog written")
