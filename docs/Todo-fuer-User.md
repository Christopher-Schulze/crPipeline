# Worker als systemd-Dienst

Dieses Dokument beschreibt kurz, wie der Background-Worker als Dienst eingerichtet wird. Die Anleitung wird zukünftig weiter ausgebaut.

1. Worker-Binary erstellen:
   ```bash
   cargo build --release --bin worker --features worker-bin
   ```
2. Dienst automatisch installieren und starten:
   ```bash
   sudo ./scripts/install_worker_service.sh
   ```
   Der Service erwartet das Projekt unter `/opt/crPipeline`. Passe den Pfad in
   der Unit-Datei an, falls es woanders liegt.

   Alternativ kann die Installation manuell erfolgen:
   ```bash
   sudo mkdir -p /opt/crPipeline/scripts
   sudo cp scripts/worker_service.sh /opt/crPipeline/scripts/
   sudo cp deploy/worker.service /etc/systemd/system/worker.service
   sudo chmod +x /opt/crPipeline/scripts/worker_service.sh
   sudo systemctl daemon-reload
   sudo systemctl enable --now worker.service
   ```
