# Prometheus Integration Guide

## How It Works

The telemetry service now exposes metrics in Prometheus format through an HTTP endpoint that Prometheus can scrape.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Telemetry Service (Port 8080)                              │
│                                                               │
│  ┌─────────────────┐        ┌──────────────────┐           │
│  │  SystemMetrics   │───────▶│  OpenTelemetry   │           │
│  │  - CPU Usage     │        │  Observable      │           │
│  │  - Memory Usage  │        │  Gauges          │           │
│  └─────────────────┘        └──────────┬───────┘           │
│                                         │                     │
│                                         ▼                     │
│                             ┌───────────────────┐            │
│                             │  Prometheus       │            │
│                             │  Exporter         │            │
│                             └─────────┬─────────┘            │
│                                       │                       │
│                                       ▼                       │
│                             ┌───────────────────┐            │
│                             │  HTTP Server      │            │
│                             │  /metrics         │◀───────────┤─┐
│                             │  /health          │            │ │
│                             └───────────────────┘            │ │
└─────────────────────────────────────────────────────────────┘ │
                                                                  │
                                                                  │
┌─────────────────────────────────────────────────────────────┐ │
│  Prometheus Server (Port 9090)                              │ │
│                                                               │ │
│  ┌──────────────────┐                                        │ │
│  │  Scrape Config   │                                        │ │
│  │  - job: telemetry│                                        │ │
│  │  - target: :8080 │────────────────────────────────────────┘
│  │  - interval: 15s │                                        │
│  └─────────┬────────┘                                        │
│            │                                                  │
│            ▼                                                  │
│  ┌──────────────────┐       ┌──────────────────┐            │
│  │  Time Series DB  │──────▶│  Query Engine    │            │
│  │  (TSDB)          │       │  (PromQL)        │            │
│  └──────────────────┘       └─────────┬────────┘            │
│                                        │                      │
│                                        ▼                      │
│                             ┌──────────────────┐             │
│                             │  Web UI / API    │             │
│                             │  :9090           │             │
│                             └──────────────────┘             │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. OpenTelemetry Observable Gauges

The service creates Observable Gauges that automatically collect metrics:

```rust
meter.f64_observable_gauge("system.cpu.utilization")
    .with_callback(move |observer| {
        let usage = cpu_metrics.get_cpu_usage();
        observer.observe(usage, &[KeyValue::new("host.name", hostname)]);
    })
    .init();
```

**Key Points:**
- Gauges are "observable" - they're queried on-demand
- Callbacks run when Prometheus scrapes `/metrics`
- Each metric includes labels (e.g., `host.name`)

### 2. Prometheus Exporter

The `opentelemetry-prometheus` crate converts OpenTelemetry metrics to Prometheus format:

```rust
let exporter = opentelemetry_prometheus::exporter()
    .with_registry(registry.clone())
    .build()?;
```

**What it does:**
- Registers with OpenTelemetry's metric provider
- Converts metric names (e.g., `system.cpu.utilization` → `system_cpu_utilization`)
- Formats metrics in Prometheus text format
- Stores metrics in Prometheus registry

### 3. HTTP Server (Axum)

Exposes two endpoints:

```rust
Router::new()
    .route("/metrics", get(metrics_handler))  // Prometheus scrapes this
    .route("/health", get(health_handler))    // Health checks
```

**Endpoints:**
- `GET /metrics` - Returns metrics in Prometheus format
- `GET /health` - Returns "OK" for health checks

### 4. Metrics Handler

Serializes metrics from the Prometheus registry:

```rust
let metric_families = registry.gather();
let encoder = TextEncoder::new();
encoder.encode(&metric_families, &mut buffer)?;
```

## Metrics Format

### OpenTelemetry Format (Internal)
```
system.cpu.utilization{host.name="my-host"} = 45.2
system.memory.utilization{host.name="my-host"} = 78.5
```

### Prometheus Format (HTTP Response)
```prometheus
# HELP system_cpu_utilization CPU utilization percentage
# TYPE system_cpu_utilization gauge
system_cpu_utilization{host_name="my-host"} 45.2

# HELP system_memory_utilization Memory utilization percentage
# TYPE system_memory_utilization gauge
system_memory_utilization{host_name="my-host"} 78.5

# HELP system_memory_usage Memory used in bytes
# TYPE system_memory_usage gauge
system_memory_usage{host_name="my-host"} 13710819328

# HELP system_memory_total Total memory in bytes
# TYPE system_memory_total gauge
system_memory_total{host_name="my-host"} 17179869184
```

**Note the transformations:**
- Dots (`.`) → Underscores (`_`)
- Label keys: `host.name` → `host_name`
- Includes `# HELP` and `# TYPE` metadata

## Prometheus Configuration

### Scrape Config (`prometheus/prometheus.yml`)

```yaml
scrape_configs:
  - job_name: 'telemetry-service'
    scrape_interval: 15s           # How often to scrape
    static_configs:
      - targets: ['telemetry-service:8080']  # Service hostname:port
    metrics_path: '/metrics'       # Endpoint to scrape
```

**How it works:**
1. Every 15 seconds, Prometheus makes an HTTP GET request
2. `GET http://telemetry-service:8080/metrics`
3. Receives metrics in Prometheus format
4. Stores time-series data in TSDB
5. Makes data available for querying

## Testing the Integration

### 1. Start the Service

```bash
# Standalone
cargo run

# With Podman Compose
make compose-up
```

### 2. Verify Metrics Endpoint

```bash
# Check health
curl http://localhost:8080/health

# View metrics
curl http://localhost:8080/metrics

# Filter specific metrics
curl http://localhost:8080/metrics | grep system_cpu
```

**Expected output:**
```prometheus
# HELP system_cpu_utilization CPU utilization percentage
# TYPE system_cpu_utilization gauge
system_cpu_utilization{host_name="special-circumstances.local"} 42.83
```

### 3. Check Prometheus Scraping

Open Prometheus UI: http://localhost:9090

**Verify targets:**
1. Go to Status → Targets
2. Look for `telemetry-service` job
3. Should show state: `UP`
4. Last scrape: Recent timestamp

**Query metrics:**
1. Go to Graph
2. Enter query: `system_cpu_utilization`
3. Execute
4. Should see data points

## PromQL Queries

Once Prometheus is scraping, you can query metrics:

### Current Values
```promql
# Current CPU usage
system_cpu_utilization

# Current memory percentage
system_memory_utilization
```

### Aggregations
```promql
# Average CPU over 5 minutes
avg_over_time(system_cpu_utilization[5m])

# Maximum memory usage in last hour
max_over_time(system_memory_utilization[1h])

# Rate of change
rate(system_memory_usage[5m])
```

### Filtering by Labels
```promql
# CPU for specific host
system_cpu_utilization{host_name="my-host"}

# Memory across all hosts
sum(system_memory_utilization) by (host_name)
```

### Calculations
```promql
# Available memory in GB
(system_memory_total - system_memory_usage) / 1024 / 1024 / 1024

# Memory usage percentage (alternative calculation)
(system_memory_usage / system_memory_total) * 100
```

## Grafana Integration

### 1. Add Prometheus Data Source

1. Open Grafana: http://localhost:3000
2. Configuration → Data Sources → Add data source
3. Select "Prometheus"
4. URL: `http://prometheus:9090`
5. Save & Test

### 2. Create Dashboard

Example panel queries:

**CPU Usage Panel:**
```promql
system_cpu_utilization{host_name=~".*"}
```

**Memory Usage Panel:**
```promql
(system_memory_usage / system_memory_total) * 100
```

**Memory Available Panel:**
```promql
system_memory_total - system_memory_usage
```

### 3. Import Example Dashboard

You can create a dashboard JSON:

```json
{
  "dashboard": {
    "title": "System Telemetry",
    "panels": [
      {
        "title": "CPU Usage",
        "targets": [
          {
            "expr": "system_cpu_utilization"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Memory Usage %",
        "targets": [
          {
            "expr": "system_memory_utilization"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
```

## Troubleshooting

### Metrics Endpoint Returns Empty

**Problem:** `curl http://localhost:8080/metrics` returns no metrics

**Solutions:**
1. Wait 1-2 seconds after service starts (CPU metrics need initialization)
2. Check logs for errors
3. Verify OpenTelemetry initialization succeeded

### Prometheus Shows Target Down

**Problem:** Prometheus UI shows telemetry-service as "DOWN"

**Solutions:**
1. Check network connectivity:
   ```bash
   podman exec prometheus ping telemetry-service
   ```
2. Verify port is exposed in `podman-compose.yml`
3. Check service is running: `podman ps`
4. Verify endpoint: `curl http://localhost:8080/metrics`

### Metrics Not Updating

**Problem:** Metrics exist but values don't change

**Solutions:**
1. Check scrape interval in `prometheus.yml`
2. Verify system metrics are actually changing
3. Check Prometheus logs: `podman logs prometheus`

### Wrong Metric Names

**Problem:** Metrics have unexpected names

**Cause:** OpenTelemetry → Prometheus name transformation

**Example transformations:**
- `system.cpu.utilization` → `system_cpu_utilization`
- `host.name` label → `host_name`

## Port Configuration

### Current Setup
- **Telemetry Service**: 8080 (metrics endpoint)
- **Prometheus**: 9090 (UI and API)
- **Grafana**: 3000 (UI)
- **Jaeger**: 16686 (UI), 4317 (OTLP)

### Changing the Metrics Port

**1. Update `src/main.rs`:**
```rust
let addr = SocketAddr::from(([0, 0, 0, 0], 8081)); // Change port
```

**2. Update `podman-compose.yml`:**
```yaml
ports:
  - "8081:8081"  # Change both sides
```

**3. Update `prometheus/prometheus.yml`:**
```yaml
targets: ['telemetry-service:8081']  # Change port
```

## Performance Considerations

### Scrape Interval

Shorter intervals = more data points but more load:

```yaml
scrape_interval: 5s   # High frequency, more load
scrape_interval: 15s  # Default, balanced
scrape_interval: 60s  # Low frequency, less load
```

### Memory Usage

Prometheus stores all scraped data:
- Default retention: 15 days
- Disk space grows with: scrape frequency × metrics count × retention

### Service Overhead

The Prometheus exporter adds minimal overhead:
- HTTP server: ~1-2 MB RAM
- Metric collection: Triggered only on scrape
- No background processing

## Advanced Configuration

### Authentication

Add basic auth to metrics endpoint:

```rust
use axum::middleware;
use tower_http::auth::RequireAuthorizationLayer;

let app = Router::new()
    .route("/metrics", get(metrics_handler))
    .layer(RequireAuthorizationLayer::basic("username", "password"));
```

### TLS/HTTPS

Use `axum-server` with TLS:

```toml
[dependencies]
axum-server = { version = "0.6", features = ["tls-rustls"] }
```

### Custom Labels

Add more labels to metrics:

```rust
observer.observe(usage, &[
    KeyValue::new("host.name", hostname),
    KeyValue::new("environment", "production"),
    KeyValue::new("region", "us-west-2"),
]);
```

## References

- [OpenTelemetry Metrics](https://opentelemetry.io/docs/specs/otel/metrics/)
- [Prometheus Exposition Format](https://prometheus.io/docs/instrumenting/exposition_formats/)
- [PromQL Documentation](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Grafana Prometheus Data Source](https://grafana.com/docs/grafana/latest/datasources/prometheus/)
