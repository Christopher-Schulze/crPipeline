# Environment

## Environment Variables
The file `backend/.env` defines required settings:
```
DATABASE_URL=postgres://postgres:postgres@localhost/db
JWT_SECRET=changeme
# Must be at least 32 characters in production
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
OCR_API_ENDPOINT=
OCR_API_KEY=
#PROCESS_ONE_JOB=1
#LOCAL_S3_DIR=/tmp/s3
```

`BASE_URL` is used when generating confirmation and reset links. `AWS_ENDPOINT` should point to your S3 or MinIO server in development. `AI_API_URL` and `AI_API_KEY` provide global defaults for the AI service. `OCR_API_ENDPOINT` and `OCR_API_KEY` configure an optional external OCR service. Organization and pipeline settings may override these values.

`PROCESS_ONE_JOB` causes the worker to exit after a single job. `LOCAL_S3_DIR` lets the worker store files on disk instead of S3 during local tests.

`METRICS_PORT` controls the port of the worker metrics HTTP endpoint. When set,
the worker exposes Prometheus metrics at `http://0.0.0.0:$METRICS_PORT/metrics`.
The backend API always serves metrics at `/metrics` on its regular port.

### Secret Management
Run `scripts/generate_secrets.sh` to create `backend/.env.prod` with random credentials. Update it with your production endpoints and load it on startup with `source backend/.env.prod`.

### Frontend Variables
Frontend builds can consume environment variables prefixed with `VITE_`.
Only `VITE_PUBLIC_BACKEND_URL` is used currently to point the Svelte app at the backend API:

```bash
VITE_PUBLIC_BACKEND_URL=http://localhost:8080
```
Provide these variables when running `npm run build` or via Kubernetes secrets referenced by `frontend-env`.
