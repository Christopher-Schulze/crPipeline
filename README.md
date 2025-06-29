# crPipeline

Multi-tenant document analysis platform built with Rust and Svelte.

All Svelte source files now live in `frontend/src`. An older top-level `src/`
folder containing experimental routes was removed.

## Table of Contents
- [Requirements](#requirements)
- [Setup](#setup)
  - [Docker Compose](#docker-compose)
- [Migrations](#migrations)
- [Testing](#testing)
- [Dev vs Prod](#dev-vs-prod)
- [Environment Variables](#environment-variables)
  - [Health check](#health-check)
- [Authentication](#authentication)
- [Organizations](#organizations)
  - [User Roles](#user-roles)
- [Pipelines](#pipelines)
  - [Advanced Stage Configuration](#advanced-stage-configuration)
  - [Documents](#documents)
  - [Settings](#settings)
  - [Dashboard](#dashboard)
  - [Audit Logs](#audit-logs)
- [Analysis Jobs](#analysis-jobs)
  - [Admin Endpoints (Require Global Admin Role)](#admin-endpoints-require-global-admin-role)
  - [Frontend UI Highlights](#frontend-ui-highlights)
  - [Worker](#worker)
  - [Cleanup](#cleanup)
- [VisionOS Glassmorphism UI](#visionos-glassmorphism-ui)
- [Security](#security)
- [Secret Management](#secret-management)
- [Production Build](#production-build)
- [Development Scripts](#development-scripts)
- [Continuous Integration](#continuous-integration)
- [Lines of Code](#lines-of-code)
- [License](#license)

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
- `npm install --prefix frontend` – install dev dependencies before running frontend tests
- `npm test --prefix frontend` – run frontend unit and component tests

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
OCR_API_ENDPOINT=
OCR_API_KEY=
```

`BASE_URL` is used when generating confirmation and reset links.
`AWS_ENDPOINT` should be set to your MinIO or AWS S3 endpoint for development. In production, this variable can be removed to use AWS defaults if IAM roles are configured.
`AI_API_URL` and `AI_API_KEY` serve as global defaults for the AI service if not overridden by more specific Organization Settings.
`OCR_API_ENDPOINT` and `OCR_API_KEY` act as global defaults for an external OCR service. If these are not set, and no organization or stage-specific OCR settings are provided, the system falls back to local Tesseract OCR for "ocr" stages not using a custom command.

Organization-specific settings for AI (including custom headers) and OCR can override these global defaults. Pipeline stage definitions can further override OCR settings for specific "ocr" stages. See 'Settings' and 'Pipelines' sections for more details.

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

Public registration via `POST /api/register` creates a new user.
- It requires `org_id` (the organization the user will belong to), `email`, and `password`.
- An optional `role` field can be provided. If omitted, it defaults to `"user"`.
  Assigning `"admin"` or `"org_admin"` roles is typically handled by a global administrator using dedicated tools or future admin panel functionalities, not through public registration.
- A confirmation email is sent to the user's email address. The user must click the link in the email to confirm their account before they can log in.

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
The Admin Panel in the frontend is accessible to users with the global "admin" role. It allows them to:
- List and create organizations.
- List all users in the system and manage their roles (see User Roles section).

Email confirmation and password reset links are sent to users using the SMTP settings defined in `backend/.env`.

### User Roles
The system defines the following user roles, stored in the `users` table:
- **`admin`**: Global Administrator. Has full system access, including managing all organizations, all users (assigning roles like "org_admin" or "user"), and viewing all data. The initial global admin is created via the `create_admin` script.
- **`org_admin`**: Organization Administrator. Manages users (within their assigned organization, excluding changing roles of global admins or their own), documents, pipelines, and settings specifically for *their assigned organization*. They cannot see or manage other organizations.
- **`user`**: Standard User. Can upload documents to their assigned organization, run analyses using pipelines available to their organization, and view their own documents and job results. Access is restricted to their assigned organization.

Global administrators can assign the "org_admin" or "user" roles to users and associate "org_admin"s with a specific organization via the "User Management" tab in the Admin Panel.

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
If no `command` is provided for a stage (and for "ocr" stages, if no overriding engine configuration is set), default processing logic is applied by the worker:
- `ocr`: Default is local Tesseract. Behavior can be modified by `command` or by `ocr_engine` settings (see Advanced Stage Configuration).
- `parse`: Creates a simple JSON structure from the text generated by a preceding OCR stage.
- `ai`: Posts the input JSON (which could be from a "parse" stage, or a formatted prompt using Organization-level templates) to the configured AI service. Behavior can be modified by `prompt_name` (see Advanced Stage Configuration) and by Organization Settings for AI (endpoint, key, custom headers).
- `report`: Generates a PDF summary from the JSON output of a preceding stage.

The worker processes stages sequentially. For key stages like "ocr", "parse", and "ai", it saves both the input to the stage and its output as distinct files in S3, recording metadata in the `job_stage_outputs` table. This aids in debugging and analysis. The final job status is updated upon completion or failure.

The frontend provides a Pipeline Editor (accessible via a slide-over panel). Administrators can define stages, specifying `type`, an optional `command` (which can be a shell command or specific configuration like a prompt name for "ai" stages), and other stage-specific settings. Stages can be reordered using drag-and-drop and removed individually. The resulting JSON structure is sent to `/api/pipelines` when saving.

### Advanced Stage Configuration

Pipeline stages can be further customized with additional fields in their JSON definition:

**AI Stages (`type: "ai"`)**
- `prompt_name`: (Optional) A string specifying the `name` of a pre-defined Prompt Template (managed in Organization Settings). If provided and found, the worker will use this template to format the input for the AI service, replacing `{{json_input}}` or `{{content}}` placeholders with the output of the previous stage. If not provided, or if the template is not found, the AI stage uses its default behavior (e.g., sending the previous stage's JSON output directly or using a generic internal prompt).
  Example: `{"type": "ai", "prompt_name": "MyCustomSummarizer"}`

**OCR Stages (`type: "ocr"`)**
The OCR process follows a priority:
1.  Custom `command` (if provided and non-empty, it's executed).
2.  Stage-specific `ocr_engine: "external"` with `ocr_stage_endpoint` (if provided).
3.  Organization-level external OCR settings (if `stage.ocr_engine` is not set or is `"default"` and no custom `command` is given, but OrgSettings has external OCR configured).
4.  Global `OCR_API_ENDPOINT`/`KEY` environment variables (if OrgSettings do not specify an external OCR).
5.  Default local Tesseract (if no external OCR is configured at any level and no custom `command`).

- `command`: (Optional) A shell command to execute for OCR. If provided and non-empty, this takes precedence over `ocr_engine` settings. Placeholders `{{input_pdf}}` and `{{output_txt}}` will be replaced by the worker with appropriate temporary file paths.
- `ocr_engine`: (Optional) Can be `"default"` or `"external"`.
  - If `"default"`, it implies using local Tesseract or a custom `command` if provided, or falling back to organization/global external OCR if those are set and no custom command is present.
  - If `"external"`, the stage should also provide `ocr_stage_endpoint`.
- `ocr_stage_endpoint`: (Required if `ocr_engine` is `"external"`) The API endpoint for a stage-specific external OCR service. This overrides any organization-level or global default external OCR endpoint.
- `ocr_stage_key`: (Optional if `ocr_engine` is `"external"`) The API key for the stage-specific external OCR service. Overrides any organization-level or global default external OCR key.

  Example for stage-specific external OCR:
  ```json
  {
    "type": "ocr",
    "ocr_engine": "external",
    "ocr_stage_endpoint": "https://my.special.ocr.api/process",
    "ocr_stage_key": "stage_specific_key_if_needed"
  }
  ```
  Example for using a custom OCR command:
  ```json
  {
    "type": "ocr",
    "command": "my_ocr_tool --input {{input_pdf}} --output {{output_txt}} --lang eng"
  }
  ```

### Documents
List and download documents via:

```
GET /api/documents/{org_id}
GET /api/download/{document_id}
```

The download endpoint returns a pre-signed S3 URL valid for one hour.

### Settings
Each organization has settings controlling monthly quotas, AI/OCR configurations, prompt templates, and the accent color used by the frontend. These are managed via the "Settings" panel in the UI (accessible to Organization Admins and Global Admins for the respective organization).

```
GET /api/settings/{org_id}
POST /api/settings
```

The `POST /api/settings` endpoint accepts a JSON payload representing the complete `OrgSettings` structure for an organization. Key configurable fields include:

- `monthly_upload_quota`, `monthly_analysis_quota`: Numeric values for monthly operational limits.
- `accent_color`: A hexadecimal color string (e.g., `#30D5C8`) used for UI theming within that organization's context.
- **AI Configuration:**
  - `ai_api_endpoint`: (Optional) Organization-specific endpoint for the AI service (e.g., OpenAI, OpenRouter). If provided, this overrides the global `AI_API_URL` environment variable for this organization.
  - `ai_api_key`: (Optional) Organization-specific API key for the AI service. Overrides `AI_API_KEY`.
    - *Security Note:* When settings are retrieved via `GET /api/settings/{org_id}`, any set API key is masked (returned as "********"). To update a key, type the new key into the field. To clear a key, delete all text from the field and save (the frontend should send `null` or an empty string). If the form is saved while "********" is visible and unchanged by the user, the backend preserves the original stored key.
  - `ai_custom_headers`: (Optional) An array of custom HTTP header objects, e.g., `[{"name": "Header-Name", "value": "Header-Value"}]`. These headers are sent with requests to the AI service. This is useful for services like OpenRouter.ai that may require specific headers like `HTTP-Referer` or `X-Title`.
- **Prompt Templates:**
  - `prompt_templates`: (Optional) A list of named prompt templates. Each template is an object, e.g., `{"name": "TemplateName", "text": "Prompt text with {{json_input}} or {{content}} placeholders."}`. These templates can be selected by name in AI pipeline stages to customize AI behavior for this organization.
- **OCR Configuration (Organization-Level Defaults):**
  - `ocr_api_endpoint`: (Optional) Organization-specific endpoint for an external OCR service. This serves as a default for "ocr" stages within this organization if they are set to use an "external" engine but don't specify their own endpoint.
  - `ocr_api_key`: (Optional) Organization-specific API key for the external OCR service. (Masked and updated like `ai_api_key`).
  - If these organization-level OCR settings are not provided or are empty, and a pipeline "ocr" stage does not specify its own external configuration or a custom command, the system falls back to the global `OCR_API_ENDPOINT`/`OCR_API_KEY` environment variables, and finally to local Tesseract OCR.

Quota limits are checked during document uploads (for target documents) and before queuing analysis jobs.

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
GET /api/jobs/{job_id}/details
```
The `/api/jobs/{job_id}/details` endpoint (GET) retrieves comprehensive information for a specific job, including its core details, associated document name, pipeline name, and a list of all its generated stage outputs (each with `id`, `stage_name`, `output_type`, S3 location, and `created_at`). This endpoint requires the user to be part of the job's organization or be a global admin.

#### Stage Output Downloads
Individual stage output files (like intermediate JSON, extracted text, or final reports) can be downloaded using pre-signed S3 URLs. First, obtain the `output_id` from the `stage_outputs` array in the job details response.
```
GET /api/jobs/outputs/{output_id}/download_url
```
This returns a JSON object `{"url": "pre-signed-s3-url"}`. The URL is typically valid for a limited time (e.g., 1 hour). Access is authorized based on the parent job's organization.

### Admin Endpoints (Require Global Admin Role)

- **`GET /api/admin/users`**: Lists all users in the system. Each user object includes `id`, `email`, `role`, `org_id`, `organization_name` (if linked), and `confirmed` status.
- **`POST /api/admin/users/{user_id}/assign_role`**: Assigns a role to a specific user and can associate them with an organization.
  - **Path Parameter:** `user_id` - The UUID of the user to modify.
  - **JSON Payload:**
    ```json
    {
      "role": "user" | "org_admin",
      "org_id": "organization_uuid" // Required if role is "org_admin", otherwise ignored or can be used to reassign a 'user'
    }
    ```
  - Only "user" or "org_admin" can be assigned via this endpoint. Global admins cannot change their own role or demote the last global admin.

### Frontend UI Highlights

The frontend provides several key interfaces:

- **Dashboard:** Shows quota usage and recent activity.
- **Document List:** Displays uploaded documents with filtering (source/target) and download options. Uses a sortable, paginated data table.
- **Jobs List:** Shows analysis jobs with their status (updates via SSE). Each job can be clicked to view details. Uses a sortable data table.
- **Pipeline Editor:** (Slide-over panel) Allows creation and modification of multi-stage analysis pipelines, including configuring OCR engine, AI prompt templates, and custom commands per stage. Supports drag-and-drop reordering.
- **Settings Form:** (Slide-over panel) Allows organization administrators to manage quotas, accent color, AI/OCR API configurations (including custom AI headers and prompt templates).
- **Admin Panel:** (For global admins) Tabbed interface for managing organizations and users (view list, assign roles).
- **Analysis Job Detail View:** (Modal overlay)
    - Displays comprehensive details of a selected job.
    - **Embedded Viewers:** Directly shows OCR text, parsed JSON, and AI output JSON within the view.
    - **"Copy to Clipboard"** for embedded viewers.
    - **AI Input/Output Side-by-Side:** A modal view to compare the JSON input sent to an AI stage and the JSON output it produced.
    - Lists all generated stage outputs with individual "Download" buttons (using pre-signed URLs) and "View (Modal)" buttons for text/JSON files.

#### Frontend Events
The Pipeline Editor dispatches a global `pipelinesUpdated` event on `document.body` after a successful save. Pages that list pipelines can listen for this event to refresh automatically:

```ts
document.body.addEventListener('pipelinesUpdated', () => {
  // reload pipelines list
});
```

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
A GitHub Actions workflow is provided at `.github/workflows/ci.yml`. On every push or pull request, it performs the following checks and builds:

- **Backend (Rust):**
  - `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- --deny warnings`: Runs Clippy for thorough static analysis and treats all warnings as errors.
  - (Implicitly, `cargo test` would also be part of a full CI suite, though not explicitly listed as modified here).
- **Frontend (Svelte/TypeScript):**
  - `npm install --prefix frontend`: Installs frontend dependencies.
  - `npm run lint --prefix frontend`: Executes `svelte-check` (using the configuration in `frontend/tsconfig.json`) for type checking and other Svelte-specific diagnostics.
  - `npm test --prefix frontend`: Runs the frontend unit and component test suite using Vitest.
  - `npm run build --prefix frontend`: Compiles the Svelte application to ensure the build process is successful.

This CI pipeline helps maintain code quality and catch issues early in both the backend and frontend parts of the project.

## Lines of Code
As of this commit the repository contains:
    - Backend: 1498 lines
    - Frontend: 523 lines
    - **Total: 2021 lines**

See [PLAN.md](PLAN.md) for the development roadmap and outstanding tasks.

## License

This project is licensed under the [MIT License](LICENSE).

