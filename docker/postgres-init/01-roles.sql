-- Provision application roles with passwords for local development.
-- In production these roles are created by the deployment without passwords
-- (mTLS/certificate auth) and the migrate role is the migration owner.

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'enginer_app') THEN
    CREATE ROLE enginer_app LOGIN NOCREATEDB NOCREATEROLE NOSUPERUSER NOBYPASSRLS PASSWORD 'enginer_app';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'enginer_worker') THEN
    CREATE ROLE enginer_worker LOGIN NOCREATEDB NOCREATEROLE NOSUPERUSER NOBYPASSRLS PASSWORD 'enginer_worker';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'enginer_migrate') THEN
    CREATE ROLE enginer_migrate LOGIN NOCREATEDB NOCREATEROLE NOSUPERUSER PASSWORD 'enginer_migrate';
  END IF;
END $$;

-- enginer_migrate owns the schemas it creates. Migrations run as enginer_migrate.
GRANT CREATE ON DATABASE enginer TO enginer_migrate;
-- enginer_app / enginer_worker get usage on schemas (grants on tables come from migrations).
