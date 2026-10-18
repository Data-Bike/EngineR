-- Application-assigned bigint ids (entity_id, statement_id) come from
-- sequences; starts high so manually seeded low ids never collide.

CREATE SEQUENCE IF NOT EXISTS core.entity_id_seq START WITH 1000000;
CREATE SEQUENCE IF NOT EXISTS core.statement_id_seq START WITH 1000000;
GRANT USAGE, SELECT ON SEQUENCE core.entity_id_seq TO enginer_app, enginer_worker;
GRANT USAGE, SELECT ON SEQUENCE core.statement_id_seq TO enginer_app, enginer_worker;

-- core.entity.type_id: required by the domain aggregate (Entity.type_id) and
-- read by PgEntityRepository, but was missing from the baseline migration.
ALTER TABLE core.entity ADD COLUMN IF NOT EXISTS type_id uuid;
CREATE INDEX IF NOT EXISTS entity_tenant_type ON core.entity (tenant_id, type_id)
    WHERE retired_at IS NULL;
