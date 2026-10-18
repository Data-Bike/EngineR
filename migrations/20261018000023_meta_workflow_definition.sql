-- Workflow definitions for the SKR process engine (docs/13 §7).
-- Persisted per tenant so the API can validate `definition_id` on
-- `POST /workflows` and guard `POST /workflows/<id>/transition` against
-- illegal state transitions instead of accepting arbitrary strings.

CREATE TABLE IF NOT EXISTS meta.workflow_definition (
    tenant_id       uuid NOT NULL,
    definition_id   text NOT NULL,
    content         jsonb NOT NULL,    -- {"initial_state": ..., "states": [{name, transitions:[{to, action}]}]}
    created_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, definition_id)
);

GRANT SELECT, INSERT, UPDATE ON meta.workflow_definition TO enginer_app, enginer_worker;
