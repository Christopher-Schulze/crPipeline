# Architecture

## Authentication
Use the following endpoints to register and log in. Successful login sets a `token` cookie automatically sent with future requests.
```text
POST /api/register
POST /api/login
GET  /api/confirm/{token}
POST /api/request_reset
POST /api/reset_password
GET  /api/me
```

## Organizations
Admins manage organizations via:
```text
POST /api/orgs
GET  /api/orgs
```

### User Roles
- **admin** – global administrator with full access.
- **org_admin** – manages users, documents and settings for their organization.
- **user** – regular member restricted to their organization.

## Pipelines
Pipelines define the stages executed when a document is uploaded.
```text
POST /api/pipelines
GET  /api/pipelines/{org_id}
PUT    /api/pipelines/{id}
DELETE /api/pipelines/{id}
```
Stages can be customised with a `command` or additional fields as described below.

### Advanced Stage Configuration
- **AI stages** may specify `prompt_name` to use an organization prompt template.
- **OCR stages** support custom commands or an external engine via `ocr_engine`, `ocr_stage_endpoint` and `ocr_stage_key`.

### Documents
List and download documents:
```text
GET /api/documents/{org_id}
GET /api/download/{document_id}
```
The download endpoint streams the PDF when `LOCAL_S3_DIR` is configured and
otherwise returns a JSON object containing a presigned URL.

### Settings
Organizations store quotas, AI/OCR configuration and prompt templates.
```text
GET /api/settings/{org_id}
POST /api/settings
```

### Dashboard
Retrieve remaining quotas and usage history:
```text
GET /api/dashboard/{org_id}
GET /api/dashboard/{org_id}/usage
```

### Audit Logs
```text
GET /api/audit/{org_id}
```

## Analysis Jobs
List jobs and get details:
```text
GET /api/jobs/{org_id}
GET /api/jobs/events/{org_id}
GET /api/jobs/{job_id}/events
GET /api/jobs/{job_id}/details
```
The `/api/jobs/events/{org_id}` endpoint streams job status updates over SSE.
Workers publish to Redis and the frontend listens with `EventSource`,
falling back to polling `/api/jobs/{org_id}` if the stream is unavailable.

### Stage Output Downloads
```text
GET /api/jobs/outputs/{output_id}/download_url
```

### Admin Endpoints
Global admins can manage users and send invites via special endpoints.

## Frontend UI Highlights
- Dashboard, Document List and Jobs List with sortable tables.
- Pipeline Editor with drag-and-drop stages.
- Settings form for organization administrators.
- Admin Panel for global admins.
- Job detail modal showing stage outputs with copy and download options.

### Frontend Events
The Pipeline Editor dispatches a `pipelinesUpdated` event on `document.body` so pages can refresh pipeline lists.

## VisionOS Glassmorphism UI
The frontend uses a glass-like aesthetic with SF Pro Display typography and an organization accent color.
