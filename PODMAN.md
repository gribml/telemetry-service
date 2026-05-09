# Podman Deployment Guide

This project is optimized for Podman, a daemonless container engine that's rootless by default and OCI-compliant.

## Why Podman?

- **Daemonless**: No background daemon required
- **Rootless**: Run containers without root privileges
- **Docker Compatible**: Drop-in replacement for Docker commands
- **Systemd Integration**: Native systemd support for container management
- **Security**: Enhanced security through user namespaces
- **Pods**: Native support for Kubernetes-style pods

## Installation

### Fedora/RHEL/CentOS
```bash
sudo dnf install podman podman-compose
```

### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install podman podman-compose
```

### macOS
```bash
brew install podman podman-compose

# Initialize and start Podman machine
podman machine init
podman machine start
```

### Verify Installation
```bash
podman --version
podman-compose --version
```

## Building the Image

### Standard Build
```bash
podman build -t telemetry-service:latest .
```

### Build with Makefile
```bash
make podman
```

### View Built Images
```bash
podman images
```

## Running the Container

### Interactive Mode
```bash
podman run -it --rm telemetry-service:latest
```

### Detached Mode
```bash
podman run -d --name telemetry-service telemetry-service:latest
```

### With Environment Variables
```bash
podman run -d \
  --name telemetry-service \
  -e OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317 \
  telemetry-service:latest
```

### View Logs
```bash
podman logs telemetry-service -f
```

## Podman Compose

### Start All Services
```bash
# Using podman-compose
podman-compose up -d

# Or using Makefile
make compose-up
```

### Check Service Status
```bash
podman-compose ps
```

### View Logs
```bash
# All services
podman-compose logs -f

# Specific service
podman logs telemetry-service -f

# Or using Makefile
make compose-logs
```

### Stop Services
```bash
# Using podman-compose
podman-compose down

# Or using Makefile
make compose-down
```

## Rootless Containers

Podman runs rootless by default, which means:

1. **No sudo required** for container operations
2. **Enhanced security** through user namespaces
3. **Process isolation** from host system
4. **User-specific containers** and images

### Checking Rootless Status
```bash
podman info | grep rootless
```

### Volume Permissions with SELinux

When mounting volumes on SELinux-enabled systems (Fedora, RHEL), use the `:Z` flag:

```bash
podman run -v ./data:/data:Z myimage
```

This is already configured in `podman-compose.yml`:
```yaml
volumes:
  - ./prometheus:/etc/prometheus:Z
  - prometheus-data:/prometheus:Z
```

## Systemd Integration

### Generate Systemd Unit File

Podman can generate systemd service files for containers:

```bash
# Run container
podman run -d --name telemetry-service telemetry-service:latest

# Generate systemd unit
podman generate systemd --new --name telemetry-service > ~/.config/systemd/user/telemetry-service.service

# Enable and start
systemctl --user enable telemetry-service
systemctl --user start telemetry-service
```

### System-wide Service (as root)

```bash
# Generate unit file for root
sudo podman generate systemd --new --name telemetry-service > /etc/systemd/system/telemetry-service.service

# Enable and start
sudo systemctl enable telemetry-service
sudo systemctl start telemetry-service
```

### Check Status
```bash
# User service
systemctl --user status telemetry-service

# System service
sudo systemctl status telemetry-service
```

## Networking

### List Networks
```bash
podman network ls
```

### Inspect Network
```bash
podman network inspect monitoring
```

### Create Custom Network
```bash
podman network create mynetwork
```

### Run Container in Network
```bash
podman run -d --network monitoring telemetry-service:latest
```

## Volumes

### List Volumes
```bash
podman volume ls
```

### Inspect Volume
```bash
podman volume inspect prometheus-data
```

### Create Volume
```bash
podman volume create mydata
```

### Remove Unused Volumes
```bash
podman volume prune
```

## Pods (Kubernetes-style)

Podman supports pods, which are groups of containers sharing network and storage:

```bash
# Create a pod
podman pod create --name monitoring -p 9090:9090 -p 3000:3000

# Run containers in the pod
podman run -d --pod monitoring docker.io/prom/prometheus
podman run -d --pod monitoring docker.io/grafana/grafana
podman run -d --pod monitoring telemetry-service:latest

# Manage the pod
podman pod ps
podman pod stop monitoring
podman pod start monitoring
```

## Security Features

### Running as Non-Root User

The Dockerfile already configures a non-root user:
```dockerfile
RUN useradd -m -u 1000 telemetry
USER telemetry
```

### Security Scanning
```bash
# Scan image for vulnerabilities
podman scan telemetry-service:latest
```

### Read-Only Root Filesystem
```bash
podman run --read-only --tmpfs /tmp telemetry-service:latest
```

### Drop Capabilities
```bash
podman run --cap-drop=ALL --cap-add=NET_BIND_SERVICE telemetry-service:latest
```

## Kubernetes Integration

### Generate Kubernetes YAML
```bash
podman generate kube telemetry-service > telemetry-k8s.yaml
```

### Deploy to Kubernetes
```bash
kubectl apply -f telemetry-k8s.yaml
```

## Troubleshooting

### Container Won't Start
```bash
# Check logs
podman logs telemetry-service

# Inspect container
podman inspect telemetry-service

# Check events
podman events --filter container=telemetry-service
```

### Volume Permission Issues

If you encounter permission issues with volumes:

1. Use the `:Z` flag for SELinux systems
2. Check ownership: `ls -l /path/to/volume`
3. Run with `--userns=keep-id` for user namespace mapping

```bash
podman run --userns=keep-id -v ./data:/data:Z telemetry-service
```

### Network Connectivity Issues

```bash
# Test network connectivity
podman exec telemetry-service ping jaeger

# Check DNS
podman exec telemetry-service nslookup jaeger

# Restart network
podman network reload monitoring
```

### SELinux Denials

Check for SELinux issues:
```bash
sudo ausearch -m avc -ts recent
```

Set SELinux to permissive (temporary):
```bash
sudo setenforce 0
```

### Port Already in Use

```bash
# Find what's using the port
sudo lsof -i :9090

# Or use ss
ss -tulpn | grep 9090

# Kill the process or change the port mapping
podman run -p 9091:9090 telemetry-service
```

## Performance Tips

### Use Overlay Storage Driver
```bash
# Check current driver
podman info | grep graphDriverName

# Overlay2 is typically the fastest
```

### Limit Resources
```bash
podman run \
  --memory=256m \
  --cpus=0.5 \
  telemetry-service:latest
```

### Clean Up Regularly
```bash
# Remove unused containers
podman container prune

# Remove unused images
podman image prune

# Remove unused volumes
podman volume prune

# Remove everything unused
podman system prune -a
```

## Migration from Docker

Podman is designed to be Docker-compatible:

```bash
# Create aliases
alias docker=podman
alias docker-compose=podman-compose

# Add to ~/.bashrc or ~/.zshrc
echo "alias docker=podman" >> ~/.bashrc
echo "alias docker-compose=podman-compose" >> ~/.bashrc
```

Most Docker commands work directly with Podman:
- `docker run` → `podman run`
- `docker build` → `podman build`
- `docker ps` → `podman ps`
- `docker-compose up` → `podman-compose up`

## References

- [Podman Documentation](https://docs.podman.io/)
- [Podman Compose](https://github.com/containers/podman-compose)
- [Rootless Containers](https://rootlesscontaine.rs/)
- [OCI Specifications](https://opencontainers.org/)

## Support

For Podman-specific issues:
- GitHub Issues: https://github.com/containers/podman/issues
- Discussions: https://github.com/containers/podman/discussions
- IRC: #podman on Libera.Chat
