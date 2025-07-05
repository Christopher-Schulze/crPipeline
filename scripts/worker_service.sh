#!/bin/bash
# Start the worker binary after loading environment variables.
set -e

# Run from repository root
cd "$(dirname "$0")/.."

ENV_FILE=${ENV_FILE:-backend/.env}
if [ ! -f "$ENV_FILE" ]; then
  echo "$ENV_FILE not found" >&2
  exit 1
fi

set -a
source "$ENV_FILE"
set +a

# Default to 1 if not set
: "${WORKER_CONCURRENCY:=1}"
export WORKER_CONCURRENCY

exec ./target/release/worker
