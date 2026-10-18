-- Role hierarchy + special roles (superadmin/superuser) + membership dedup.

-- Tree of roles: a member of a child group inherits all rules of its
-- ancestor groups (resolved via recursive CTE on parent_id).
ALTER TABLE sec."group"
    ADD COLUMN IF NOT EXISTS parent_id uuid NULL
        REFERENCES sec."group"(group_id);

CREATE INDEX IF NOT EXISTS group_parent ON sec."group"(parent_id);

-- Prevent inheritance cycles: depth-first check before insert/update.
CREATE OR REPLACE FUNCTION sec.check_group_cycle() RETURNS trigger AS $$
DECLARE
    cur uuid := NEW.parent_id;
    depth int := 0;
BEGIN
    WHILE cur IS NOT NULL AND depth < 64 LOOP
        IF cur = NEW.group_id THEN
            RAISE EXCEPTION 'group hierarchy cycle detected';
        END IF;
        SELECT parent_id INTO cur FROM sec."group" WHERE group_id = cur;
        depth := depth + 1;
    END LOOP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS group_cycle_check ON sec."group";
CREATE TRIGGER group_cycle_check
    BEFORE INSERT OR UPDATE OF parent_id ON sec."group"
    FOR EACH ROW EXECUTE FUNCTION sec.check_group_cycle();

-- Dedup memberships: keep the earliest open interval per (tenant, principal, group).
DELETE FROM sec.principal_group_membership a
USING sec.principal_group_membership b
WHERE a.tenant_id = b.tenant_id
  AND a.principal_id = b.principal_id
  AND a.group_id = b.group_id
  AND a.valid_from > b.valid_from;

-- Special preset roles.
INSERT INTO sec."group" (group_id, tenant_id, class, alias, description)
SELECT '00000000-0000-0000-0000-0000000000c0', t.tenant_id, 'capability', 'superadmin',
       'SUPERADMIN: unrestricted access to everything'
FROM (SELECT DISTINCT tenant_id FROM sec.principal) t
ON CONFLICT (group_id) DO NOTHING;

INSERT INTO sec."group" (group_id, tenant_id, class, alias, description)
SELECT '00000000-0000-0000-0000-0000000000c1', t.tenant_id, 'capability', 'superuser',
       'SUPERUSER: unrestricted access except administrative functions'
FROM (SELECT DISTINCT tenant_id FROM sec.principal) t
ON CONFLICT (group_id) DO NOTHING;

-- The seeded admin persona is the superadmin.
INSERT INTO sec.principal_group_membership
    (tenant_id, principal_id, group_id, valid_from, groups_revision)
SELECT c.tenant_id, c.principal_id, '00000000-0000-0000-0000-0000000000c0',
       '2025-01-01 00:00:00+00', 1
FROM sec.principal_credential c
WHERE c.login = 'admin'
  AND NOT EXISTS (
      SELECT 1 FROM sec.principal_group_membership m
      WHERE m.tenant_id = c.tenant_id AND m.principal_id = c.principal_id
        AND m.group_id = '00000000-0000-0000-0000-0000000000c0'
        AND m.valid_to IS NULL);

-- users.manage capability rule for the admins group.
INSERT INTO sec.policy_rule
    (tenant_id, policy_revision, effect, group_id, action, priority)
SELECT g.tenant_id, 1, 'allow', g.group_id, 'users.manage', 0
FROM sec."group" g
WHERE g.alias = 'admins'
  AND NOT EXISTS (
      SELECT 1 FROM sec.policy_rule r
      WHERE r.tenant_id = g.tenant_id AND r.group_id = g.group_id
        AND r.action = 'users.manage');
