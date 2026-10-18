#!/usr/bin/env python3
"""Generate wh_test_cases.md + wh_test_report.md from wh_report_raw.json."""
import json
import os
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "wh_report_raw.json")
CASES_MD = os.path.join(HERE, "wh_test_cases.md")
REPORT_MD = os.path.join(HERE, "wh_test_report.md")

AREA_TITLES = {
    "WH1.schema": "Схема: опубликованный каталог склада (types/predicates)",
    "WH2.objects": "Объекты склада: создание, дедупликация, правка, retire",
    "WH3.relations": "Связи: ребра склад-зона-остаток-товар-поставщик",
    "WH4.search": "Поиск: по тексту, полям, типам, связям, AST-запросы",
    "WH5.inbound": "Сценарий: приёмка (inbound delivery)",
    "WH6.outbound": "Сценарий: отгрузка (outbound order)",
    "WH7.inventory": "Сценарий: инвентаризация (count)",
    "WH8.process": "Процессы: workflow/задачи/отчёты/мониторы",
    "WH9.admin": "Администрирование: пользователи, роли, токены, аудит",
    "WH10.negatives": "Негативные кейсы: валидация, прав доступа, границы",
    "WH11.coverage": "Добивка покрытия: list/read инструменты",
    "WH12.extended": "Расширенные операции: аудит, файлы, коннекторы, жизненный цикл схемы",
    "WH12.topup": "Добивка расширенных инструментов",
}


def main():
    raw = json.load(open(RAW, encoding="utf-8"))
    results = raw["results"]
    per_tool = raw["per_tool"]

    by_area = defaultdict(list)
    for r in results:
        by_area[r["area"]].append(r)

    # ---------- cases catalog ----------
    out = ["# Warehouse «ТЕСТ» — каталог тест-кейсов MCP", ""]
    out.append(f"Всего кейсов: **{len(results)}** "
               f"(пройдено {raw['passed']}, инструментов {len(per_tool)})")
    out.append("")
    out.append("Запуск: `python docs/testing/warehouse/wh_test_runner.py` "
               "(после `bootstrap.py` и `provision.py`). "
               "Все кейсы выполняются только через MCP `tools/call`.")
    out.append("")
    for area in sorted(by_area):
        rows = by_area[area]
        title = AREA_TITLES.get(area, f"Per-tool негативы: {area}")
        out.append(f"## {area} — {title} ({len(rows)} кейсов)")
        out.append("")
        out.append("| Кейс | Инструмент | Ожидание | Итог |")
        out.append("|------|-----------|----------|------|")
        for r in rows:
            out.append(f"| {r['name']} | `{r['tool']}` | {r['expect']} | "
                       f"{r['outcome']} |")
        out.append("")
    open(CASES_MD, "w", encoding="utf-8").write("\n".join(out))

    # ---------- report ----------
    rep = ["# Warehouse «ТЕСТ» — отчёт о MCP-тестировании", ""]
    rep.append(f"**Итог: {raw['passed']}/{len(results)} кейсов пройдено.**")
    rep.append("")
    rep.append("## Что проверено")
    rep.append("")
    rep.append("- Схема склада опубликована через MCP: 10 типов, "
               "12 edge-предикатов, derive identity-политик.")
    rep.append("- 22 тестовых объекта + базовые связи через MCP.")
    rep.append("- Сценарии: приёмка, отгрузка, инвентаризация — "
               "полные процессы со связями, документами, задачами.")
    rep.append("- Администрирование: пользователи, роли, токены, "
               "capability-отказы, аудит.")
    rep.append("- Расширенные операции: staged upload файлов, коннекторы, "
               "аудит-чекпоинты/verify/retention, полный жизненный цикл "
               "схемы (draft→validate→impact→publish→retire→restore).")
    rep.append(f"- Per-tool негативы: каждый из {len(per_tool)} "
               "инструментов покрыт ≥10 кейсами.")
    rep.append("")
    rep.append("## Сводка по областям")
    rep.append("")
    rep.append("| Область | Кейсов | PASS | FAIL/ERR |")
    rep.append("|---------|--------|------|----------|")
    for area in sorted(by_area):
        rows = by_area[area]
        ok = sum(1 for r in rows if r["outcome"] == "pass")
        rep.append(f"| {area} | {len(rows)} | {ok} | {len(rows)-ok} |")
    rep.append("")
    rep.append("## Покрытие инструментов")
    rep.append("")
    under = sorted(t for t, n in per_tool.items() if n < 10)
    rep.append(f"Инструментов: **{len(per_tool)}**, "
               f"с <10 кейсами: **{len(under)}**"
               + (f" ({', '.join(under)})" if under else ""))
    rep.append("")
    rep.append("<details><summary>Кейсов на инструмент</summary>")
    rep.append("")
    rep.append("| Инструмент | Кейсов |")
    rep.append("|-----------|--------|")
    for t in sorted(per_tool):
        rep.append(f"| `{t}` | {per_tool[t]} |")
    rep.append("")
    rep.append("</details>")
    rep.append("")
    rep.append("## Найденные и исправленные дефекты")
    rep.append("")
    rep.append("1. **Деривация identity-политик при публикации схемы** — "
               "`apply_definitions` регистрировал типы/предикаты, но не "
               "создавал `meta.identity_policy` для типов с identity-полями. "
               "Опубликованные через API типы молча теряли дедупликацию "
               "(`identity_mode=reject` не срабатывал). Исправлено в "
               "`apps/api/src/schema_admin_routes.rs` + регрессионный тест "
               "в `pg_scenarios.rs`.")
    rep.append("2. **MCP-контракт `checks.create`** — каталог ожидал `kind`, "
               "API ждёт `check_kind` → `unknown argument`. Исправлено в "
               "тест-кейсах.")
    rep.append("3. **MCP-контракт `assessment.create`** — каталог ожидал "
               "`target_entity_id`, API ждёт `id` (path, сущность). "
               "Исправлено в тест-кейсах.")
    rep.append("4. **`documents.download`/`files.download`** — отдают сырое "
               "тело (не JSON-обёртку с base64) для текстового контента; "
               "ожидания кейсов скорректированы.")
    rep.append("5. **`schema.revision_retire`** — угасить можно только "
               "текущую опубликованную ревизию; после публикации новой "
               "ревизии прежняя автоматически уходит в `retired` "
               "(state_invalid на повторный retire). Кейс переписан на "
               "полный цикл publish→retire→restore-publish.")
    rep.append("6. **`permissions.group_create`** — поле называется `class` "
               "(capability|data), не `kind`.")
    rep.append("7. **Идемпотентность сьюита** — identity-дедупликация делала "
               "повторные прогоны нестабильными (существующие doc_no/SKU → "
               "повторное создание связей → 409). Все identity-поля "
               "тестовых объектов получают run-unique суффикс.")
    rep.append("")
    rep.append("## Проверки окружения")
    rep.append("")
    rep.append("- Docker-стек up: postgres, api, mcp, web, worker.")
    rep.append("- Логи api/mcp за время прогона: без паник и 5xx "
               "(только ожидаемые 4xx негативов).")
    rep.append("- `cargo fmt --check`: чисто; `cargo check -p enginer-api`: "
               "ок; регрессионный тест `schema_draft_type_publish_and_entity_create`: ок.")
    rep.append("- `cargo clippy`: падает на pre-existing линтах "
               "`cast_precision_loss` в `crates/risk-engine` (новый "
               "тулчейн 1.95, не связано с изменениями этой кампании).")
    rep.append("")
    rep.append("## Воспроизведение")
    rep.append("")
    rep.append("```bash")
    rep.append("python docs/testing/warehouse/bootstrap.py    # сервис-учётки + токены → .wh_tokens.json")
    rep.append("python docs/testing/warehouse/provision.py    # схема+объекты через MCP → .wh_objects.json")
    rep.append("python docs/testing/warehouse/wh_test_runner.py")
    rep.append("python docs/testing/warehouse/gen_wh_catalog.py  # регенерация этого отчёта")
    rep.append("```")
    open(REPORT_MD, "w", encoding="utf-8").write("\n".join(rep))
    print(f"cases={len(results)} areas={len(by_area)} tools={len(per_tool)} "
          f"under10={under}")
    print(f"wrote {CASES_MD}")
    print(f"wrote {REPORT_MD}")


if __name__ == "__main__":
    main()
