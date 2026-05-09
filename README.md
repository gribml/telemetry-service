# Telemetry Service

A Rust service that collects and publishes CPU and memory utilization telemetry using OpenTelemetry standards.

## Features

- 📊 **CPU Monitoring**: Tracks overall CPU utilization percentage
- 💾 **Memory Monitoring**: Tracks memory usage (percentage, used bytes, and total bytes)
- 🔄 **OpenTelemetry Integration**: Uses industry-standard OpenTelemetry for metrics
- 📤 **Multiple Export Formats**: 
  - Stdout (human-readable logs)
  - Prometheus HTTP endpoint (`/metrics`)
  - OTLP (configurable)
- ⚡ **Async Runtime**: Built with Tokio for efficient async operations
- 🏷️ **Rich Metadata**: Includes hostname and service information
- 📊 **Prometheus Ready**: Built-in HTTP server exposing `/metrics` endpoint

## Metrics Collected

### CPU Metrics
- `system.cpu.utilization` - CPU usage percentage (0-100%)
  - Unit: `%`
  - Type: Observable Gauge
  - Prometheus name: `system_cpu_utilization_percent`

- `system.cpu.time` - Cumulative CPU time consumed across all cores
  - Unit: `s`
  - Type: Observable Counter
  - Prometheus name: `system_cpu_time_total`

### Memory Metrics
- `system.memory.utilization` - Memory usage percentage (0-100%)
  - Unit: `%`
  - Type: Observable Gauge
  - Prometheus name: `system_memory_utilization_percent`

- `system.memory.usage` - Memory used in bytes
  - Unit: `By` (bytes)
  - Type: Observable Gauge
  - Prometheus name: `system_memory_usage_bytes`

- `system.memory.total` - Total available memory in bytes
  - Unit: `By` (bytes)
  - Type: Observable Gauge
  - Prometheus name: `system_memory_total_bytes`

### Process Metrics

- `process.cpu.usage` - CPU usage of the telemetry-service process
  - Unit: `%`
  - Type: Observable Gauge
  - Prometheus name: `process_cpu_usage_percent`
  - Note: per-core percentage (100% = one full core)

- `process.memory.usage` - RSS memory usage of the telemetry-service process
  - Unit: `By` (bytes)
  - Type: Observable Gauge
  - Prometheus name: `process_memory_usage_bytes`

All metrics include the `host.name` attribute for identification.

## Prometheus Integration

The service exposes metrics in Prometheus format via HTTP:

- **Metrics Endpoint**: `http://localhost:8080/metrics`
- **Health Check**: `http://localhost:8080/health`

Prometheus automatically scrapes these metrics every 15 seconds (configurable).

**How it works:**
1. Service collects CPU/memory metrics using OpenTelemetry
2. Prometheus exporter converts to Prometheus format
3. HTTP server exposes `/metrics` endpoint
4. Prometheus scrapes endpoint every 15 seconds
5. Data stored in Prometheus TSDB for querying

See [PROMETHEUS_INTEGRATION.md](PROMETHEUS_INTEGRATION.md) for detailed documentation.

## Installation

### Prerequisites
- Rust 1.70 or later
- Cargo

### Build from Source

```bash
cd telemetry-service
cargo build --release
```

## Usage

### Quick Start with Make

The project includes a Makefile for common tasks:

```bash
# See all available commands
make help

# Run the service
make run

# Run tests
make test

# Build release binary
make release

# Start full stack with Podman Compose
make compose-up
```

### Run the Service (Traditional)

```bash
cargo run
```

The service will:
1. Initialize OpenTelemetry metrics
2. Start collecting CPU and memory metrics
3. Export metrics to stdout every 5 seconds
4. Run until interrupted with Ctrl+C

### Example Output

```
📊 Telemetry Export at 2024-01-15 10:30:45
─────────────────────────────────────────────────────────
  Host: your-hostname
  CPU Utilization: 33.25%
  CPU Time (accumulated): 12.450s
  Memory Utilization: 79.17%
  Memory Usage: 12.67 GB / 16.00 GB
  Memory Usage (bytes): 13601275904 / 17179869184
  [self] CPU: 0.21%  Memory: 14.3 MB
```

## Deployment

**⚠️ IMPORTANT:** This service should run directly on the host machine, NOT in a container, because:
1. It needs to measure the actual host's CPU and memory, not container resources
2. The metrics endpoint must be accessible from the network for Prometheus to scrape

### Recommended: Native Binary Installation

#### Quick Install

```bash
# Build release binary
make release

# Install to system
make install
```

#### Run as Foreground Service

```bash
# Run directly
cargo run --release

# Or if installed
telemetry-service
```

The service will:
- Bind to `0.0.0.0:8080` (accessible from network)
- Expose metrics at `http://<your-ip>:8080/metrics`
- Log to stdout every 5 seconds

### Linux systemd Service (Recommended for Production)

1. Build and install:
```bash
make install-linux-service
```

Or manually:

```bash
# Build release binary
cargo build --release

# Install binary
sudo cp target/release/telemetry-service /usr/local/bin/
sudo chmod +x /usr/local/bin/telemetry-service

# Install systemd service
sudo cp telemetry-service.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable telemetry-service
sudo systemctl start telemetry-service
```

2. Check status:
```bash
sudo systemctl status telemetry-service
sudo journalctl -u telemetry-service -f

# Test metrics endpoint
curl http://localhost:8080/metrics
```

3. Configure firewall (if needed):
```bash
# Allow Prometheus to scrape from other machines
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --reload
```

### macOS launchd Service (Recommended for Production)

1. Build and install:
```bash
make install-macos-service
```

Or manually:

```bash
# Build release binary
cargo build --release

# Install binary
sudo cp target/release/telemetry-service /usr/local/bin/
sudo chmod +x /usr/local/bin/telemetry-service

# Create directories
sudo mkdir -p /usr/local/var/log

# Install launchd service
sudo cp com.telemetry.service.plist /Library/LaunchDaemons/
sudo launchctl load /Library/LaunchDaemons/com.telemetry.service.plist
```

2. Check status:
```bash
sudo launchctl list | grep telemetry
tail -f /usr/local/var/log/telemetry-service.log

# Test metrics endpoint
curl http://localhost:8080/metrics
```

3. Allow network access (if needed):
```bash
# macOS firewall will prompt to allow connections
# Or configure in System Preferences → Security & Privacy → Firewall
```

### Monitoring Stack (Prometheus, Grafana, Jaeger)

Run the supporting observability stack in containers while the telemetry service runs natively:

```bash
# Run monitoring stack in containers
make compose-up

# Run telemetry service natively on the host
cargo run --release
```

## Configuration

### Change Export Interval

Modify the interval in `src/main.rs`:

```rust
let mut interval = time::interval(Duration::from_secs(10)); // Change from 5 to 10 seconds
```

### Export to OTLP Endpoint

Replace the stdout exporter with OTLP exporter in `src/main.rs`:

```rust
use opentelemetry_otlp::WithExportConfig;

fn init_metrics() -> Result<SdkMeterProvider> {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317")
        .build_metrics_exporter(
            Box::new(DefaultAggregationSelector::new()),
            Box::new(DefaultTemporalitySelector::new()),
        )?;
    
    // ... rest of the setup
}
```

### Add Custom Attributes

Add more attributes to metrics in the callback:

```rust
observer.observe(usage, &[
    KeyValue::new("host.name", hostname),
    KeyValue::new("environment", "production"),
    KeyValue::new("region", "us-west-2"),
]);
```

## Architecture

```
┌─────────────────────┐
│  SystemMetrics      │
│  - Collects OS data │
│  - Thread-safe      │
└──────────┬──────────┘
           │
           │ Updates every 5s
           ▼
┌─────────────────────┐
│  OpenTelemetry      │
│  Observable Gauges  │
│  - CPU utilization  │
│  - Memory stats     │
└──────────┬──────────┘
           │
           │ Exports
           ▼
┌─────────────────────┐
│  Exporter           │
│  - Stdout (default) │
│  - OTLP (optional)  │
│  - Prometheus       │
└─────────────────────┘
```

## Dependencies

- **opentelemetry**: Core OpenTelemetry API
- **opentelemetry_sdk**: OpenTelemetry SDK with Tokio runtime support
- **opentelemetry-otlp**: OTLP exporter for remote telemetry backends
- **opentelemetry-stdout**: Stdout exporter for development/debugging
- **sysinfo**: Cross-platform system information library
- **tokio**: Async runtime
- **tracing**: Logging and diagnostics
- **anyhow**: Error handling

## Use Cases

1. **Development Monitoring**: Run locally to monitor system resources during development
2. **Production Telemetry**: Export to observability platforms (Jaeger, Prometheus, Grafana, etc.)
3. **Performance Testing**: Track system utilization during load tests
4. **Resource Optimization**: Identify memory leaks or CPU bottlenecks
5. **Infrastructure Monitoring**: Deploy as a system service for continuous monitoring

## Integrations

The service can export metrics to any OpenTelemetry-compatible backend:

- **Jaeger**: Distributed tracing and monitoring
- **Prometheus**: Time-series metrics storage and alerting
- **Grafana**: Visualization and dashboards
- **Datadog**: Cloud monitoring and analytics
- **New Relic**: Application performance monitoring
- **Honeycomb**: Observability platform
- **AWS CloudWatch**: AWS native monitoring

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
