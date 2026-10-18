-- Principal email — used for user administration search (spec: filter by
-- name/email/IP) and shown in the users list/detail.
ALTER TABLE sec.principal ADD COLUMN IF NOT EXISTS email text;

CREATE INDEX IF NOT EXISTS principal_email_lower_idx
    ON sec.principal (tenant_id, lower(email)) WHERE email IS NOT NULL;
