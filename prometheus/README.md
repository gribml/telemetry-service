# Prometheus Configuration

This directory contains Prometheus configuration files for monitoring the telemetry service.

## Files

- **prometheus.yml**: Main Prometheus configuration file
- **alerts.yml**: Example alerting rules (optional)

## Usage

### With Podman Compose

The podman-compose.yml file already mounts this directory:

```bash
podman-compose up -d
# Or use: make compose-up
```

Prometheus will be available at: http://localhost:9090

### Standalone Prometheus

Run Prometheus with this configuration:

```bash
podman run -d \
  --name prometheus \
  -p 9090:9090 \
  -v $(pwd)/prometheus:/etc/prometheus:Z \
  -v prometheus-data:/prometheus:Z \
  docker.io/prom/prometheus \
  --config.file=/etc/prometheus/prometheus.yml \
  --storage.tsdb.path=/prometheus
```

## Enabling Alerts

To enable alerting rules:

1. Uncomment the `rule_files` section in `prometheus.yml`:
   ```yaml
   rule_files:
     - "alerts.yml"
   ```

2. Restart Prometheus:
   ```bash
   podman-compose restart prometheus
   # Or: make compose-down && make compose-up
   ```

3. View alerts in the Prometheus UI: http://localhost:9090/alerts

## Scrape Targets

### Current Configuration

- **prometheus**: Prometheus self-monitoring (localhost:9090)
- **telemetry-service**: The telemetry service (when Prometheus exporter is enabled)
- **jaeger**: Jaeger metrics endpoint (jaeger:14269)

### Adding Custom Targets

Add new scrape targets to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'my-service'
    static_configs:
      - targets: ['my-service:8080']
```

## Alert Rules

The `alerts.yml` file includes example alerts for:

- **HighCPUUsage**: CPU > 80% for 5 minutes
- **CriticalCPUUsage**: CPU > 95% for 2 minutes
- **HighMemoryUsage**: Memory > 85% for 5 minutes
- **CriticalMemoryUsage**: Memory > 95% for 2 minutes
- **TelemetryServiceDown**: Service unavailable for 1 minute
- **MemoryLeakDetected**: Rapid memory increase over 10 minutes

## Connecting to Alertmanager

To send alerts to Alertmanager:

1. Update the `alerting` section in `prometheus.yml`:
   ```yaml
   alerting:
     alertmanagers:
       - static_configs:
           - targets: ['alertmanager:9093']
   ```

2. Start Alertmanager:
   ```bash
   podman run -d \
     --name alertmanager \
     -p 9093:9093 \
     --network monitoring \
     docker.io/prom/alertmanager
   ```

## Grafana Integration

Import Prometheus as a data source in Grafana:

1. Open Grafana: http://localhost:3000
2. Go to Configuration > Data Sources
3. Add Prometheus:
   - URL: `http://prometheus:9090`
   - Access: Server (default)
4. Save & Test

## PromQL Examples

### CPU Queries

```promql
# Current CPU usage
system_cpu_utilization

# Average CPU over 5 minutes
avg_over_time(system_cpu_utilization[5m])

# Hosts with CPU > 80%
system_cpu_utilization > 80
```

### Memory Queries

```promql
# Current memory usage percentage
system_memory_utilization

# Memory in GB
system_memory_usage / 1024 / 1024 / 1024

# Available memory
system_memory_total - system_memory_usage
```

## Retention & Storage

Default retention: 15 days

To change retention:

```bash
podman run -d \
  --name prometheus \
  -p 9090:9090 \
  -v $(pwd)/prometheus:/etc/prometheus:Z \
  docker.io/prom/prometheus \
  --config.file=/etc/prometheus/prometheus.yml \
  --storage.tsdb.retention.time=30d
```

## Troubleshooting

### Prometheus won't start

Check logs:
```bash
podman logs prometheus
```

Common issues:
- Invalid YAML syntax in prometheus.yml
- File permissions issues
- Port 9090 already in use

### No metrics appearing

1. Check if targets are up: http://localhost:9090/targets
2. Verify scrape configuration
3. Ensure telemetry-service exposes metrics endpoint

### Alerts not firing

1. Check alert rules: http://localhost:9090/alerts
2. Verify PromQL expressions
3. Check evaluation interval in prometheus.yml
