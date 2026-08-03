#!/bin/bash
set -e

echo "=== EngineR API starting ==="

# Wait for postgres (use env var for host)
PGHOST=${PGHOST:-postgres}
echo "Waiting for postgres at ${PGHOST}:5432..."
until pg_isready -h "$PGHOST" -U enginer_admin -d enginer 2>/dev/null; do
  sleep 1
done
echo "Postgres is ready."

# Run migrations as enginer_migrate
echo "Running migrations..."
for f in /app/migrations/*.sql; do
  echo "  Applying: $(basename $f)"
  PGPASSWORD=enginer_migrate psql -h "$PGHOST" -U enginer_migrate -d enginer -v ON_ERROR_STOP=1 -f "$f" >/dev/null 2>&1 || {
    echo "  (skipped or already applied)"
  }
done
echo "Migrations complete."

# Start API
echo "Starting enginer-api on :${ROCKET_PORT:-8010}"
exec /app/enginer-api
