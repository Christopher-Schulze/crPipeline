#!/bin/bash
set -e

# Generate a .env file with random secrets suitable for production
OUTPUT=${1:-backend/.env.prod}

JWT_SECRET=$(openssl rand -hex 32)
AWS_ACCESS_KEY=${AWS_ACCESS_KEY:-$(openssl rand -hex 12)}
AWS_SECRET_KEY=${AWS_SECRET_KEY:-$(openssl rand -hex 32)}
REDIS_URL=${REDIS_URL:-redis://localhost/}
CSRF_SECRET_KEY_B64=$(openssl rand -base64 32)

cat > "$OUTPUT" <<EOT
DATABASE_URL=
JWT_SECRET=$JWT_SECRET
AWS_ENDPOINT=
AWS_ACCESS_KEY=$AWS_ACCESS_KEY
AWS_SECRET_KEY=$AWS_SECRET_KEY
S3_BUCKET=uploads
FRONTEND_ORIGIN=
REDIS_URL=$REDIS_URL
AI_API_URL=
AI_API_KEY=
CSRF_SECRET_KEY_B64=$CSRF_SECRET_KEY_B64
EOT

echo "Generated $OUTPUT with random credentials. Fill DATABASE_URL and other settings as needed."
