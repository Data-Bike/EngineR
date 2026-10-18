-- Stable special-role detection + granular user-admin capabilities.
--
-- `special` marks the preset roles with a stable flag: SUPERADMIN /
-- SUPERUSER evaluation no longer depends on the display alias, and every
-- tenant gets its own rows (the previous migration used a fixed group_id,
-- which ON CONFLICT-skipped every tenant except the first).

ALTER TABLE sec."group"
    ADD COLUMN IF NOT EXISTS special text NOT NULL DEFAULT '';

ALTER TABLE sec."group"
    ADD CONSTRAINT group_special_check
    CHECK (special IN ('', 'superadmin', 'superuser'));

-- Backfill rows created before the flag existed.
UPDATE sec."group" SET special = alias
WHERE special = '' AND alias IN ('superadmin', 'superuser');

-- Ensure every tenant has both special roles. The group_id is a deterministic
-- per-tenant uuid so rows do not collide across tenants.
INSERT INTO sec."group" (group_id, tenant_id, class, alias, description, special)
SELECT md5(t.tenant_id::text || ':superadmin')::uuid, t.tenant_id, 'capability',
       'superadmin', 'SUPERADMIN: unrestricted access to everything', 'superadmin'
FROM (SELECT DISTINCT tenant_id FROM sec.principal) t
WHERE NOT EXISTS (
    SELECT 1 FROM sec."group" g
    WHERE g.tenant_id = t.tenant_id AND g.special = 'superadmin')
ON CONFLICT (group_id) DO NOTHING;

INSERT INTO sec."group" (group_id, tenant_id, class, alias, description, special)
SELECT md5(t.tenant_id::text || ':superuser')::uuid, t.tenant_id, 'capability',
       'superuser', 'SUPERUSER: unrestricted access except administrative functions',
       'superuser'
FROM (SELECT DISTINCT tenant_id FROM sec.principal) t
WHERE NOT EXISTS (
    SELECT 1 FROM sec."group" g
    WHERE g.tenant_id = t.tenant_id AND g.special = 'superuser')
ON CONFLICT (group_id) DO NOTHING;

-- Grant the granular user-administration capabilities to the admins group in
-- the currently published policy revision (users.manage remains the umbrella).
INSERT INTO sec.policy_rule
    (tenant_id, policy_revision, effect, group_id, action, priority)
SELECT g.tenant_id, pr.revision_id, 'allow', g.group_id, a.action, 0
FROM sec."group" g
JOIN sec.policy_revision pr
  ON pr.tenant_id = g.tenant_id AND pr.state = 'published'
CROSS JOIN (VALUES ('user.read'), ('user.create'), ('user.edit'), ('user.delete'))
    AS a(action)
WHERE g.alias = 'admins'
  AND NOT EXISTS (
      SELECT 1 FROM sec.policy_rule r
      WHERE r.tenant_id = g.tenant_id AND r.group_id = g.group_id
        AND r.action = a.action);
