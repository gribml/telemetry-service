# Docker to Podman Migration Summary

This document outlines the changes made to migrate from Docker to Podman.

## Changes Made

### 1. Container Compose File

**Changed:**
- `docker-compose.yml` → `podman-compose.yml`

**Key Updates:**
- All images now use fully qualified names (e.g., `docker.io/prom/prometheus:latest`)
- Added `:Z` flag to volume mounts for SELinux compatibility
- Added `prometheus-data` volume for persistent storage

```yaml
# Before
image: prom/prometheus:latest
volumes:
  - ./prometheus.yml:/etc/prometheus/prometheus.yml

# After
image: docker.io/prom/prometheus:latest
volumes:
  - ./prometheus:/etc/prometheus:Z
  - prometheus-data:/prometheus:Z
```

### 2. Dockerfile

**Updated:**
- Changed base images to use fully qualified names
- Updated Rust version to 1.93
- Maintained rootless user configuration

```dockerfile
# Before
FROM rust:1.76 as builder
FROM debian:bookworm-slim

# After
FROM docker.io/library/rust:1.93 as builder
FROM docker.io/library/debian:bookworm-slim
```

### 3. Makefile

**Changed Commands:**
- `docker` → `podman`
- `docker-run` → `podman-run`
- `docker-compose` → `podman-compose`

All container-related commands now use Podman:
```bash
make podman         # Build image
make podman-run     # Run container
make compose-up     # Start services
make compose-down   # Stop services
```

### 4. Documentation Updates

**Files Updated:**
- `README.md` - Changed all Docker references to Podman
- `QUICKSTART.md` - Updated all examples with Podman commands
- `FEATURES.md` - Added Podman to supported platforms
- `prometheus/README.md` - Updated Prometheus setup with Podman

**New Files:**
- `PODMAN.md` - Comprehensive Podman deployment guide
- `podman-compose.yml` - Podman compose configuration

### 5. Volume Mounting

**SELinux Compatibility:**
Added `:Z` flag to all volume mounts for proper SELinux labeling on Fedora/RHEL/CentOS:

```yaml
volumes:
  - ./prometheus:/etc/prometheus:Z
  - prometheus-data:/prometheus:Z
  - grafana-storage:/var/lib/grafana:Z
```

This ensures containers can properly access host directories on SELinux-enabled systems.

## Benefits of Podman

### Security
- **Rootless by default** - No daemon running as root
- **User namespaces** - Better process isolation
- **No daemon** - Reduced attack surface

### Compatibility
- **OCI compliant** - Works with any OCI container
- **Docker compatible** - Same CLI commands
- **Kubernetes native** - Generate k8s YAML directly

### System Integration
- **Systemd native** - Generate systemd units automatically
- **No background daemon** - Containers run as child processes
- **Resource limits** - Native cgroup integration

## Migration Guide for Users

### Quick Migration

If you're switching from Docker:

```bash
# Install Podman
sudo dnf install podman podman-compose  # Fedora/RHEL
sudo apt install podman podman-compose  # Ubuntu/Debian
brew install podman podman-compose      # macOS

# Use the same commands
podman build -t telemetry-service .
podman-compose up -d

# Or use Make
make podman
make compose-up
```

### Optional: Create Aliases

For seamless Docker compatibility:

```bash
# Add to ~/.bashrc or ~/.zshrc
alias docker='podman'
alias docker-compose='podman-compose'
```

## Command Comparison

| Task | Docker | Podman |
|------|--------|--------|
| Build image | `docker build -t myimage .` | `podman build -t myimage .` |
| Run container | `docker run myimage` | `podman run myimage` |
| List containers | `docker ps` | `podman ps` |
| View logs | `docker logs container` | `podman logs container` |
| Start services | `docker-compose up` | `podman-compose up` |
| Stop services | `docker-compose down` | `podman-compose down` |

## Makefile Commands

All container operations are available through the Makefile:

```bash
make help           # Show all commands
make podman         # Build Podman image
make podman-run     # Run container
make compose-up     # Start all services
make compose-down   # Stop all services
make compose-logs   # View logs
```

## Testing the Migration

### Build and Test

```bash
# Build the image
make podman

# Run tests
make test

# Start the full stack
make compose-up

# Check services
podman ps
podman-compose ps

# View logs
make compose-logs

# Stop services
make compose-down
```

### Verify Services

After `make compose-up`:
- Telemetry Service: `podman logs telemetry-service -f`
- Jaeger UI: http://localhost:16686
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin/admin)

## Troubleshooting

### SELinux Issues

If you encounter permission errors on Fedora/RHEL:

```bash
# Check SELinux status
getenforce

# Volumes already have :Z flag in podman-compose.yml
# If issues persist, temporarily set to permissive:
sudo setenforce 0
```

### Port Conflicts

```bash
# Check what's using a port
sudo lsof -i :9090

# Or use ss
ss -tulpn | grep 9090

# Stop conflicting service or change port in podman-compose.yml
```

### Image Pull Issues

```bash
# Explicitly pull images
podman pull docker.io/prom/prometheus:latest
podman pull docker.io/grafana/grafana:latest
podman pull docker.io/jaegertracing/all-in-one:latest
```

## Additional Resources

- [PODMAN.md](PODMAN.md) - Comprehensive Podman guide
- [README.md](README.md) - Main documentation
- [QUICKSTART.md](QUICKSTART.md) - Quick start guide

## Support

For Podman-specific questions, see:
- [Podman Documentation](https://docs.podman.io/)
- [Project PODMAN.md](PODMAN.md)
- GitHub Issues
