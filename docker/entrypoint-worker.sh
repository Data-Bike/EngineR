#!/bin/bash
set -e

echo "=== EngineR Worker starting ==="

PGHOST=${PGHOST:-postgres}
echo "Waiting for postgres at ${PGHOST}:5432..."
until pg_isready -h "$PGHOST" -U enginer_admin -d enginer 2>/dev/null; do
  sleep 1
done
echo "Postgres is ready."

echo "Starting enginer-worker"
exec /app/enginer-worker
