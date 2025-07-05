#!/bin/bash
set -e

# Generate a .env file with random secrets suitable for production.
# Usage: ./generate_secrets.sh [OUTPUT] [--k8s]
# When --k8s is supplied the script also prints a Kubernetes Secret manifest
# to stdout. Redirect the output to a file or pipe it directly to kubectl.

OUTPUT="backend/.env.prod"
K8S=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --k8s)
      K8S=1
      shift
      ;;
    *)
      OUTPUT="$1"
      shift
      ;;
  esac
done

JWT_SECRET=$(openssl rand -hex 32)
AWS_ACCESS_KEY=${AWS_ACCESS_KEY:-$(openssl rand -hex 12)}
AWS_SECRET_KEY=${AWS_SECRET_KEY:-$(openssl rand -hex 32)}
REDIS_URL=${REDIS_URL:-redis://localhost/}

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
EOT

echo "Generated $OUTPUT with random credentials. Fill DATABASE_URL and other settings as needed."

if [[ $K8S -eq 1 ]]; then
cat <<EOF
apiVersion: v1
kind: Secret
metadata:
  name: backend-env
type: Opaque
data:
  DATABASE_URL: $(printf "" | base64 -w0)
  JWT_SECRET: $(printf "%s" "$JWT_SECRET" | base64 -w0)
  AWS_ENDPOINT: $(printf "" | base64 -w0)
  AWS_ACCESS_KEY: $(printf "%s" "$AWS_ACCESS_KEY" | base64 -w0)
  AWS_SECRET_KEY: $(printf "%s" "$AWS_SECRET_KEY" | base64 -w0)
  S3_BUCKET: $(printf "uploads" | base64 -w0)
  FRONTEND_ORIGIN: $(printf "" | base64 -w0)
  REDIS_URL: $(printf "%s" "$REDIS_URL" | base64 -w0)
  AI_API_URL: $(printf "" | base64 -w0)
  AI_API_KEY: $(printf "" | base64 -w0)
EOF
fi
