# Running Backend Tests

The integration tests load environment variables from `../.env.test`.
Create this file based on `.env.test.example` before running `cargo test`.

At a minimum set `DATABASE_URL_TEST` to point to a PostgreSQL database used only
for tests. Some tests also use `REDIS_URL` for Redis-based rate limiting. Other
values from `.env.example` may be included as needed.

Example setup:

```bash
cp ../.env.test.example ../.env.test
cargo test --manifest-path ../Cargo.toml
```

