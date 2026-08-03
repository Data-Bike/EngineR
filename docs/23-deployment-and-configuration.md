# Развёртывание и конфигурация

## 1. Поддерживаемые топологии

### On-prem single site

Load balancer → несколько enginer-api → PostgreSQL primary/replica → object storage → workers. Security PDP local crate/internal endpoint. Подходит intranet.

### HA multi-zone

API/workers по зонам, PostgreSQL HA, replicated object storage, redundant egress/proxy, centralized OIDC/KMS/observability.

### Dedicated tenant

Крупный/регулируемый tenant получает отдельный DB/storage shard, но тот же application release/contracts.

## 2. Containers

Минимальный non-root runtime image:

- pinned digest;
- read-only filesystem;
- writable tmp bounded;
- no shell/package manager if possible;
- seccomp/capabilities drop;
- health endpoints;
- SBOM/signature/provenance;
- UTC OS timezone, locale data/font packages only needed;
- CA trust controlled.

API и worker разные images/commands или один image с explicit binary.

## 3. Environments

local, test, staging, production. Production data/secrets не копируются вниз. Каждая среда имеет own OIDC clients, DB, storage, keys, domains и connector accounts. UI показывает environment banner вне production и safe production marker.

## 4. Typed configuration

Пример categories:

~~~toml
[http]
public_origin = "https://skr.example"
request_timeout_ms = 5000
max_json_bytes = 1048576

[database]
url_secret_ref = "secret://enginer/db"
max_connections = 40
acquire_timeout_ms = 500
statement_timeout_ms = 4000
db_executor_queue = 200

[security]
policy_cache_ttl_seconds = 30
fail_closed = true
internal_mtls_identity = "enginer-api"

[storage]
backend = "s3"
bucket = "enginer-prod"
encryption_key_ref = "kms://..."

[jobs]
lease_seconds = 60
max_attempts = 8
~~~

Файл не содержит passwords/tokens. Environment overrides имеют explicit prefix и typed parse. Unknown production setting — startup error.

## 5. Secrets

Secret manager/KMS:

- DB credential;
- OIDC client secret;
- token signing/encryption keys;
- connector credentials;
- object storage keys;
- keyed identity hash keys;
- SMTP/webhook credentials.

Приложение получает short-lived credentials where possible. Rotation поддерживает old/new overlap. Secret value не логируется/возвращается UI.

## 6. TLS/network

- HTTPS/mTLS termination с trusted proxy contract;
- Rocket доверяет forwarded headers только known proxies;
- DB TLS verification;
- network policies: browser→public only, API→DB/PDP/storage, worker→DB/storage/egress, no DB public;
- egress allowlist/proxy;
- DNS/time services redundant;
- certificate expiry monitoring.

## 7. CI/CD

1. format/lint/unit;
2. integration PostgreSQL/security;
3. API/UI contract;
4. SCA/secret/SBOM/container;
5. migration fresh/upgrade;
6. E2E/a11y;
7. performance smoke;
8. signed artifacts;
9. deploy staging;
10. smoke/canary;
11. approval;
12. production rolling/canary;
13. post-deploy SLO/security checks.

## 8. Database deploy

Expand migration до new app. sqlx migrate отдельным job с advisory lock и owner role. Backfill — resumable worker, не one-shot blocking DDL. Contract migration после old app drained и verification. CREATE INDEX CONCURRENTLY/partition operations по runbook.

## 9. Rollout

- backward-compatible app first;
- feature/schema flags default off;
- canary tenant/instances;
- monitor SLO/errors/DB/outbox/PDP;
- gradual enable;
- rollback binary while expanded schema compatible;
- irreversible external side effect требует compensation, не deploy rollback.

## 10. Nuxt delivery

Static client assets hashed/immutable; index no-cache/revalidate. API same-origin preferred. runtime public config содержит только public API origin/build/version. Source maps private/protected. CSP/SRI where applicable.

## 11. Readiness order

1. secrets/config;
2. DB TLS/connectivity;
3. migration compatibility;
4. policy/schema load;
5. object storage/critical dependencies;
6. start workers/routes;
7. readiness true.

Worker не обрабатывает job с unsupported schema/event version.

## 12. Scaling

HPA/instance scaling по CPU + request/queue metrics, но DB connection global budget сохраняется. Worker scaling по queue class. Scale-to-zero запрещён для latency-critical API/PDP. Tenant fairness enforced quotas.

## 13. Backup/DR

См. [operations](./21-observability-and-operations.md). Deployment pipeline не изменяет backup retention без отдельного approval. Restore environment закрыт от external connector side effects.

## 14. Production checklist

- no default/test passwords;
- migrations verified;
- RLS FORCE and role grants;
- OIDC/MFA/CSRF/CORS/CSP;
- keys/certs/rotation;
- backup restore current;
- audit chain;
- resource limits;
- alerts/runbooks/on-call;
- connector legal/quotas;
- locale/fonts;
- load/penetration tests;
- rollback/compensation.
