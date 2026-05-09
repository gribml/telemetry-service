# Quick Start Guide

This guide will help you get the telemetry service up and running in just a few minutes.

## 1. Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: For cloning the repository

## 2. Clone and Build

```bash
# Clone the repository
git clone https://github.com/yourusername/telemetry-service.git
cd telemetry-service

# Build the project
cargo build --release

# The binary will be at: target/release/telemetry-service
```

## 3. Run Locally

### Simple Test Run

```bash
cargo run
```

You should see output like:

```
🚀 Starting Telemetry Service...
📊 Collecting CPU and Memory metrics with OpenTelemetry
📤 Publishing metrics to stdout every 5 seconds
Press Ctrl+C to stop

📊 Telemetry Export at 2024-01-15 10:30:45
─────────────────────────────────────────────────────────
  Host: your-machine
  CPU Utilization: 25.50%
  Memory Utilization: 65.30%
  Memory Usage: 10.45 GB / 16.00 GB
  Memory Usage (bytes): 11220156416 / 17179869184
```

Press `Ctrl+C` to stop the service.

## 4. Podman Quick Start

### Build and Run

```bash
# Build Podman image
podman build -t telemetry-service .

# Run the container
podman run -it --rm telemetry-service
```

Or use the Makefile:

```bash
make podman         # Build image
make podman-run     # Run container
```

### Using Podman Compose with Full Stack

Run with Jaeger, Prometheus, and Grafana:

```bash
podman-compose up -d
```

Or use the Makefile:

```bash
make compose-up     # Start all services
make compose-logs   # View logs
make compose-down   # Stop all services
```

Then access:
- **Telemetry Service**: Check logs with `podman logs telemetry-service -f`
- **Jaeger UI**: http://localhost:16686
- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000 (login: admin/admin)

## 5. Integration Examples

### A. Export to Jaeger

1. Start Jaeger:
```bash
podman run -d --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  docker.io/jaegertracing/all-in-one:latest
```

2. Update `src/main.rs` to use OTLP exporter (see OTLP configuration in README)

3. Run the service:
```bash
cargo run
```

4. View metrics in Jaeger UI at http://localhost:16686

### B. Export to Prometheus

1. Create `prometheus` directory and `prometheus/prometheus.yml`:
```bash
mkdir -p prometheus
cat > prometheus/prometheus.yml << EOF
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'telemetry-service'
    static_configs:
      - targets: ['localhost:8080']
EOF
```

2. Start Prometheus:
```bash
podman run -d --name prometheus \
  -p 9090:9090 \
  -v $(pwd)/prometheus:/etc/prometheus:Z \
  docker.io/prom/prometheus
```

3. Add Prometheus exporter to the service (requires code modification)

4. View metrics at http://localhost:9090

### C. Export to Grafana Cloud

1. Sign up for a free [Grafana Cloud](https://grafana.com/products/cloud/) account

2. Get your OTLP endpoint and API token

3. Update the service to use your endpoint:
```rust
let exporter = opentelemetry_otlp::new_exporter()
    .tonic()
    .with_endpoint("https://otlp-gateway-prod-us-east-0.grafana.net/otlp")
    .with_metadata([
        ("authorization", format!("Bearer {}", api_token))
    ])
    // ... rest of config
```

4. Run and view metrics in Grafana Cloud

## 6. Production Deployment

### Linux (systemd)

```bash
# Build release
cargo build --release

# Install
sudo cp target/release/telemetry-service /usr/local/bin/
sudo cp telemetry-service.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable telemetry-service
sudo systemctl start telemetry-service

# Check status
sudo systemctl status telemetry-service
```

### macOS (launchd)

```bash
# Build release
cargo build --release

# Install
sudo cp target/release/telemetry-service /usr/local/bin/
sudo cp com.telemetry.service.plist /Library/LaunchDaemons/
sudo launchctl load /Library/LaunchDaemons/com.telemetry.service.plist

# Check status
sudo launchctl list | grep telemetry
```

### Kubernetes

Create `deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: telemetry-service
spec:
  replicas: 1
  selector:
    matchLabels:
      app: telemetry-service
  template:
    metadata:
      labels:
        app: telemetry-service
    spec:
      containers:
      - name: telemetry-service
        image: telemetry-service:latest
        resources:
          limits:
            memory: "256Mi"
            cpu: "500m"
        env:
        - name: OTEL_EXPORTER_OTLP_ENDPOINT
          value: "http://jaeger:4317"
```

Deploy:
```bash
kubectl apply -f deployment.yaml
```

## 7. Monitoring the Service

### View Logs

**Podman:**
```bash
podman logs telemetry-service -f
```

**Linux systemd:**
```bash
sudo journalctl -u telemetry-service -f
```

**macOS launchd:**
```bash
tail -f /usr/local/var/log/telemetry-service.log
```

### Check Performance

The service is designed to be lightweight:
- **Memory usage**: ~10-20 MB
- **CPU usage**: <1% average
- **Metrics interval**: 5 seconds (configurable)

## 8. Troubleshooting

### Service won't start

1. Check if port is already in use
2. Verify file permissions
3. Check system logs for errors

### No metrics appearing

1. Verify the service is running
2. Check network connectivity to exporters
3. Review exporter configuration
4. Check firewall settings

### High CPU/Memory usage

1. Increase metrics collection interval
2. Reduce number of metrics
3. Check for system resource constraints

## 9. Next Steps

- Read the full [README.md](README.md) for detailed configuration
- Explore the `src/main.rs` to understand the implementation
- Customize metrics collection for your use case
- Integrate with your existing monitoring infrastructure
- Set up alerts and dashboards in your observability platform

## 10. Support

- Open an issue on GitHub
- Check existing issues for solutions
- Contribute improvements via pull requests

Happy monitoring! 📊
