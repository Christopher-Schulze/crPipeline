# Monitoring

The backend exposes Prometheus metrics at `http://localhost:9100/metrics`. To visualize these metrics, run Grafana with a preconfigured dashboard. In addition to job and stage metrics, the exporter collects S3 error counts (`s3_errors_total`), OCR latency histograms (`ocr_duration_seconds`), and login failure counts (`login_failures_total`).

## docker-compose example

```yaml
version: '3.9'
services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
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
  "panels": [
    {
      "type": "graph",
      "title": "Stage Duration",
      "targets": [{ "expr": "stage_duration_seconds", "legendFormat": "{{stage}}" }]
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
      "title": "Login Failures",
      "targets": [{ "expr": "login_failures_total", "legendFormat": "{{reason}}" }]
    }
  ]
}
```

Grafana loads the dashboard on startup. Navigate to `http://localhost:3000` to view the charts.

## Alerting

Grafana supports alert rules on any Prometheus query. To be notified when many login attempts fail, open the *Login Failures* panel and create an alert with `increase(login_failures_total[5m]) > 5`. Configure a notification channel such as email or Slack to receive alerts.
