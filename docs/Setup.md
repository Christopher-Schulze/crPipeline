## Setup

1. Copy environment variables:
   ```bash
   cp backend/.env.example backend/.env
   ```
2. Ensure PostgreSQL and MinIO are running locally. Update `backend/.env` if your services use custom ports or credentials.
   Alternatively run `docker compose up -d db minio` to start the services via Docker.
3. Install Rust and Node dependencies (requires network access):
   ```bash
   ./scripts/bootstrap_deps.sh
   ```
4. Run database migrations:
   ```bash
   (cd backend && sqlx migrate run)
   ```
5. Start the backend and frontend in separate terminals:
   ```bash
   cargo run --manifest-path backend/Cargo.toml
   npm run dev --prefix frontend
   ```
6. The backend will be on `http://localhost:8080`, frontend on `http://localhost:5173`.

7. Backend tests read `backend/.env.test` for `DATABASE_URL_TEST`. Edit the file
   if your test database differs and run:
   ```bash
   cargo test --manifest-path backend/Cargo.toml
   ```

Environment variables can be tweaked in `backend/.env` to point to a different database or S3 endpoint. Ensure the bucket defined in `S3_BUCKET` exists in your MinIO or AWS account.
`PROCESS_ONE_JOB` makes the worker exit after a single job. Setting `LOCAL_S3_DIR` lets the worker store uploaded files under that path instead of S3, handy for local tests.
`WORKER_CONCURRENCY` controls how many jobs a single worker processes in parallel.
`SHUTDOWN_AFTER_IDLE` shuts the worker down after the given minutes of inactivity.

The backend optionally supports an external OCR service. Set `OCR_API_ENDPOINT` and `OCR_API_KEY` in `backend/.env` to provide a global endpoint and API key used when no organization or stage-specific OCR configuration is present.

### Docker Compose
All services can also be started via Docker for convenience:

```bash
docker compose up --build
```

This launches Postgres, MinIO, Redis, the backend API and the compiled frontend. The
application will be available on the same ports as above.

When preparing a production build, generate the DaisyUI themes before the regular
Vite build:

```bash
npm run build:prod --prefix frontend
```

After the first migration you can seed an admin user (role `admin`) with the following command:
```bash
cargo run --bin create_admin -- email@example.com password
```

Run `scripts/seed_demo.sh` afterwards to insert example documents stored under `LOCAL_S3_DIR`. The script requires `psql` on your `PATH`.

For production instructions see [Deployment](Deployment.md). To expose metrics and dashboards consult [Monitoring](Monitoring.md).

## Worker als Dienst

Der Hintergrund-Worker kann dauerhaft als systemd-Dienst laufen.

1. **Worker-Binary erstellen**
   ```bash
   cargo build --release --bin worker --features worker-bin
   ```

2. **Service installieren** – das Skript `scripts/install_worker_service.sh`
   kopiert die Unit-Datei `deploy/worker.service` nach `/etc/systemd/system/worker.service`,
   legt unter `/opt/crPipeline` die notwendigen Dateien an und erzeugt bei Bedarf
   automatisch den Systemnutzer `crpipeline`.
   ```bash
   sudo ./scripts/install_worker_service.sh
   ```
   Passe die Pfade über die Umgebungsvariablen `TARGET` und `UNIT_FILE` oder den
   Benutzernamen über `SERVICE_USER` an, falls andere Werte gewünscht sind.

3. **Dienst starten und Status prüfen**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now worker.service
   sudo systemctl status worker.service
   ```

4. **Umgebungsvariablen setzen** – `scripts/worker_service.sh` lädt standardmäßig
   `backend/.env`. Lege diese Datei unter `/opt/crPipeline` ab oder setze in
   `worker.service` die Variable `ENV_FILE`, z.B.:
   ```
   Environment=ENV_FILE=/opt/crPipeline/backend/.env.prod
   ```

5. **Logrotate einrichten** – um Logdateien in `/var/log/worker.log` zu rotieren,
   kann folgende Konfiguration unter `/etc/logrotate.d/worker` abgelegt werden:
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

6. **Neustartstrategie** – in `deploy/worker.service` ist `Restart=always` gesetzt.
   Damit startet systemd den Worker automatisch neu, falls er unerwartet beendet
   wird oder beim Systemstart noch nicht läuft.

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

Um die gebaute Weboberfläche im Produktivbetrieb bereitzustellen,
sollten die Dateien aus `frontend/dist` von einem Webserver wie nginx
ausgeliefert werden. Baue das Frontend zuvor mit dem DaisyUI-Schritt:

```bash
npm run build:prod --prefix frontend
```

Kopiere danach den Inhalt von `frontend/dist` an den Ort, den dein Webserver
als Dokumentenwurzel nutzt oder binde das Verzeichnis als Volume ein.

