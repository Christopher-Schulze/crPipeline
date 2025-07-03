# Deployment

The project ships with a GitHub Actions workflow that can build Docker images for the backend and frontend. The `release` job in `.github/workflows/ci.yml` runs after all tests succeed.

## Release workflow
1. CI installs dependencies, lints and runs unit tests for the frontend.
2. The `backend-tests` matrix job executes `cargo test` on `ubuntu-latest`, `windows-latest` and `macos-latest`.
3. When every job succeeds, `release` builds container images using the provided Dockerfiles.
4. If repository secrets `REGISTRY`, `REGISTRY_USERNAME` and `REGISTRY_PASSWORD` are set, the job logs in and pushes the images to that registry.

The resulting images are tagged as `backend:latest` and `frontend:latest`. Use a tag strategy that fits your deployment pipeline or add additional steps to push versioned tags.

## Kubernetes Manifests

Example manifests are provided in the `k8s/` directory.
Apply them with:

```bash
kubectl apply -f k8s/backend-deployment.yaml
kubectl apply -f k8s/frontend-deployment.yaml
```

Secrets referenced by the deployments should contain the required environment variables.
