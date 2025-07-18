# Deployment

The project ships with a GitHub Actions workflow that can build Docker images for the backend and frontend. The `release` job in `.github/workflows/ci.yml` runs after all tests succeed.

## Release workflow
1. CI installs dependencies, lints and runs unit tests for the frontend.
2. The `backend-tests` matrix job executes `cargo test` on `ubuntu-latest`, `windows-latest` and `macos-latest`.
3. When every job succeeds, `release` builds container images using the provided Dockerfiles.
4. If repository secrets `REGISTRY`, `REGISTRY_USERNAME` and `REGISTRY_PASSWORD` are set, the job logs in and pushes the images to that registry.

The resulting images are tagged as `backend:$IMAGE_TAG` and `frontend:$IMAGE_TAG`.
By default `IMAGE_TAG` is set to `latest`. When starting the workflow manually
you can pass a Git tag or any other value as the `IMAGE_TAG` input to publish
versioned images.

## Kubernetes Manifests

Example manifests are provided in the `k8s/` directory.
Apply them with:

```bash
kubectl apply -f k8s/backend-deployment.yaml
kubectl apply -f k8s/frontend-deployment.yaml
```

Secrets referenced by the deployments should contain the required environment variables.
For example:

```bash
kubectl create secret generic backend-env \
  --from-literal=DATABASE_URL=postgres://user:pass@db/production \
  --from-literal=JWT_SECRET=$(openssl rand -hex 32) \
  --from-literal=S3_BUCKET=uploads
kubectl create secret generic frontend-env \
  --from-literal=VITE_PUBLIC_BACKEND_URL=https://example.com
```

Equivalent manifest files `k8s/backend-secret.yaml` and
`k8s/frontend-secret.yaml` are provided for declarative deployments:

```bash
kubectl apply -f k8s/backend-secret.yaml
kubectl apply -f k8s/frontend-secret.yaml
```

Set the `REGISTRY` secret in your CI settings so images are pushed to your container registry.

## TLS / Ingress

The backend itself does not terminate TLS. In production deploy it behind a reverse proxy or Kubernetes ingress that provides HTTPS. Configure `BASE_URL` and `FRONTEND_ORIGIN` with `https://` URLs so that login cookies receive the `Secure` flag and browsers enforce encrypted connections. Any ingress controller such as Nginx or Traefik can handle certificates and forward traffic to the pod on port `8080`.

### Example Nginx ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: backend-ingress
  annotations:
    nginx.ingress.kubernetes.io/force-ssl-redirect: "true"
spec:
  tls:
  - hosts:
    - example.com
    secretName: tls-secret
  rules:
  - host: example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: backend
            port:
              number: 80
```

This manifest is also available as `k8s/backend-ingress.yaml`.

### Example Traefik ingress

```yaml
apiVersion: traefik.io/v1alpha1
kind: IngressRoute
metadata:
  name: backend
spec:
  entryPoints:
    - websecure
  routes:
  - match: Host(`example.com`)
    kind: Rule
    services:
    - name: backend
      port: 80
  tls:
    secretName: tls-secret
```

### Certificate management

Use a tool like [cert-manager](https://cert-manager.io/) or your ingress controller's ACME integration to automatically request Let's Encrypt certificates. Ensure the secret referenced by `tls-secret` exists and is kept up to date. When running the included Nginx or Traefik examples outside Kubernetes, copy your certificate files into `nginx/certs` or configure Traefik's ACME support accordingly.

## Versioned images with Helm

An example `values.yaml` could look like (also provided as `k8s/values-example.yaml`):

```yaml
backendImage:
  repository: myorg/backend
  tag: v1.0.0
frontendImage:
  repository: myorg/frontend
  tag: v1.0.0
```

These values are referenced in the deployment manifest as:

```yaml
image: "{{ .Values.backendImage.repository }}:{{ .Values.backendImage.tag }}"
```

Kustomize users can achieve the same by applying an `image` patch that sets the
tag for each deployment.

Refer to [Setup](Setup.md) for local development. For metrics and dashboards see [Monitoring](Monitoring.md).

## Systemd

For bare-metal deployments the worker can run as a systemd service. The unit file
`deploy/worker.service` and the helper script `scripts/install_worker_service.sh`
install the service under `/etc/systemd/system`. Step-by-step instructions and
customization hints are provided in
[`Todo-fuer-User.md`](Todo-fuer-User.md). The service expects the same
environment variables that are used by the Docker Compose setup and Kubernetes
manifests.

## Produktivbetrieb

### Docker-Images bauen
Die Images werden normalerweise durch den CI-Workflow erstellt. Bei Bedarf können sie auch lokal gebaut werden:
```bash
docker build -t myorg/backend:latest backend
docker build -t myorg/frontend:latest frontend
```
The frontend Dockerfile runs `npm run build:prod` so DaisyUI themes are included
in the production build. If you build the frontend without Docker, run this
command manually before copying `frontend/dist`.

### Kubernetes-Manifeste nutzen
Alle notwendigen Ressourcen befinden sich im Verzeichnis `k8s/`. Mit
`kubectl apply -f k8s/` werden Deployments, Services und Ingress-Regeln im Cluster angelegt.

### TLS/Ingress und Secrets
Die Applikation erwartet TLS-Beendigung durch einen Ingress oder Proxy. Legen Sie die benötigten Secrets vor dem Deployment an und verweisen Sie im Ingress auf ein gültiges Zertifikat.

### Lokale Tests
Zum Testen ohne Kubernetes steht die Datei [`docker-compose.yml`](../docker-compose.yml) bereit.
