# crPipeline

Multi-tenant document analysis platform built with Rust and Svelte.

## Requirements
- Rust toolchain
- Node.js (npm)
- PostgreSQL and MinIO (or AWS S3)

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

Environment variables can be tweaked in `backend/.env` to point to a different
database or S3 endpoint. Ensure the bucket defined in `S3_BUCKET` exists in your
MinIO or AWS account.

### Docker Compose
All services can also be started via Docker for convenience:

```bash
docker compose up --build
```

This launches Postgres, MinIO, Redis, the backend API and the compiled frontend. The
application will be available on the same ports as above.

After the first migration you can seed an admin user (role `admin`) with the following command:
```bash
cargo run --bin create_admin -- email@example.com password
```

## Migrations
Run migrations using `sqlx`:
```bash
cd backend
sqlx migrate run
```

## Testing
- `cargo test` – backend unit tests
- `npm run build --prefix frontend` – ensure Svelte app compiles

## Dev vs Prod
During development use `cargo run` and `npm run dev`. For production build the frontend and compile the backend in release mode as shown above.

## Environment Variables
`backend/.env` defines all required settings:

```
DATABASE_URL=postgres://postgres:postgres@localhost/db
JWT_SECRET=changeme
AWS_ENDPOINT=http://localhost:9000
AWS_ACCESS_KEY=minioadmin
AWS_SECRET_KEY=minioadmin
S3_BUCKET=uploads
FRONTEND_ORIGIN=http://localhost:5173
REDIS_URL=redis://localhost/
AI_API_URL=https://api.example.com/ai
AI_API_KEY=changeme
SMTP_SERVER=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=your_username
SMTP_PASSWORD=your_password
SMTP_FROM=noreply@example.com
BASE_URL=http://localhost:8080
```

`BASE_URL` is used when generating confirmation and reset links. Set `AWS_ENDPOINT` to your MinIO or AWS S3 endpoint. In production, remove this variable to use AWS defaults.

### Health check
Verify the server is running with:

```
GET /api/health
```

The endpoint simply returns `ok`.

## Authentication
Use the following endpoints to register and log in. Successful login sets a `token` cookie (HttpOnly) automatically sent with future requests.

```
POST /api/register
POST /api/login
GET  /api/confirm/{token}
POST /api/request_reset
POST /api/reset_password
```

Registration requires an `org_id` specifying the organization the user belongs
to and accepts an optional `role` (defaults to `user`).

Check the authentication state with:

```
GET /api/me
```

If the cookie is valid the response includes `user_id`, `org_id` and the user `role`.

## Organizations
Admins can manage organizations via these endpoints:

```
POST /api/orgs
GET  /api/orgs
```

Creating an organization automatically generates an API key and default settings.
The admin panel in the frontend lists existing organizations and allows creating new ones.
Email confirmation and password reset links are emailed to the user using the SMTP settings defined in `backend/.env`.

## Pipelines
Pipelines define the stages executed when a target document is uploaded. Use the REST API to manage pipelines:

```
POST /api/pipelines
GET  /api/pipelines/{org_id}
```

Uploading with `pipeline_id` will automatically create an `AnalysisJob` record.
Only PDF files up to 200 MB are accepted. The server counts pages using a PDF parser before storing the file.

Pipeline definitions use JSON. Each stage entry must include a `type` field:

```json
{
  "name": "Default",
  "org_id": "ORG_UUID",
  "stages": [
    { "type": "ocr", "command": "./scripts/ocr.sh" },
    { "type": "parse", "command": "./scripts/parse.sh" },
    { "type": "ai", "command": "./scripts/ai.sh" },
    { "type": "report", "command": "./scripts/report.sh" }
  ]
}
```
If no `command` is provided the built-in implementations are used:
`ocr` runs Tesseract on the uploaded PDF, `parse` creates a simple JSON
structure from the text, `ai` posts the JSON to `AI_API_URL` with the
`AI_API_KEY` header and stores the response, and `report` generates a PDF
summary uploaded back to S3.

The worker processes stages sequentially and updates the job status when done.

The frontend provides a Pipeline Editor allowing administrators to enter the stage
`type` and optional shell `command` for each step. Stages can be reordered via
arrow buttons and removed individually. The resulting JSON structure is sent to
`/api/pipelines` when saving.

### Documents
List and download documents via:

```
GET /api/documents/{org_id}
GET /api/download/{document_id}
```

The download endpoint returns a pre-signed S3 URL valid for one hour.

### Settings
Each organization has settings controlling monthly quotas and the accent color used by the frontend.

```
GET /api/settings/{org_id}
POST /api/settings
```

The update endpoint accepts JSON fields `org_id`, `monthly_upload_quota`, `monthly_analysis_quota` and `accent_color`.
Quota limits are checked during uploads and job creation.

### Dashboard
Retrieve remaining monthly quotas:

```
GET /api/dashboard/{org_id}
```

Retrieve monthly upload and analysis counts for the last six months:

```
GET /api/dashboard/{org_id}/usage
```
The frontend displays these metrics in a bar chart using Chart.js so admins can easily monitor trends.

### Audit Logs
View recent user actions for an organization:

```
GET /api/audit/{org_id}
```

## Analysis Jobs
Jobs are created when uploading a document with a `pipeline_id`. List jobs per organization:

```
GET /api/jobs/{org_id}
```

Subscribe to job status updates via SSE:

```
GET /api/jobs/{job_id}/events
```
The frontend opens an `EventSource` connection to this endpoint so job entries
update automatically.

### Worker
Run the background worker to process pending jobs:

```bash
cargo run --bin worker
```

The worker pulls job IDs from Redis (set via `REDIS_URL`). For each job it executes the configured pipeline stages sequentially and updates the status in PostgreSQL.

### Cleanup
Expired target documents can be removed from S3 and the database using the cleanup utility:

```bash
cargo run --bin cleanup
```

Run this periodically (e.g. via cron) to keep storage usage in check.

## VisionOS Glassmorphism UI
The frontend embraces a glass aesthetic inspired by Apple's VisionOS. Components
use translucent panels with backdrop blur, soft shadows and pastel accent colors.
The reusable `GlassCard` component exposes props for opacity, blur and depth. All
inputs apply the frosted style via the `glass-input` class and buttons reuse the
primary and secondary variants from `Button.svelte`.
Layouts rely on a responsive 12‑column grid and SF Pro Display typography. The
accent color defined in each organization's settings is applied to interactive
elements.

## Security
All endpoints enforce JWT authentication and CORS restrictions via the `FRONTEND_ORIGIN` environment variable. A simple in-memory rate limiter restricts each IP to 100 requests per minute. Audit logs capture user actions such as login, uploads and downloads.

## Secret Management
Use `scripts/generate_secrets.sh` to create a `backend/.env.prod` file with random credentials:
```bash
./scripts/generate_secrets.sh
```
Edit the file to add your database URL and other production endpoints, then load it on startup with `source backend/.env.prod`.

## Production Build
Compile the backend in release mode and build the frontend:

```bash

cargo build --release --manifest-path backend/Cargo.toml
npm run build --prefix frontend
```

Serve the contents of `frontend/dist` with any static web server and run the compiled `backend/target/release/backend` binary with the environment configured for your production services.

## Development Scripts
`scripts/setup_dev.sh` provides a small helper for local setup. It copies `backend/.env.example` if needed, runs migrations and prints commands to start the services.

Use `scripts/bootstrap_deps.sh` to pre-fetch Rust crates and NPM packages and generate lockfiles. This step requires network access on first run but allows repeatable offline builds afterwards.

## Continuous Integration
A basic GitHub Actions workflow is provided at `.github/workflows/ci.yml` which builds the frontend and runs `cargo check` on every push or pull request.

## Lines of Code
As of this commit the repository contains:
    - Backend: 1498 lines
    - Frontend: 523 lines
    - **Total: 2021 lines**
