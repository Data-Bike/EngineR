# Коннекторы, проверки и движок риска

## 1. Connector SDK

Connector — адаптер источника, реализующий:

- descriptor/capabilities;
- credential validation;
- health/quota;
- search/fetch/monitor operations;
- request/response schema versions;
- normalization mapping;
- rate limit/retry;
- legal/licensing metadata;
- redacted diagnostics.

Ядро не знает поля конкретного API. Connector возвращает SourceObservation и mapping result.

## 2. Реестр требуемых источников

Пакет СКР предусматривает adapters для:

- внутренней базы EngineR;
- АСУ;
- «Евразия»;
- Контур.Фокус;
- ФНС;
- ФССП;
- ВАС/актуального правопреемника источника судебных данных;
- ФМС/актуального правопреемника;
- реестра недобросовестных поставщиков;
- Вестника государственной регистрации;
- Росздравнадзора;
- открытого Интернета;
- социальных сетей.

Название в legacy ТЗ не гарантирует доступность API. Перед реализацией каждого adapter фиксируются владелец, актуальный endpoint, договор, лицензия, jurisdiction, retention и разрешённые purposes.

## 3. Connector instance

Настройка tenant-specific:

- connector type/version;
- enabled operations;
- environment/base URL/proxy;
- secret references;
- organization account;
- rate/concurrency quotas;
- timeout/retry;
- cache/freshness;
- legal basis/purpose;
- permitted target types/fields;
- retention/raw payload policy;
- trust rank;
- mapping package version.

Secrets доступны только runner и никогда не возвращаются Admin API.

## 4. Выполнение

1. PEP проверяет check.run и доступ к target/input fields.
2. Job получает минимальный input projection.
3. Connector нормализует request и вычисляет remote idempotency key.
4. Rate limiter/quota reserve.
5. Вызов с timeout/circuit breaker.
6. Raw response stream сохраняется encrypted blob/hash по policy.
7. Parser создаёт typed observations.
8. Mapper связывает predicates с provenance.
9. Identity resolution выполняет exact link/candidate.
10. Risk engine получает событие.
11. Audit фиксирует источник, purpose, объём, не секретный payload.

## 5. Импорт выгрузок Контур.Фокус/ФНС

Upload определяется как connector operation file_import. Каждая строка/source record имеет external ID/fingerprint. Mapping profile показывает preview, conflicts и target predicates. Нельзя просто перезаписать пользовательские values: создаётся новый source-specific assertion и effective resolution.

## 6. Trust/resolution

Predicate может иметь policy:

- manual_verified first;
- authoritative source rank;
- freshest within source class;
- consensus;
- keep_all + conflict;
- derived formula.

Effective value — projection с объяснением. Исходные assertions неизменны. Любое auto-change high-impact поля может требовать approval.

## 7. CheckDefinition

- target types;
- required inputs;
- connector operations;
- parallel/sequential dependencies;
- freshness/cache;
- criteria package;
- expected artifacts;
- timeout/SLA;
- retry;
- assignment;
- data/legal purpose;
- report profile.

CheckRun pins definition/schema/policy/connector versions.

## 8. Risk criterion DSL

Никакого arbitrary code. Typed expression AST поддерживает:

- compare/range/existence/count;
- temporal age/duration;
- string exact/normalized set;
- graph relation exists/path with bounded depth;
- source freshness/trust/conflict;
- arithmetic decimal;
- boolean all/any/not;
- lookup in versioned dictionary;
- previous assessment change.

Каждый criterion имеет ID/version, input predicates, severity, weight, explanation key, evidence selector, effective interval, owner, tests.

## 9. Factor

RiskFactor — entity/statement set:

- criterion/version;
- target;
- detected/resolved time;
- severity/score contribution;
- status;
- evidence statement/source refs;
- explanation parameters;
- suppress/override with actor/reason/expiry.

Повторная evaluation upsert по target+criterion+evidence window, не создаёт миллион одинаковых factors.

## 10. Assessment

Assessment хранит:

- target/as_of;
- model/version;
- input watermark/fingerprint;
- factor refs;
- component scores;
- final decimal score/category;
- uncertainty/data freshness;
- generated/approved actors;
- status draft/approved/superseded;
- explanation.

Score не является единственным решением. Decision отделено и может учитывать policy/лимиты/человека.

## 11. Лимиты и отсрочки

Запрошенный/установленный ЛТК, запрошенная отсрочка, платёжная дисциплина, ОДЗ и ПДЗ описываются schema package как financial predicates/entities с currency, as_of и source. Risk model использует их по IDs; core не знает аббревиатур.

## 12. Автоматические реакции

Criterion/assessment threshold может создать task, alert, report, повторную проверку или outbound notification. High-impact изменение/блокировка требует workflow approval. Rule обязана иметь cooldown/dedup key, иначе одна проблема породит storm.

## 13. Интернет/соцсети

Это отдельный high-risk connector class:

- allowlisted providers/queries;
- соблюдение ToS/legal basis;
- minimization;
- provenance URL/fetch time;
- content hash/snippet limits;
- human verification;
- защита от prompt/content injection, если используется ML;
- нельзя автоматически считать упоминание доказанным фактом.

## 14. Тестирование adapters

- contract fixtures без live secrets;
- sandbox integration;
- schema drift;
- rate limit/timeout/retry;
- malformed/oversized payload;
- Unicode/locale;
- duplicate observations;
- legal/retention flags;
- redaction logs;
- reconciliation после ambiguous network result.
