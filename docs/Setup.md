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

Environment variables can be tweaked in `backend/.env` to point to a different database or S3 endpoint. Ensure the bucket defined in `S3_BUCKET` exists in your MinIO or AWS account.

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

