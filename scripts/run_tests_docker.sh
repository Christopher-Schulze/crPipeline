#!/bin/bash
set -e

# Ensure running from repository root
cd "$(dirname "$0")/.."

# Start required services
docker compose up -d db redis minio
trap 'docker compose down' EXIT

# Wait for Postgres to accept connections
printf "Waiting for database to be ready"
until docker compose exec -T db pg_isready -U postgres >/dev/null 2>&1; do
  printf '.'
  sleep 1
done
printf "\n"

# Create test database if missing
if ! docker compose exec -T db psql -U postgres -tAc "SELECT 1 FROM pg_database WHERE datname='testdb'" | grep -q 1; then
  docker compose exec -T db psql -U postgres -c 'CREATE DATABASE testdb;'
fi

# Run backend and frontend tests
cargo test --manifest-path backend/Cargo.toml --all-targets
npm test --prefix frontend

