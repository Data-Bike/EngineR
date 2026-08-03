-- EngineR baseline: extensions, roles, schemas.
-- docs/10 §1-2.
--
-- The application DB role is NOT the owner and has no BYPASSRLS (docs/08 §8).
-- Roles here are created idempotently; in production they are provisioned by
-- the deployment, not by migrations. We create them only if they do not exist.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "citext";
-- pg_trgm is optional (trigram search); enable when the search profile needs it.
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Schemas (docs/10 §2).
CREATE SCHEMA IF NOT EXISTS core;
CREATE SCHEMA IF NOT EXISTS meta;
CREATE SCHEMA IF NOT EXISTS sec;
CREATE SCHEMA IF NOT EXISTS ops;
CREATE SCHEMA IF NOT EXISTS audit;
CREATE SCHEMA IF NOT EXISTS integration;
CREATE SCHEMA IF NOT EXISTS report;

-- Application roles (idempotent). DO NOT use these as migration owner.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'enginer_app') THEN
    CREATE ROLE enginer_app LOGIN NOCREATEDB NOCREATEROLE NOSUPERUSER NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'enginer_worker') THEN
    CREATE ROLE enginer_worker LOGIN NOCREATEDB NOCREATEROLE NOSUPERUSER NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'enginer_migrate') THEN
    CREATE ROLE enginer_migrate LOGIN NOCREATEDB NOCREATEROLE NOSUPERUSER;
  END IF;
END $$;

-- Default privileges: application roles get usage on schemas; grants on
-- individual tables are added by later migrations after table creation.
GRANT USAGE ON SCHEMA core, meta, sec, ops, audit, integration, report TO enginer_app, enginer_worker;
