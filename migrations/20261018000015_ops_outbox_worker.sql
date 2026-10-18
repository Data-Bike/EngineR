-- The worker drains outbox events across tenants; tenant-scoped policies
-- would hide all rows when app.tenant_id is unset. Permissive policies are
-- OR'ed, so this gives enginer_worker full visibility on ops.outbox while
-- enginer_app remains tenant-scoped.

CREATE POLICY worker_drain ON ops.outbox
    TO enginer_worker
    USING (true)
    WITH CHECK (true);
