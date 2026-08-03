# Административная панель

## 1. Принцип

Admin UI — управляемый редактор versioned resources, а не прямой доступ к таблицам. Каждое изменение: draft, validation, impact, approval, publish, audit. High-risk разделы требуют MFA/step-up.

## 2. Schema catalog

Показывает packages, types, predicates, revisions, dependencies и usage. Действия:

- создать draft from published;
- добавить/наследовать type;
- добавить predicate/scalar/relation;
- cardinality/domain/range/temporal;
- identity/normalization;
- validation/index/search;
- classification;
- translations;
- layouts/views;
- validate/impact/publish/retire.

Stable IDs видны, но не редактируются после публикации. Alias collision/confusables проверяются.

## 3. Form/View designer

- sections/tabs/columns;
- field components;
- conditional visibility по безопасному DSL;
- list filters/facets/sorts;
- relation groups;
- actions/capability requirements;
- responsive preview;
- persona/data-policy preview;
- locale/RTL preview;
- accessibility warnings.

Designer не может сделать скрытое policy поле видимым.

## 4. Migration center

- source/target;
- primitive steps;
- sample preview;
- affected entities/statements;
- invalid/ambiguous/collisions;
- index/storage/time estimate;
- dry run;
- approval;
- progress/checkpoint;
- quarantine/issues;
- verification/checksum;
- cutover/compensation.

Publish блокируется при unmapped security rule или required migration issue.

## 5. Users and identity

- user/federated identity status;
- assigned capability groups/data groups отдельно;
- valid intervals;
- MFA/session/device summary;
- revoke sessions;
- service accounts/scopes;
- access review date/owner;
- impersonation/delegation audit.

Password/token/secret не показывается.

## 6. Capability groups

Matrix function/action vs allow. Default deny. Group composition допускается без cycle. Effective preview для пользователя показывает origin rules. Capability group не содержит data selectors.

## 7. Data groups/policies

Builder:

- target tenant/type/entity/predicate/relation/classification;
- actions read/create/edit/retire/traverse/export/history;
- row conditions;
- fields/masks;
- validity/purpose;
- allow/deny;
- owner/legal basis.

Simulation:

- principal/persona;
- example query/resource;
- resulting obligations/SQL plan summary;
- before/after access impact;
- aggregate/side-channel warnings.

Нельзя публиковать rule с raw SQL.

## 8. Four-eyes publish

Schema/security/connector high-risk changes:

1. author submits;
2. independent approver reviews diff/impact;
3. step-up;
4. publish;
5. notifications/audit.

Author не approve собственный change. Emergency path — break-glass и post-review.

## 9. Connectors

- catalog/instance;
- operations/capabilities;
- endpoint/proxy (safe);
- secret presence/version/rotate;
- license/legal purpose;
- quota/rate;
- mapping version;
- health/last success/error redacted;
- test connection;
- enable/disable;
- raw payload retention;
- affected monitors.

Secret value write-only.

## 10. Risk/workflow editor

### Criteria

Typed expression builder, inputs, severity/weight, explanation, evidence, version/effective dates и test cases. Preview на synthetic/authorized sample.

### Workflow

States/transitions, tasks, assignments, SLA/calendar, conditions, actions, escalation, compensation. Linter detects unreachable states, no terminal state, loops without guard/cooldown и missing permissions.

## 11. Reports

Definitions/templates, parameters, dataset AST, columns/sections, formats, classification, schedule, recipients, locale, limits. Preview использует synthetic or explicitly authorized data. Template variables allowlisted.

## 12. Translations

Matrix resource/key × locale:

- completeness/fallback;
- labels/help/errors/templates;
- ICU plural preview;
- length overflow;
- RTL preview;
- import/export translation bundle;
- draft/review/publish.

User business data не переводится автоматически этим экраном.

## 13. Jobs/projections

Queues, lag, dead letters, leases, retry/cancel, projection generations/checkpoints/checksum. Rebuild требует capability/reason, создаёт новую generation. Нельзя вручную править watermark.

## 14. Audit explorer

Filters actor/action/resource/time/decision/request. Detail показывает safe diff, policy/schema revision, original/effective actor, reason и integrity status. Export — отдельное право/limit/watermark.

## 15. System settings

Tenant locale/timezone/retention, brand tokens, limits, origins, OIDC metadata refs, calendars, feature flags, maintenance windows. DB URL/secrets не редактируются обычным tenant admin.

## 16. Protection from operator error

- explicit environment banner;
- destructive confirmation с resource name/reason;
- impact/dry run;
- optimistic concurrency;
- undo через new revision;
- no bulk select «все» без estimate;
- time-limited grants;
- diff in stable IDs + labels;
- prevent orphan policy/view/workflow refs;
- publish locks краткие и observable.

## 17. Acceptance

- схема ФЛ/ЮЛ создаётся UI и публикуется;
- field rename не меняет ID/data;
- identity change показывает collisions;
- person deny simulation удаляет поля/edges/query;
- user получает отдельные group classes;
- connector secret не читается назад;
- translation/RTL preview;
- policy/schema publish требует second approver;
- audit связывает draft, approvals и publish.
