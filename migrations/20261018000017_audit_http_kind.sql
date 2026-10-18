-- Allow 'http_request' audit kind for the mutation audit fairing
-- (docs/08 §11 — every API mutation is recorded post-response).

ALTER TABLE audit.event DROP CONSTRAINT IF EXISTS audit_event_kind_check;
ALTER TABLE audit.event
    ADD CONSTRAINT audit_event_kind_check CHECK (kind IN (
        'authentication', 'decision', 'denied_request', 'policy_change',
        'export', 'blob_read', 'bulk_action', 'break_glass', 'impersonation',
        'http_request'
    ));
