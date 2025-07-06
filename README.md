# crPipeline

Multi-tenant document analysis platform built with Rust and Svelte.

## Documentation
- [Usage](docs/Usage.md)
- [Architecture](docs/Architecture.md)
- [Environment](docs/Environment.md)
- [Setup](docs/Setup.md)
- [Worker als Dienst](docs/Setup.md#worker-als-dienst)
- [Security](docs/Security.md)
- [Continuous Integration](docs/Continuous_Integration.md)
- [Deployment](docs/Deployment.md)
- [Monitoring](docs/Monitoring.md)
- [Changelog](docs/Changelog.md)
- [API Examples](docs/API_Examples.md)
- [API Reference](docs/api.html)

- [AI Prompt Examples](docs/Usage.md#example-prompt_templates-json)

See [PLAN.md](PLAN.md) for the project roadmap.

## CI/CD secret generation

Automated pipelines can run `scripts/generate_secrets.sh` to create the `.env` file used by the backend. Pass `--k8s` to also emit a Kubernetes Secret manifest which can be applied directly:

```bash
scripts/generate_secrets.sh backend/.env.prod --k8s > k8s/backend-secret.yaml
kubectl apply -f k8s/backend-secret.yaml
```

## License

This project is licensed under the [MIT License](LICENSE).
