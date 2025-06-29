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
sqlx migrate run
```

## Testing
- `cargo test` – backend unit tests
- `npm run build --prefix frontend` – ensure Svelte app compiles
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

## Cleanup
Remove expired documents periodically:
```bash
cargo run --bin cleanup
```

## Production Build
Compile the backend and build the frontend:
```bash
cargo build --release --manifest-path backend/Cargo.toml
npm run build --prefix frontend
```
Serve `frontend/dist` with a static server and run the compiled backend binary with the appropriate environment.

## Development Scripts
`scripts/setup_dev.sh` copies example env files, runs migrations and prints commands to start the services.
`scripts/bootstrap_deps.sh` pre-fetches crates and npm packages and generates lockfiles.
