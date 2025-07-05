#!/bin/bash
# Install worker systemd service
set -e

# Run from repository root
cd "$(dirname "$0")/.."

TARGET=/opt/crPipeline
UNIT_FILE=/etc/systemd/system/worker.service

sudo mkdir -p "$TARGET/scripts"

# Copy files for reference
sudo cp deploy/worker.service "$TARGET/worker.service"
sudo cp scripts/worker_service.sh "$TARGET/scripts/"

sudo chmod +x "$TARGET/scripts/worker_service.sh"

# Install systemd unit
sudo cp deploy/worker.service "$UNIT_FILE"

sudo systemctl daemon-reload
sudo systemctl enable --now worker.service
