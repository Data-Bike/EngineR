-- Upload sessions gain an owner: staged-upload chunk/complete/abort and
-- status polling previously trusted the upload_id alone, so any principal
-- who learned a session id could push bytes into (or complete/abort) a
-- stranger's upload. `created_by` ties a session to its uploader; NULL on
-- pre-existing rows means "unknown" and is treated as unclaimable.
ALTER TABLE core.upload_session
    ADD COLUMN IF NOT EXISTS created_by uuid;
