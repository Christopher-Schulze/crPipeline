#!/bin/bash
# Fetch Rust and Node dependencies to generate lockfiles.
set -e

# Ensure running from project root
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null; then
  echo "cargo not found" >&2
  exit 1
fi

if ! command -v npm >/dev/null; then
  echo "npm not found" >&2
  exit 1
fi

# Generate Cargo.lock and vendor folder
echo "Fetching Rust crates..."
cargo fetch --manifest-path backend/Cargo.toml
cargo generate-lockfile --manifest-path backend/Cargo.toml

# Install Node packages
echo "Installing node packages..."
cd frontend
npm install
cd ..

echo "Dependency setup complete."
