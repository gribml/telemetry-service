# Build stage
FROM docker.io/library/rust:1.93 as builder

WORKDIR /usr/src/telemetry-service

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Update cargo
RUN cargo update

# Build release binary
RUN cargo build --release

# Runtime stage
FROM docker.io/library/debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 telemetry

# Copy binary from builder
COPY --from=builder /usr/src/telemetry-service/target/release/telemetry-service /usr/local/bin/

# Switch to non-root user
USER telemetry

# Set working directory
WORKDIR /home/telemetry

# Run the service
CMD ["telemetry-service"]
