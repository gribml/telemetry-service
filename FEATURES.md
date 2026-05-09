# Features & Technical Overview

## Core Features

### 📊 System Metrics Collection

The service collects the following metrics every 5 seconds (configurable):

1. **CPU Utilization** (`system.cpu.utilization`)
   - Type: Observable Gauge (f64)
   - Unit: Percentage (%)
   - Description: Overall CPU usage across all cores
   - Range: 0-100%

2. **Memory Utilization** (`system.memory.utilization`)
   - Type: Observable Gauge (f64)
   - Unit: Percentage (%)
   - Description: Memory usage as a percentage of total available
   - Range: 0-100%

3. **Memory Usage** (`system.memory.usage`)
   - Type: Observable Gauge (u64)
   - Unit: Bytes
   - Description: Absolute memory usage in bytes
   
4. **Memory Total** (`system.memory.total`)
   - Type: Observable Gauge (u64)
   - Unit: Bytes
   - Description: Total available system memory

### 🏷️ Metadata & Attributes

All metrics include:
- **Host Name**: Automatically detected system hostname
- **Service Name**: "telemetry-service"
- **Service Version**: "0.1.0"

### 🔄 OpenTelemetry Integration

Built on industry-standard OpenTelemetry:
- **Observable Gauges**: Metrics are collected on-demand
- **Resource Attributes**: Service identification and metadata
- **Meter Provider**: Centralized metrics management
- **Extensible**: Easy to add custom metrics

## Technical Architecture

### Stack

```
┌─────────────────────────────────────┐
│  Application Layer                  │
│  - Tokio async runtime              │
│  - Signal handling (Ctrl+C)         │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│  Metrics Collection Layer           │
│  - OpenTelemetry SDK                │
│  - Observable Gauges                │
│  - 5-second interval                │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│  System Information Layer           │
│  - sysinfo crate                    │
│  - CPU monitoring                   │
│  - Memory monitoring                │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│  Export Layer                       │
│  - Stdout (default)                 │
│  - OTLP (configurable)              │
│  - JSON format                      │
└─────────────────────────────────────┘
```

### Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| opentelemetry | 0.22 | Core OpenTelemetry API |
| opentelemetry_sdk | 0.22 | SDK implementation with Tokio runtime |
| opentelemetry-otlp | 0.15 | OTLP protocol support |
| sysinfo | 0.30 | Cross-platform system information |
| tokio | 1.36 | Async runtime |
| chrono | 0.4 | Timestamp formatting |
| hostname | 0.3 | Hostname detection |
| anyhow | 1.0 | Error handling |

### Performance Characteristics

- **Memory Footprint**: ~10-20 MB
- **CPU Usage**: <1% average
- **Collection Interval**: 5 seconds (configurable)
- **Startup Time**: <1 second
- **Thread Count**: Minimal (Tokio async)

## Export Formats

### 1. Stdout (Default)

Human-readable format printed to console:

```
📊 Telemetry Export at 2024-01-15 10:30:45
─────────────────────────────────────────────────────────
  Host: your-hostname
  CPU Utilization: 33.25%
  Memory Utilization: 79.17%
  Memory Usage: 12.67 GB / 16.00 GB
  Memory Usage (bytes): 13601275904 / 17179869184
```

### 2. OTLP (OpenTelemetry Protocol)

Industry-standard protocol for sending telemetry data to backends:
- Protocol: gRPC
- Format: Protobuf
- Endpoints: Jaeger, Prometheus, Grafana, etc.

### 3. JSON (Custom Implementation)

Structured JSON format for logging systems or custom integrations.

## Platform Support

### Operating Systems

- ✅ **macOS**: Full support (tested)
- ✅ **Linux**: Full support
- ✅ **Windows**: Full support (via sysinfo)
- ✅ **FreeBSD**: Full support
### Deployment Options

1. **Standalone Binary**
   - Single executable
   - No dependencies
   - Cross-platform

2. **System Service**
   - systemd (Linux)
   - launchd (macOS)
   - Windows Service Manager

## Security Features

### System Service Security

- Dedicated service user
- Restricted file system access
- Private temporary directories
- No new privileges flag

### Network Security

- HTTPS/TLS support for OTLP
- Configurable endpoints
- No exposed ports (default config)

## Extensibility

### Adding Custom Metrics

Easy to extend with additional metrics:

```rust
// Example: Add disk usage metric
let disk_gauge = meter
    .u64_observable_gauge("system.disk.usage")
    .with_description("Disk usage in bytes")
    .with_unit(Unit::new("By"))
    .with_callback(move |observer| {
        let usage = get_disk_usage();
        observer.observe(usage, &[KeyValue::new("host.name", hostname)]);
    })
    .init();
```

### Custom Exporters

Pluggable exporter architecture:
- Implement custom export logic
- Support any backend
- Multiple exporters simultaneously

### Configuration Options

- Export interval
- Metric selection
- Attribute customization
- Backend endpoints
- Authentication

## Use Cases

### 1. Development Monitoring
Monitor system resources during local development.

### 2. Production Observability
Track server health in production environments.

### 3. Performance Testing
Measure system utilization during load tests.

### 4. Resource Optimization
Identify bottlenecks and optimization opportunities.

### 5. Capacity Planning
Historical data for infrastructure planning.

### 6. Alert Generation
Feed into alerting systems (Prometheus Alertmanager, etc.).

### 7. Dashboard Visualization
Power Grafana dashboards with real-time metrics.

## Roadmap

### Planned Features

- [ ] Configurable metrics via TOML file
- [ ] Additional system metrics (disk, network)
- [ ] Prometheus exporter endpoint
- [ ] Per-process metrics
- [ ] GPU utilization tracking
- [ ] Custom alerting thresholds
- [ ] Web UI for configuration
- [ ] Historical data storage
- [ ] Metric aggregation
- [ ] Multi-host deployment support

### Future Integrations

- [ ] AWS CloudWatch native integration
- [ ] Azure Monitor integration
- [ ] Google Cloud Monitoring
- [ ] Datadog native exporter
- [ ] New Relic integration
- [ ] Splunk integration

## Testing

### Test Coverage

- ✅ System metrics collection
- ✅ Memory unit conversions
- ✅ Hostname retrieval
- ✅ OpenTelemetry initialization
- ✅ Observable gauge creation

### Running Tests

```bash
cargo test
```

### Integration Testing

```bash
# Run with Podman Compose
podman-compose up -d
# Or use: make compose-up

# Verify metrics in Jaeger
curl http://localhost:16686
```

## Contributing

We welcome contributions! Areas where help is needed:

1. **Additional Metrics**: Disk, network, GPU
2. **Exporters**: New backend integrations
3. **Configuration**: TOML/YAML config file support
4. **Documentation**: Tutorials and guides
5. **Testing**: More comprehensive test coverage
6. **Performance**: Optimization opportunities

## License

MIT License - see LICENSE file for details.

## Acknowledgments

Built with:
- OpenTelemetry community
- Rust ecosystem
- sysinfo library maintainers
- Tokio async runtime team
