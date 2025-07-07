## Security
All endpoints enforce JWT authentication and CORS restrictions via the `FRONTEND_ORIGIN` environment variable. The rate limiter normally stores counters in Redis. If Redis is unavailable, the behavior is controlled by `REDIS_RATE_LIMIT_FALLBACK` which defaults to an in-memory limit of 100 requests per minute. Setting this variable to `deny` will reject requests outright on Redis failures. Audit logs capture user actions such as login, uploads and downloads.

### Secrets

For production deployments generate unique credentials instead of relying on the example values in `backend/.env`. Run `scripts/generate_secrets.sh` to create `backend/.env.prod` with random keys. Use `--k8s --apply` to immediately populate the `backend-env` secret. Review the generated file and provide your own database and service endpoints before starting the application.

### Recommended production values

- **`JWT_SECRET`** – generate a 256‑bit value for signing tokens. Example:
  ```bash
  openssl rand -hex 32 # => e1c0c1d2edb1af378b90d593ca110ec3d3f211ba6f9e9ff5ca0a8f97b2f53811
  ```
  Use the same secret on all backend instances so issued JWTs remain valid.
- **`CSRF_TOKEN`** – set this to a high‑entropy random string such as the output of `openssl rand -hex 32`. When present the backend rejects requests whose `X-CSRF-Token` header does not match this value.
- **Secure cookies** – the login cookie is flagged `HttpOnly` and `SameSite=Lax` by default. The `Secure` flag is automatically enabled when `BASE_URL` starts with `https://`. Ensure `BASE_URL` and `FRONTEND_ORIGIN` use HTTPS in production so cookies are transmitted only over TLS.
- **HTTPS only** – terminate TLS in a reverse proxy or load balancer and always access the API via `https://`. This protects JWTs and session cookies in transit.
- **Rate limiting** – provide a production Redis instance via `REDIS_URL`. Set `REDIS_RATE_LIMIT_FALLBACK=deny` so that API requests are rejected if Redis becomes unavailable instead of falling back to the in-memory limiter.

### Updating API keys

The settings endpoints mask sensitive keys. When retrieving settings the fields `ai_api_key` and `ocr_api_key` appear as `********`. To keep an existing key send the masked value back unchanged. Provide a new value to rotate the key or send an empty string to clear it. See `backend/src/handlers/settings.rs` for the implementation.
