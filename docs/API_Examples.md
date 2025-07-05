# API Examples

This document shows typical requests and responses for key endpoints.

## Login

### Request
```http
POST /api/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "secret"
}
```

### Success Response
Status: `200 OK`
```json
{"success": true}
```
The server also sets a `token` cookie used for authentication.

### Error Responses
- `401 Unauthorized` – invalid credentials or account not confirmed.
- `500 Internal Server Error` – unexpected error during login.

## Document Upload

### Request
```http
POST /api/upload?org_id=<org_uuid>&pipeline_id=<pipeline_uuid>&is_target=true
Content-Type: multipart/form-data

file: <binary PDF file>
```

### Success Response
Status: `200 OK`
```json
{
  "id": "<document_uuid>",
  "org_id": "<org_uuid>",
  "owner_id": "<owner_uuid>",
  "filename": "<s3-key>",
  "display_name": "report.pdf",
  "pages": 5,
  "is_target": true,
  "expires_at": null
}
```

### Error Responses
- `400 Bad Request` – invalid file or parameters.
- `401 Unauthorized` – uploading to another organization without admin rights.
- `429 Too Many Requests` – monthly quota exceeded.
- `500 Internal Server Error` – failure while uploading or saving metadata.

## Document Download

### Request
```http
GET /api/download/<document_uuid>
Authorization: Bearer <token>
```

### Success Response
If `LOCAL_S3_DIR` is set, the PDF bytes are streamed directly with status `200 OK`.
Otherwise the response contains a JSON object with a presigned URL:
```json
{"url": "https://s3.example.com/..."}
```

## Pipeline Editor

### Create Pipeline
```http
POST /api/pipelines
Content-Type: application/json

{
  "org_id": "<org_uuid>",
  "name": "My Pipeline",
  "stages": [
    { "id": "s1", "type": "ocr" },
    { "id": "s2", "type": "parse" }
  ]
}
```
Success: `200 OK` with the created pipeline JSON.

### Update Pipeline
```http
PUT /api/pipelines/<pipeline_id>
Content-Type: application/json

{
  "org_id": "<org_uuid>",
  "name": "My Pipeline",
  "stages": [
    { "id": "s1", "type": "ocr" },
    { "id": "s2", "type": "parse" }
  ]
}
```
Success: `200 OK` with the updated pipeline JSON.

### Error Responses
- `400 Bad Request` – invalid name or stages.
- `401 Unauthorized` – modifying a pipeline from another organization.
- `404 Not Found` – pipeline does not exist.
- `500 Internal Server Error` – failure while saving changes.

