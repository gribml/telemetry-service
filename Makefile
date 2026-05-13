.PHONY: help build run test clean compose-up compose-down install release

# Default target
help:
	@echo "Telemetry Service - Available commands:"
	@echo ""
	@echo "  make build          - Build the project in debug mode"
	@echo "  make release        - Build optimized release binary"
	@echo "  make run            - Run the service locally"
	@echo "  make test           - Run all tests"
	@echo "  make clean          - Clean build artifacts"
	@echo ""
	@echo "  make compose-up     - Start monitoring stack (Prometheus, Grafana, Jaeger)"
	@echo "  make compose-down   - Stop podman-compose stack"
	@echo ""
	@echo "  make install        - Install binary to /usr/local/bin"
	@echo "  make check          - Run cargo check"
	@echo "  make fmt            - Format code"
	@echo "  make clippy         - Run clippy lints"

# Build commands
build:
	cargo build

release:
	cargo build --release
	@echo "Release binary: target/release/telemetry-service"

check:
	cargo check

# Run commands
run:
	cargo run

test:
	cargo test

# Code quality
fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

# Clean
clean:
	cargo clean
	rm -rf target/

compose-up:
	podman-compose up -d
	@echo ""
	@echo "Monitoring stack started:"
	@echo "  - Jaeger UI:         http://localhost:16686"
	@echo "  - Prometheus:        http://localhost:9090"
	@echo "  - Grafana:           http://localhost:3000 (admin/admin)"
	@echo ""
	@echo "Run the telemetry service natively: make run"

compose-down:
	podman-compose down

compose-logs:
	podman-compose logs -f

# Installation
install: release
	@echo "Installing telemetry-service to /usr/local/bin/"
	@if [ "$$(uname)" = "Darwin" ]; then \
		sudo cp target/release/telemetry-service /usr/local/bin/; \
		sudo chmod +x /usr/local/bin/telemetry-service; \
		echo "Installed successfully!"; \
		echo "To run as service: make install-macos-service"; \
	elif [ "$$(uname)" = "Linux" ]; then \
		sudo cp target/release/telemetry-service /usr/local/bin/; \
		sudo chmod +x /usr/local/bin/telemetry-service; \
		echo "Installed successfully!"; \
		echo "To run as service: make install-linux-service"; \
	fi

install-linux-service: install
	@echo "Installing systemd service..."
	sudo cp telemetry-service.service /etc/systemd/system/
	sudo systemctl daemon-reload
	sudo systemctl enable telemetry-service
	@echo "Service installed. Start with: sudo systemctl start telemetry-service"

install-macos-service: install
	@echo "Installing launchd agent..."
	mkdir -p ~/Library/LaunchAgents
	cp com.telemetry.service.plist ~/Library/LaunchAgents/
	launchctl unload ~/Library/LaunchAgents/com.telemetry.service.plist 2>/dev/null || true
	launchctl load -w ~/Library/LaunchAgents/com.telemetry.service.plist
	@echo "Agent installed and started"

uninstall:
	@if [ "$$(uname)" = "Darwin" ]; then \
		launchctl bootout gui/$$(id -u)/com.telemetry.service 2>/dev/null || true; \
		rm -f ~/Library/LaunchAgents/com.telemetry.service.plist; \
	elif [ "$$(uname)" = "Linux" ]; then \
		sudo systemctl stop telemetry-service 2>/dev/null || true; \
		sudo systemctl disable telemetry-service 2>/dev/null || true; \
		sudo rm -f /etc/systemd/system/telemetry-service.service; \
		sudo systemctl daemon-reload; \
	fi
	sudo rm -f /usr/local/bin/telemetry-service
	@echo "Uninstalled successfully"

# Development helpers
dev: fmt clippy test
	@echo "Development checks passed!"

watch:
	cargo watch -x run

bench:
	cargo bench

# Show binary size
size: release
	@ls -lh target/release/telemetry-service | awk '{print "Binary size:", $$5}'
	@file target/release/telemetry-service
