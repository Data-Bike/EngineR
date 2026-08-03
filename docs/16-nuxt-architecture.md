# Архитектура Nuxt / Nuxt UI

## 1. Технологический профиль

- Nuxt 4;
- Vue 3 Composition API;
- TypeScript strict;
- Nuxt UI 4;
- @nuxtjs/i18n;
- Pinia либо Nuxt useState по единому проектному решению; domain server state не дублируется бессистемно;
- generated typed API client из OpenAPI;
- Vitest, Vue Test Utils, Playwright;
- axe-core для accessibility.

Versions фиксируются lockfile, обновляются отдельным dependency PR с contract/E2E tests.

## 2. Rendering mode

Аутентифицированная intranet/admin часть работает SPA client-side (ssr: false), чтобы исключить утечки пользовательского response через SSR/cache и упростить WebSocket/session. Публичные help/login pages МОГУТ быть SSR при отдельном cache policy.

Nuxt server не хранит business state и не обращается к PostgreSQL. Если Nitro используется как same-origin proxy, он только управляет cookie/headers/CSRF и проксирует в Rocket.

## 3. Структура

~~~text
web/
├─ app/
│  ├─ pages/
│  ├─ layouts/
│  ├─ components/
│  │  ├─ entity/
│  │  ├─ graph/
│  │  ├─ query/
│  │  ├─ workflow/
│  │  └─ admin/
│  ├─ composables/
│  ├─ stores/
│  ├─ middleware/
│  ├─ plugins/
│  ├─ assets/
│  └─ error.vue
├─ layers/
│  ├─ core/
│  └─ skr/
├─ generated/api/
├─ i18n/locales/
└─ tests/
~~~

SKR layer содержит route presets/views/translations, но использует core components.

## 4. API client

Единственный wrapper:

- base URL и credentials;
- X-Request-Id/locale/timezone;
- CSRF header;
- timeout/AbortController;
- 401 refresh singleflight;
- 403/404/409/412 typed errors;
- ETag/If-Match;
- idempotency keys;
- cursor;
- no automatic retry mutation без idempotency key;
- redacted diagnostics.

Components не вызывают $fetch напрямую.

## 5. Session и authorization context

GET /me/context возвращает profile, locale/timezone, menu capabilities, available workspaces, policy revision и безопасные presentation grants. Route middleware проверяет coarse capability для UX, но сервер проверяет заново.

PolicyChanged event:

1. invalidate context/caches;
2. cancel active queries;
3. refetch current view;
4. redirect, если screen больше недоступен.

Browser storage не хранит refresh/access token при cookie mode. Sensitive form draft не сохраняется в localStorage по умолчанию.

## 6. Metadata-driven UI

### EntityFormRenderer

Вход:

- type/predicate revision;
- ViewDefinition;
- entity values/revision;
- field permissions/actions;
- locale;
- validation schema.

Field registry:

- text/textarea/rich text (sanitized);
- boolean;
- integer/decimal/money/measurement;
- date/time/datetime/timezone;
- enum/dictionary;
- entity reference/autocomplete;
- multi-value/repeating group;
- address/contact/document;
- file;
- derived/read-only;
- relation editor.

Неизвестный field kind показывает безопасную unsupported component, а не raw JSON editor.

### EntityListRenderer

Columns, filters, facets, sort, row actions и export берутся из view. Virtualization и cursor pagination. Column preference хранит IDs, не labels.

## 7. State

- server state cache key: tenant + resource + revision + view + locale + policy revision;
- editable draft отделён от fetched entity;
- optimistic update только для обратимых низкорисковых команд;
- 412 открывает compare/conflict resolution;
- job state обновляется events + periodic fallback;
- graph state хранит nodes/edges by ID, expansion cursor и visited marks.

Нельзя помещать весь результат большого запроса в global store.

## 8. Layout

Основной layout:

- header: product/workspace, global search, jobs/notifications, locale, profile;
- collapsible left navigation по capabilities и schema menu;
- breadcrumbs;
- page header: title, status, primary actions;
- content;
- optional right context panel;
- global command palette;
- visible environment/classification banner.

Legacy красная тема не копируется. Tenant brand tokens настраиваются, но контраст/состояния сохраняются.

## 9. Search UX

Global search:

- exact identifier mode;
- full-text suggestions;
- type filters;
- recent/saved queries;
- keyboard navigation;
- result highlights только разрешённых fields;
- debounced cancellable requests;
- no query text in analytics if classified.

Advanced search строит AST визуально: type, conditions, relation paths, columns/sort. UI показывает reason, если операция batch-only/forbidden.

## 10. Performance

- route/component lazy loading;
- code split admin/report/graph;
- debounce/cancel;
- cursor/virtual list;
- summary first, relations tabs lazy;
- graph progressive expansion;
- worker для тяжёлой client layout при необходимости;
- no client-side filtering огромного набора;
- no N+1 API calls;
- images/files lazy;
- performance budgets в CI.

## 11. Validation

Client validation улучшает UX, но server authoritative. Server errors привязываются к predicate IDs/row paths и переводятся. UI не теряет draft при 422/412. Decimal хранится строкой до submit.

## 12. Accessibility

WCAG 2.2 AA:

- keyboard/full focus;
- skip links;
- semantic landmarks/headings;
- labels/help/error associations;
- focus после dialog/navigation;
- таблицы с headers/captions;
- graph имеет альтернативный accessible list/tree;
- цвет не единственный носитель статуса;
- prefers-reduced-motion;
- 200% zoom/reflow;
- RTL;
- screen-reader announcements jobs/validation.

## 13. Security клиента

- CSP без unsafe-eval;
- sanitize any rich/source content;
- no v-html для source data;
- external links rel=noopener;
- download через API;
- file preview sandbox;
- no secrets in runtimeConfig.public;
- dependency/SCA;
- clickjacking protection на server;
- auto-lock/session expiry;
- reauthentication for high-risk actions.

## 14. Error handling

error.vue различает offline, unauthenticated, forbidden/not-found, stale, conflict, rate-limited, dependency degraded. Показывает request ID и безопасное действие. Stack/raw payload не показываются.

## 15. Tests

- component registry contract на каждый scalar kind;
- schema-generated random forms;
- mocked OpenAPI;
- Playwright role/persona matrix;
- a11y/keyboard/RTL;
- 412 conflict;
- policy change while page open;
- virtual lists/graph cycles;
- file/import/report workflows;
- screenshot visual regression ключевых layouts.
