## Continuous Integration
A GitHub Actions workflow is provided at `.github/workflows/ci.yml`. On every push or pull request, it performs the following checks and builds:

- **Backend (Rust):**
  - `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- --deny warnings`: Runs Clippy for thorough static analysis and treats all warnings as errors.
  - `cargo fmt --manifest-path backend/Cargo.toml --all -- --check`: Ensures code is formatted according to `rustfmt` and fails the build on mismatches.
  - (Implicitly, `cargo test` would also be part of a full CI suite, though not explicitly listed as modified here).
- **Frontend (Svelte/TypeScript):**
- `npm install --prefix frontend`: Installs frontend dependencies. Run this
    before executing `npm test --prefix frontend` locally so Vitest and other
    packages are available. This mirrors the CI workflow where `npm install`
    precedes the test step.
  - `npm run lint --prefix frontend`: Executes `svelte-check` (using the configuration in `frontend/tsconfig.json`) for type checking and other Svelte-specific diagnostics.
  - `npm test --prefix frontend`: Runs the frontend unit and component test suite using Vitest.
  - `npm run build --prefix frontend`: Compiles the Svelte application to ensure the build process is successful.

This CI pipeline helps maintain code quality and catch issues early in both the backend and frontend parts of the project.

