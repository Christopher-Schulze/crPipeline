# Worker als systemd-Dienst

Dieses Dokument beschreibt, wie der Background-Worker dauerhaft als Dienst eingerichtet wird.

Die folgenden Schritte richten den Service unter systemd ein.

1. **Worker-Binary erstellen**
   ```bash
   cargo build --release --bin worker --features worker-bin
   ```

2. **Service installieren** – das Skript `scripts/install_worker_service.sh` kopiert die Unit-Datei `deploy/worker.service` nach `/etc/systemd/system/worker.service` und legt unter `/opt/crPipeline` die benötigten Dateien an.
   ```bash
   sudo ./scripts/install_worker_service.sh
   ```
   Passe `WorkingDirectory` und `User` in der Unit-Datei bei Bedarf an, bevor du das Skript ausführst.

3. **Dienst starten und Status prüfen**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now worker.service
   sudo systemctl status worker.service
   ```

4. **Umgebungsvariablen setzen** – `scripts/worker_service.sh` lädt standardmäßig `backend/.env`. Lege diese Datei unter `/opt/crPipeline` ab oder setze in `worker.service` die Variable `ENV_FILE`, z.B.:
   ```
   Environment=ENV_FILE=/opt/crPipeline/backend/.env.prod
   ```

5. **Logrotate einrichten** – um Logdateien in `/var/log/worker.log` zu rotieren, kann folgende Konfiguration unter `/etc/logrotate.d/worker` abgelegt werden:
   ```
   /var/log/worker.log {
       daily
       rotate 7
       compress
       missingok
       notifempty
       copytruncate
   }
   ```
   Ergänze dazu in `worker.service` die Zeile `StandardOutput=append:/var/log/worker.log`.

6. **Neustartstrategie** – in `deploy/worker.service` ist `Restart=always` gesetzt. Damit startet systemd den Worker automatisch neu, falls er unerwartet beendet wird oder beim Systemstart noch nicht läuft.

Alternativ kann die Installation auch manuell erfolgen:
```bash
sudo mkdir -p /opt/crPipeline/scripts
sudo cp scripts/worker_service.sh /opt/crPipeline/scripts/
sudo cp deploy/worker.service /etc/systemd/system/worker.service
sudo chmod +x /opt/crPipeline/scripts/worker_service.sh
sudo systemctl daemon-reload
sudo systemctl enable --now worker.service
```

## Statische Assets ausliefern

Um die gebaute Weboberfläche im Produktivbetrieb bereitzustellen, sollten die Dateien aus
`frontend/dist` von einem Webserver wie nginx ausgeliefert werden. Baue das
Frontend zuvor mit dem DaisyUI-Schritt:

```bash
npm run build:prod --prefix frontend
```

Kopiere danach den Inhalt von `frontend/dist` an den Ort, den dein Webserver
als Dokumentenwurzel nutzt oder binde das Verzeichnis als Volume ein.
