# Usage

## Requirements
- Rust toolchain
- Node.js (npm)
- PostgreSQL and MinIO (or AWS S3)

## Setup
See [Setup](Setup.md) for dependency installation and how to launch the services locally.

## Migrations
Run migrations with:
```bash
cd backend
cargo run --bin migrate
```

## Testing
- `cargo test` – backend unit tests
- `npm run build:prod --prefix frontend` – ensure Svelte app compiles and DaisyUI themes are generated
- `npm install --prefix frontend` – install dev dependencies before running frontend tests
- `npm test --prefix frontend` – run frontend unit and component tests

## Dev vs Prod
During development use `cargo run` and `npm run dev`.
For production build the frontend and compile the backend in release mode as shown below.

## Health check
Ensure the server is running with:
```bash
curl http://localhost:8080/api/health
```
It should return `ok`.

## Worker
Run the background worker to process jobs:
```bash
cargo run --bin worker --features worker-bin
```

In production compile and run the worker binary in release mode. Set
`REDIS_URL` to the Redis instance used by the API. `WORKER_CONCURRENCY`
specifies how many jobs a single worker processes in parallel. Optionally use
`SHUTDOWN_AFTER_IDLE` to exit after a period of inactivity or
`PROCESS_ONE_JOB=1` for pre-stop hooks. Multiple worker processes can be started
if desired:
```bash
cargo build --release --bin worker --features worker-bin
REDIS_URL=redis://redis:6379/ ./target/release/worker
```
Running multiple worker processes or increasing `WORKER_CONCURRENCY` allows jobs
to be handled concurrently.

## Cleanup
Remove expired documents that have passed their `expires_at` timestamp.

Run once:
```bash
cargo run --bin cleanup
```

Set `CLEANUP_INTERVAL_MINUTES` to continuously run on a schedule:
```bash
CLEANUP_INTERVAL_MINUTES=60 cargo run --bin cleanup
```

### Cron
After compiling the cleanup binary in release mode, invoke it from cron using `scripts/cleanup_cron.sh`:
```
0 * * * * /path/to/project/scripts/cleanup_cron.sh
```

### Kubernetes CronJob
An example CronJob manifest is provided at `k8s/cleanup-cronjob.yaml`.
Apply it with:
```bash
kubectl apply -f k8s/cleanup-cronjob.yaml
```

### Example prompt_templates JSON
Add AI prompt templates to organization settings using the following structure:

```json
{
  "prompt_templates": [
    { "name": "summary", "text": "Summarize the document" },
    { "name": "qa", "text": "Answer questions about the document" }
  ]
}
```

## Production Build
Compile the backend and build the frontend:
```bash
cargo build --release --manifest-path backend/Cargo.toml
npm run build:prod --prefix frontend
```
Serve `frontend/dist` with a static server and run the compiled backend binary with the appropriate environment.

## Development Scripts
`scripts/setup_dev.sh` copies example env files, runs migrations and prints commands to start the services.
`scripts/bootstrap_deps.sh` pre-fetches crates and npm packages and generates lockfiles.

## API-Client
Generate TypeScript API types from the OpenAPI specification:
```bash
npm run gen:api --prefix frontend
```
The generated client is written to `frontend/src/lib/api/index.ts`.
