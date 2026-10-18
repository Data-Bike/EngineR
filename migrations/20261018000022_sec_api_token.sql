-- API tokens for service accounts and programmatic access (docs/08 §2,
-- docs/09 §3):
--   * sec.api_token — a bearer credential bound to a principal. Deliberately
--     NOT under RLS for the same reason as sec.session: the token IS the
--     presented credential, its hash lookup must succeed before any tenant
--     context can be installed. Tenant scoping is enforced in application
--     code (all lookups bind token_hash / (tenant_id, principal_id)).
--   * Only the sha256 hash of the presented token is stored; the plaintext
--     token is returned exactly once at creation.

CREATE TABLE IF NOT EXISTS sec.api_token (
    token_id        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       uuid NOT NULL,
    principal_id    uuid NOT NULL REFERENCES sec.principal(principal_id),
    name            text NOT NULL,
    token_hash      text NOT NULL,                    -- sha256 hex of the token
    created_at      timestamptz NOT NULL DEFAULT now(),
    created_by      uuid,
    expires_at      timestamptz,                      -- NULL = never expires
    revoked_at      timestamptz,
    last_used_at    timestamptz
);
CREATE UNIQUE INDEX IF NOT EXISTS api_token_hash ON sec.api_token (token_hash);
CREATE INDEX IF NOT EXISTS api_token_principal
    ON sec.api_token (tenant_id, principal_id);

GRANT SELECT, INSERT, UPDATE ON sec.api_token TO enginer_app, enginer_worker;
