-- Production authentication support (docs/08 §2, docs/09 §3):
--   * sec.principal_credential — login + argon2id password hash. Deliberately
--     NOT under RLS: the credential lookup runs before any tenant context
--     exists; `login` is globally unique and the row itself yields the tenant.
--   * sec.session — extended with CSRF token + client metadata, and taken out
--     of RLS: the session cookie IS the credential, its PK lookup must succeed
--     before `app.tenant_id` can be installed. Tenant scoping of session rows
--     is enforced in application code (all lookups bind the PK).

CREATE TABLE IF NOT EXISTS sec.principal_credential (
    credential_id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       uuid NOT NULL,
    principal_id    uuid NOT NULL REFERENCES sec.principal(principal_id),
    login           text NOT NULL,
    password_hash   text NOT NULL,                    -- argon2id PHC string
    status          text NOT NULL DEFAULT 'active',   -- active|disabled
    failed_attempts int  NOT NULL DEFAULT 0,
    locked_until    timestamptz,
    updated_at      timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS principal_credential_login
    ON sec.principal_credential (login);
CREATE INDEX IF NOT EXISTS principal_credential_principal
    ON sec.principal_credential (tenant_id, principal_id);

GRANT SELECT, INSERT, UPDATE ON sec.principal_credential TO enginer_app;

ALTER TABLE sec.session ADD COLUMN IF NOT EXISTS csrf_token  text;
ALTER TABLE sec.session ADD COLUMN IF NOT EXISTS user_agent  text;
ALTER TABLE sec.session ADD COLUMN IF NOT EXISTS ip_address  text;
ALTER TABLE sec.session ADD COLUMN IF NOT EXISTS last_seen_at timestamptz;

-- Session is the presented credential; unscoped PK lookup must work pre-context.
ALTER TABLE sec.session NO FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_isolation_session ON sec.session;
ALTER TABLE sec.session DISABLE ROW LEVEL SECURITY;
