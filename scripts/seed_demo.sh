#!/bin/bash
set -e

# Seed demo data: admin user and example documents.
# Ensure running from project root
cd "$(dirname "$0")/.."

if [ ! -f backend/.env ]; then
  echo "backend/.env not found" >&2
  exit 1
fi

set -a
source backend/.env
set +a

: "${DATABASE_URL:?DATABASE_URL not set in backend/.env}"
S3_BUCKET=${S3_BUCKET:-uploads}
LOCAL_S3_DIR=${LOCAL_S3_DIR:-./local_s3}

# create demo admin user
cargo run --bin create_admin -- demo@example.com password

PDF_CONTENT="%PDF-1.5\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF"
mkdir -p "$LOCAL_S3_DIR/$S3_BUCKET"

echo -e "$PDF_CONTENT" > "$LOCAL_S3_DIR/$S3_BUCKET/sample1.pdf"
echo -e "$PDF_CONTENT" > "$LOCAL_S3_DIR/$S3_BUCKET/sample2.pdf"

if ! command -v psql >/dev/null; then
  echo "psql not found" >&2
  exit 1
fi

psql "$DATABASE_URL" <<SQL
INSERT INTO documents (id, org_id, owner_id, filename, pages, is_target, display_name)
VALUES
  (gen_random_uuid(), (SELECT id FROM organizations LIMIT 1),
   (SELECT id FROM users LIMIT 1), 'sample1.pdf', 1, TRUE, 'sample1.pdf'),
  (gen_random_uuid(), (SELECT id FROM organizations LIMIT 1),
   (SELECT id FROM users LIMIT 1), 'sample2.pdf', 1, FALSE, 'sample2.pdf');
SQL

echo "Demo data seeded."
