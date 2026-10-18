-- The fact-dedup unique index embedded the raw `object` JSONB in a B-tree:
-- scalar values longer than ~2.7KB exceeded Postgres' index tuple cap and
-- inserts failed with a generic error (long text fields were unwritable).
-- Index a hash of the object instead; the app layer verifies full-fact
-- equality (docs/04 §8) and the stored `fingerprint` covers the same tuple.
DROP INDEX IF EXISTS core.statement_active_fact_unique;

CREATE UNIQUE INDEX statement_active_fact_unique
    ON core.statement (tenant_id, subject, predicate, object_kind, md5(object::text), valid_from)
    WHERE status = 'active';
