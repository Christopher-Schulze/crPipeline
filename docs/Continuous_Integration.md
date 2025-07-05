## Continuous Integration
A GitHub Actions workflow is provided at `.github/workflows/ci.yml`. On every push or pull request, it performs the following checks and builds:

- **Backend (Rust):**
  - `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- --deny warnings`: Runs Clippy for thorough static analysis and treats all warnings as errors.
  - `cargo fmt --manifest-path backend/Cargo.toml --all -- --check`: Ensures code is formatted according to `rustfmt` and fails the build on mismatches.
  - (Implicitly, `cargo test` would also be part of a full CI suite, though not explicitly listed as modified here).
- **Frontend (Svelte/TypeScript):**
  - `npm install --prefix frontend`: Installs the frontend dev dependencies. **Run this before `npm test --prefix frontend`** so Vitest and other packages are available. Without installing dependencies first, the test runner may fail to launch.
  - `npm run lint --prefix frontend`: Executes `svelte-check` (using the configuration in `frontend/tsconfig.json`) for type checking and other Svelte-specific diagnostics.
  - `npm test --prefix frontend`: Runs the frontend unit and component test suite using Vitest. Ensure `npm install --prefix frontend` has been run first so all dev dependencies are available.
  - `npm run build --prefix frontend`: Compiles the Svelte application to ensure the build process is successful.

This CI pipeline helps maintain code quality and catch issues early in both the backend and frontend parts of the project.

### Zero Warnings
During development, unused imports may generate warnings when running `cargo test`.
After removing these warnings (for example deleting an unused `Value` import in
`backend/src/processing.rs` and unused imports in `backend/tests`), verify the
backend builds cleanly with:
```bash
cargo check --all-targets
```
This should produce no warnings. The build was also confirmed with
`cargo test --no-run` and `cargo check --tests --all-targets`.

Integration tests cover common authentication flows. New tests verify that
login succeeds for confirmed users and fails for unconfirmed accounts. Start the
database services via `docker compose up -d db redis minio` and run `cargo test`
to execute the full suite.

