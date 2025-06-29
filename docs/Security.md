## Security
All endpoints enforce JWT authentication and CORS restrictions via the `FRONTEND_ORIGIN` environment variable. The rate limiter normally stores counters in Redis. If Redis is unavailable, the behavior is controlled by `REDIS_RATE_LIMIT_FALLBACK` which defaults to an in-memory limit of 100 requests per minute. Setting this variable to `deny` will reject requests outright on Redis failures. Audit logs capture user actions such as login, uploads and downloads.

