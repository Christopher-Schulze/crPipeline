#!/bin/bash
set -e

if [ ! -f backend/.env ]; then
  echo "Copying backend/.env"
  cp backend/.env.example backend/.env
fi

# Ensure test environment file exists for integration tests
if [ ! -f backend/.env.test ]; then
  echo "Copying backend/.env.test"
  cp backend/.env.test.example backend/.env.test
fi

./scripts/bootstrap_deps.sh

echo "Running migrations..."
(cd backend && sqlx migrate run)

cat <<EOF
Setup complete. Start services in separate terminals:

  cargo run --manifest-path backend/Cargo.toml
  npm run dev --prefix frontend

Backend will listen on http://localhost:8080
Frontend on http://localhost:5173
EOF
