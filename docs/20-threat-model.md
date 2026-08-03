# Модель угроз и меры защиты

## 1. Активы

- PII/идентификационные документы;
- сведения ЮЛ, договоры, лимиты, задолженность, залоги;
- граф связей/аффилированность;
- критерии и решения риска;
- учётные записи/tokens/secrets;
- коннекторные лицензии/raw payload;
- отчёты/выгрузки;
- schemas/policies;
- audit/integrity;
- availability/performance.

## 2. Границы доверия

Browser ↔ Nuxt/proxy ↔ Rocket API ↔ Security PDP ↔ PostgreSQL/object storage ↔ worker ↔ external sources. Каждая граница authenticates, authorizes, validates, limits и audits.

## 3. STRIDE-угрозы

### Spoofing

Угон session, forged service identity, weak local password, OIDC confusion.

Меры: OIDC PKCE/state/nonce, MFA, secure cookie, refresh rotation/reuse detection, token issuer/audience/time validation, mTLS service identity, Argon2id, session revocation.

### Tampering

SQL injection, policy/schema manipulation, event/report/file alteration.

Меры: parameterized sqlx, no raw user SQL, stable IDs, four-eyes publish, signed bundle/content hash, immutable revisions, outbox/inbox, artifact checksums, tamper-evident audit.

### Repudiation

Пользователь отрицает export/decision/override.

Меры: original/effective actor, request/decision IDs, reason, auth strength, immutable event/audit, trusted timestamps/checkpoints.

### Information disclosure

Tenant leak, hidden field/edge/count, logs/cache, report, browser DOM, connector payload.

Меры: PEP rewrite before SQL, RLS, response allowlist, aggregate policy, tenant/policy cache keys, redaction, classified search, encrypted storage, short download, no sensitive telemetry.

### Denial of service

Huge JSON/graph/query/export, zip bomb, connector storm, DB pool exhaustion.

Меры: body/depth/complexity limits, cost guard, bounded queues, statement_timeout, keyset, async jobs, upload scanning, rate/quota/circuit breaker, workload pools/backpressure.

### Elevation

Broken object authorization, admin misuse, service account bypass, policy conflict.

Меры: default deny/deny-overrides, two permission classes, scoped service identity, step-up/four-eyes, break-glass, policy simulation, negative access tests.

## 4. Specific attacks

### SQL injection

Ни одно value не интерполируется. Dynamic identifiers только по catalog mapping. Текущий prototype format SQL должен быть удалён.

### IDOR

Public UUID не даёт доступ. GET/mutation решается PDP; hidden resource может отвечать 404. Related/blob/history отдельные actions.

### Mass assignment

Command DTO deny unknown fields. Predicate allowlist from schema + mutation obligations. System metadata actor/tenant/revision нельзя принять из body.

### XSS/content injection

Vue escaping, no untrusted v-html, rich text sanitizer, CSP, sandbox file preview, safe external links. Source HTML never rendered directly.

### CSRF/CORS

HttpOnly cookie mode: CSRF token, SameSite, Origin/Referer allowlist. CORS exact origins, credentials only approved. Bearer endpoints не смешиваются без policy.

### SSRF

Connector URL нельзя задавать произвольным user input; endpoint allowlist, DNS/IP validation, egress proxy, redirect revalidation, block metadata/private ranges, response size/time limits.

### File attacks

MIME/signature, extension irrelevant, zip entry/ratio/depth limits, macro/external link detection, AV, isolated converter, content-disposition, immutable storage, no execution.

### Formula injection

CSV/XLSX text escaping; templates allowlist; formulas not imported/executed по умолчанию.

### Graph inference

Запрещённый node/edge исключён на каждом recursion step. Counts/facets/path existence также authorized. Timing padded only where risk analysis requires; главное — uniform indexed false path.

### Cache leak

Key включает tenant/principal policy shape/locale/revision. Shared caching private responses запрещён. Policy change invalidates. No caching Authorization header response in CDN.

### Logging leak

Structured allowlist logs; bodies/headers/tokens/secrets/PII excluded. Debug logging production disabled. Error ID вместо raw SQL/payload.

### Supply chain

Pinned lock, cargo-deny/audit, npm audit/SCA, provenance/signatures, protected CI, least-privilege build, SBOM, reproducible container, secret scanning.

## 5. Encryption

- TLS 1.2+ policy, modern ciphers;
- PostgreSQL/storage volumes encrypted;
- application envelope encryption for selected secrets/high-risk values;
- per-tenant/key-purpose key IDs;
- KMS/HSM rotation;
- keyed hashes for sensitive identity keys;
- backups encrypted;
- keys never in repository/log.

Encryption не заменяет access control.

## 6. Data lifecycle

Classification drives retention, archive, legal hold, export и deletion. GDPR/local laws реализуются tenant jurisdiction policy. Right-to-erasure может конфликтовать с legal hold/audit; outcome документируется. Crypto-shredding только после dependency/backup plan.

## 7. Security tests/gates

- unauthenticated/unauthorized matrix;
- cross-tenant property tests;
- hidden field/filter/sort/facet/graph/report/export;
- policy conflict/deny;
- RLS pooled context leak;
- CSRF/CORS/IDOR/mass assignment;
- SQL/AST fuzz;
- upload/zip bomb/formula;
- SSRF redirects/DNS rebinding;
- dependency/container scan;
- secrets scan;
- external penetration test до production.

Любая подтверждённая data/tenant leak — release blocker.

## 8. Incident readiness

Runbook: detect, revoke sessions/keys, disable connector/feature, preserve evidence, assess affected tenants/data, notify по требованиям, patch, rotate, verify, postmortem. Audit/storage clocks синхронизированы. Emergency access не уничтожает доказательства.
