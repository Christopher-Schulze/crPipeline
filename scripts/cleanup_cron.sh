#!/bin/bash
# Run the cleanup binary built in release mode. Intended for cron.
set -e
cd "$(dirname "$0")/.."
exec ./target/release/cleanup
