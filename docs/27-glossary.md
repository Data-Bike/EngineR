# Глоссарий

| Термин | Определение |
|---|---|
| EngineR | Универсальное платформенное ядро |
| СКР САНАР | Прикладной пакет контроля/автоматизации нахождения аннуитетных рисков |
| Аннуитетный риск | Постоянно возможный риск непрерывной финансово-хозяйственной деятельности |
| Entity / сущность | Стабильный якорь идентичности узла графа |
| Statement / утверждение | Атом subject–predicate–entity/scalar с временем/provenance |
| Fact | В документах синоним statement; в коде предпочтительно Statement |
| Predicate | Определение смысла свойства/связи |
| TypeDefinition | Версионированные ограничения/поведение класса entities |
| Meta-kernel | Малый закрытый набор примитивов самой платформы |
| Schema package | Подписанный декларативный набор типов, views, policies refs, workflows/reports |
| Schema revision | Immutable версия опубликованной схемы |
| Residual fact | Исходный факт, невыразимый target view, сохранённый с lineage |
| Lineage | Трасса происхождения/преобразования |
| Provenance | Источник и доказательство утверждения |
| Effective view | Вычисленное представление нескольких источников по trust policy |
| IdentityPolicy | Правило exact/fuzzy идентификации сущности |
| Identity key | Канонический уникальный ключ, указывающий owner entity |
| Match candidate | Предположение о совпадении, не merge |
| Fingerprint | Детерминированный отпечаток canonical command/fact/query |
| Idempotency ledger | Хранилище результатов повторяемых команд |
| Merge/Split | Управляемое объединение/разделение identity с lineage |
| Projection | Восстанавливаемая оптимизированная read model |
| Watermark | До какой canonical revision построена projection |
| Query AST | Типизированное JSON-дерево запроса без SQL |
| PEP | Policy Enforcement Point, место обязательного применения решения |
| PDP | Policy Decision Point, вычисляет решение/obligations |
| Obligation | Ограничение/маска/фильтр, обязательный для PEP |
| Capability | Право на системную функцию |
| Data permission | Право на type/entity/field/relation/row |
| Capability group | Группа только функциональных прав |
| Data group | Группа только прав данных |
| RLS | PostgreSQL Row-Level Security, дополнительный barrier |
| Purpose of use | Заявленная/разрешённая цель обработки данных |
| Break-glass | Временный аварийный доступ со step-up/audit |
| Outbox | События, атомарно записанные с mutation |
| Inbox | Ledger обработанных consumer events |
| Job | Возобновляемая фоновая операция |
| Connector | Адаптер внутреннего/внешнего источника |
| Observation | Полученная от источника версия сведений |
| Check | Оркестрация источников и критериев по target |
| Risk criterion | Версионированное typed правило выявления фактора |
| Risk factor | Обнаруженное условие с evidence |
| Assessment | Расчётная оценка target на versioned inputs |
| Decision | Отдельное бизнес-решение человека/workflow |
| Monitor | Плановая повторная проверка/детектор изменений |
| ViewDefinition | Метаданные списка/карточки/формы/отчёта |
| Public ID | UUIDv7, доступный API |
| Internal ID | Compact bigint внутри DB/shard |
| Keyset cursor | Opaque signed continuation по sort keys |
| ЛТК | Лимит товарного кредита (прикладной термин пакета) |
| ОДЗ/ПДЗ | Прикладные показатели дебиторской задолженности; точная семантика задаётся schema package заказчика |
| ЮЛ/ФЛ | Юридическое/физическое лицо — прикладные types, не core structs |
| Холдинг | Temporal graph view владения/контроля/аффилированности |
