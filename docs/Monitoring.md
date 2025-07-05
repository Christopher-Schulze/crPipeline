# Monitoring

The backend exposes Prometheus metrics at `http://localhost:9100/metrics`. To visualize these metrics, run Grafana with a preconfigured dashboard. In addition to job and stage metrics, the exporter collects S3 error counts (`s3_errors_total`), job duration histograms (`job_duration_seconds`), OCR latency histograms (`ocr_duration_seconds`), failed AI/OCR calls (`ai_ocr_errors_total`), login failure counts (`login_failures_total`), and rate limit fallback events (`rate_limit_fallback_total`).

## docker-compose example

```yaml
version: '3.9'
services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - ./alert-rules.yml:/etc/prometheus/alert-rules.yml
    ports:
      - "9090:9090"

  grafana:
    image: grafana/grafana:9.5.2
    environment:
      GF_SECURITY_ADMIN_USER: admin
      GF_SECURITY_ADMIN_PASSWORD: admin
    volumes:
      - ./grafana/provisioning:/etc/grafana/provisioning
      - ./grafana/dashboards:/var/lib/grafana/dashboards
    ports:
      - "3000:3000"
```

Create `prometheus.yml` next to the compose file:

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'crPipeline'
    static_configs:
      - targets: ['backend:9100']
rule_files:
  - alert-rules.yml
```

Define alerts in `alert-rules.yml`:

```yaml
groups:
  - name: example
    rules:
      - alert: HighLoginFailures
        expr: increase(login_failures_total[5m]) > 5
        for: 1m
        labels:
          severity: warning
        annotations:
          description: Too many failed login attempts
      - alert: ManyS3Errors
        expr: increase(s3_errors_total[5m]) > 10
        for: 1m
        labels:
          severity: warning
        annotations:
          description: S3 errors detected
```

Place the following provisioning file at `grafana/provisioning/dashboards/dashboard.yaml`:

```yaml
apiVersion: 1
providers:
  - name: 'default'
    folder: ''
    type: file
    options:
      path: /var/lib/grafana/dashboards
```

Create the dashboard JSON at `grafana/dashboards/metrics.json`:

```json
{
  "title": "crPipeline Metrics",
  "schemaVersion": 37,
  "version": 1,
  "refresh": "5s",
  "panels": [
    {
      "type": "graph",
      "title": "Stage Duration",
      "targets": [{ "expr": "stage_duration_seconds", "legendFormat": "{{stage}}" }]
    },
    {
      "type": "graph",
      "title": "Job Duration",
      "targets": [{ "expr": "job_duration_seconds", "legendFormat": "{{status}}" }]
    },
    {
      "type": "graph",
      "title": "Jobs Processed",
      "targets": [{ "expr": "jobs_total", "legendFormat": "{{status}}" }]
    },
    {
      "type": "graph",
      "title": "S3 Errors",
      "targets": [{ "expr": "s3_errors_total", "legendFormat": "{{operation}}" }]
    },
    {
      "type": "graph",
      "title": "OCR Duration",
      "targets": [{ "expr": "ocr_duration_seconds", "legendFormat": "{{engine}}" }]
    },
    {
      "type": "graph",
      "title": "AI/OCR Failures",
      "targets": [{ "expr": "ai_ocr_errors_total", "legendFormat": "{{service}}" }]
    },
    {
      "type": "graph",
      "title": "Login Failures",
      "targets": [{ "expr": "login_failures_total", "legendFormat": "{{reason}}" }]
      },
    {
      "type": "graph",
      "title": "Rate Limit Fallbacks",
      "targets": [{ "expr": "rate_limit_fallback_total", "legendFormat": "" }]
      }
    ]
 }
```

Grafana loads the dashboard on startup. Navigate to `http://localhost:3000` to view the charts.
Kubernetes manifests for deploying Prometheus and Grafana are available under `k8s/`.

## Alerting

Grafana supports alert rules on any Prometheus query. To be notified when many login attempts fail, open the *Login Failures* panel and create an alert with `increase(login_failures_total[5m]) > 5`. Configure a notification channel such as email or Slack to receive alerts.
Similarly, monitor S3 problems with `increase(s3_errors_total[5m]) > 10` and detect failing jobs using `increase(jobs_total{status="failed"}[5m]) > 1`.
Detect long-running jobs with `histogram_quantile(0.9, rate(job_duration_seconds_sum[5m]) / rate(job_duration_seconds_count[5m])) > 30`.
