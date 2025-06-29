#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building CLI binaries..."
cargo build --bins --manifest-path backend/Cargo.toml

echo "Checking create_admin error output..."
output=$(backend/target/debug/create_admin 2>&1 || true)
if echo "$output" | grep -q "Usage: create_admin <email> <password>"; then
  echo "create_admin error message OK"
else
  echo "Unexpected output from create_admin:" >&2
  echo "$output" >&2
  exit 1
fi

echo "Checking cleanup error output..."
output=$(env -u DATABASE_URL backend/target/debug/cleanup 2>&1 || true)
if echo "$output" | grep -q "environment variable"; then
  echo "cleanup error message OK"
else
  echo "Unexpected output from cleanup:" >&2
  echo "$output" >&2
  exit 1
fi

echo "Manual CLI checks passed."
