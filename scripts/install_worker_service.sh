#!/bin/bash
# Install worker systemd service
set -e

# Run from repository root
cd "$(dirname "$0")/.."

# Configurable paths
TARGET=${TARGET:-/opt/crPipeline}
UNIT_FILE=${UNIT_FILE:-/etc/systemd/system/worker.service}
SERVICE_USER=${SERVICE_USER:-crpipeline}
SERVICE_GROUP=${SERVICE_GROUP:-$SERVICE_USER}

sudo mkdir -p "$TARGET/scripts"

# Create service user and group if missing
if ! getent group "$SERVICE_GROUP" >/dev/null; then
    sudo groupadd --system "$SERVICE_GROUP"
fi
if ! id "$SERVICE_USER" >/dev/null 2>&1; then
    sudo useradd --system --gid "$SERVICE_GROUP" --home "$TARGET" \
        --shell /usr/sbin/nologin "$SERVICE_USER"
fi

# Copy files for reference
sudo cp deploy/worker.service "$TARGET/worker.service"
sudo cp scripts/worker_service.sh "$TARGET/scripts/"

sudo chmod +x "$TARGET/scripts/worker_service.sh"

# Install systemd unit with adjusted paths
sudo sed -e "s|WorkingDirectory=.*|WorkingDirectory=$TARGET|" \
         -e "s|ExecStart=.*|ExecStart=$TARGET/scripts/worker_service.sh|" \
         -e "s|User=.*|User=$SERVICE_USER|" \
         deploy/worker.service | sudo tee "$UNIT_FILE" >/dev/null

sudo systemctl daemon-reload
sudo systemctl enable --now "$(basename "$UNIT_FILE")"
