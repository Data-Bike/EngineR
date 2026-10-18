-- The worker schedules due monitor runs: it must see monitors across
-- tenants to enqueue them (tenant-scoped policy would hide everything when
-- app.tenant_id is unset). Same pattern as ops.outbox worker_drain.

CREATE POLICY worker_schedule ON integration.monitor
    TO enginer_worker
    USING (true)
    WITH CHECK (true);
