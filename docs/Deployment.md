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
  --from-literal=PUBLIC_BACKEND_URL=https://example.com
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

Use a tool like [cert-manager](https://cert-manager.io/) or your ingress controller's ACME integration to automatically request Let's Encrypt certificates. Ensure the secret referenced by `tls-secret` exists and is kept up to date.

## Versioned images with Helm

An example `values.yaml` could look like:

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
