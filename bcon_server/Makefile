# Bcon Server Makefile

.PHONY: help build build-release test clean wasm native docker check fmt clippy docs run dev install uninstall

# Default target
help:
	@echo "Bcon Server Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  build         - Build debug version"
	@echo "  build-release - Build optimized release version"
	@echo "  test          - Run all tests"
	@echo "  clean         - Clean build artifacts"
	@echo "  wasm          - Build WebAssembly packages"
	@echo "  native        - Build native binary only"
	@echo "  docker        - Build Docker image"
	@echo "  check         - Run cargo check"
	@echo "  fmt           - Format code"
	@echo "  clippy        - Run clippy lints"
	@echo "  docs          - Generate documentation"
	@echo "  run           - Run debug version"
	@echo "  dev           - Run with file watching"
	@echo "  install       - Install to system"
	@echo "  uninstall     - Uninstall from system"

# Build targets
build:
	cargo build

build-release:
	cargo build --release

native: build-release

# Test targets
test:
	cargo test --all-features

test-verbose:
	cargo test --all-features -- --nocapture

# WASM targets
wasm:
	@echo "Building WASM packages..."
	@if [ ! -f "wasm-build.sh" ]; then echo "wasm-build.sh not found!"; exit 1; fi
	@chmod +x wasm-build.sh
	./wasm-build.sh

wasm-dev:
	wasm-pack build --dev --target web --out-dir pkg-dev

# Quality targets
check:
	cargo check --all-features

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-features --all-targets -- -D warnings

clippy-fix:
	cargo clippy --all-features --all-targets --fix

# Documentation
docs:
	cargo doc --all-features --no-deps --open

docs-build:
	cargo doc --all-features --no-deps

# Docker targets
docker:
	docker build -t bcon-server:latest .

docker-run:
	docker run -p 8081:8081 -p 8082:8082 bcon-server:latest

# Development targets
run:
	RUST_LOG=info cargo run

run-debug:
	RUST_LOG=debug cargo run

dev:
	@echo "Starting development server with file watching..."
	@if command -v cargo-watch >/dev/null 2>&1; then \
		RUST_LOG=debug cargo watch -x run; \
	else \
		echo "cargo-watch not installed. Install with: cargo install cargo-watch"; \
		echo "Falling back to single run..."; \
		make run-debug; \
	fi

# Configuration helpers
config-example:
	cargo run -- --generate-config example-config.json
	@echo "Example configuration generated: example-config.json"

config-validate:
	@if [ -f "config.json" ]; then \
		echo "Validating config.json..."; \
		cargo run -- --config config.json --validate; \
	else \
		echo "config.json not found. Generate one with: make config-example"; \
	fi

# Install targets (Linux/macOS)
install: build-release
	@echo "Installing bcon to /usr/local/bin..."
	@sudo cp target/release/bcon /usr/local/bin/
	@sudo chmod +x /usr/local/bin/bcon
	@echo "Installation complete. Run with: bcon"

install-user: build-release
	@echo "Installing bcon to ~/.local/bin..."
	@mkdir -p ~/.local/bin
	@cp target/release/bcon ~/.local/bin/
	@chmod +x ~/.local/bin/bcon
	@echo "Installation complete. Make sure ~/.local/bin is in your PATH"

uninstall:
	@echo "Removing bcon from /usr/local/bin..."
	@sudo rm -f /usr/local/bin/bcon
	@echo "Uninstallation complete"

uninstall-user:
	@echo "Removing bcon from ~/.local/bin..."
	@rm -f ~/.local/bin/bcon
	@echo "Uninstallation complete"

# Packaging targets
package-native: build-release
	@echo "Creating native package..."
	@mkdir -p dist/bcon-native
	@cp target/release/bcon dist/bcon-native/
	@cp README.md dist/bcon-native/
	@cp LICENSE dist/bcon-native/ 2>/dev/null || echo "LICENSE file not found, skipping..."
	@echo "#!/bin/bash" > dist/bcon-native/install.sh
	@echo "cp bcon /usr/local/bin/ && chmod +x /usr/local/bin/bcon" >> dist/bcon-native/install.sh
	@chmod +x dist/bcon-native/install.sh
	@cd dist && tar -czf bcon-native.tar.gz bcon-native/
	@echo "Native package created: dist/bcon-native.tar.gz"

package-wasm: wasm
	@echo "Creating WASM packages..."
	@mkdir -p dist
	@cd pkg && tar -czf ../dist/bcon-wasm-web.tar.gz .
	@cd pkg-node && tar -czf ../dist/bcon-wasm-node.tar.gz .
	@echo "WASM packages created:"
	@echo "  dist/bcon-wasm-web.tar.gz"
	@echo "  dist/bcon-wasm-node.tar.gz"

package-all: package-native package-wasm
	@echo "All packages created in dist/"

# Benchmarks
bench:
	cargo bench

bench-save:
	cargo bench -- --save-baseline main

# Security audit
audit:
	@if command -v cargo-audit >/dev/null 2>&1; then \
		cargo audit; \
	else \
		echo "cargo-audit not installed. Install with: cargo install cargo-audit"; \
	fi

# Clean targets
clean:
	cargo clean
	rm -rf pkg pkg-node pkg-dev dist
	rm -f example-config.json

clean-all: clean
	rm -rf target

# Utility targets
size: build-release
	@echo "Binary size analysis:"
	@ls -lh target/release/bcon
	@if command -v bloaty >/dev/null 2>&1; then \
		echo ""; \
		echo "Detailed size breakdown:"; \
		bloaty target/release/bcon; \
	fi

deps:
	cargo tree

deps-update:
	cargo update

deps-outdated:
	@if command -v cargo-outdated >/dev/null 2>&1; then \
		cargo outdated; \
	else \
		echo "cargo-outdated not installed. Install with: cargo install cargo-outdated"; \
	fi

# Pre-commit checks
pre-commit: fmt-check clippy test
	@echo "All pre-commit checks passed!"

# CI targets
ci: check fmt-check clippy test audit
	@echo "CI checks completed successfully!"

# Development setup
dev-setup:
	@echo "Installing development dependencies..."
	@cargo install cargo-watch || echo "cargo-watch already installed"
	@cargo install cargo-audit || echo "cargo-audit already installed"
	@cargo install cargo-outdated || echo "cargo-outdated already installed"
	@echo "Development setup complete!"

# Version management
version:
	@grep version Cargo.toml | head -1

version-bump-patch:
	@echo "This would bump patch version (implement with sed/awk if needed)"

version-bump-minor:
	@echo "This would bump minor version (implement with sed/awk if needed)"

# Performance profiling
profile: build-release
	@echo "Run with profiling tools like perf, valgrind, etc."
	@echo "Example: perf record ./target/release/bcon"

# Database migration (if needed in future)
migrate:
	@echo "No database migrations needed (memory-only storage)"

# Environment-specific builds
build-linux:
	cargo build --release --target x86_64-unknown-linux-gnu

build-windows:
	cargo build --release --target x86_64-pc-windows-gnu

build-macos:
	cargo build --release --target x86_64-apple-darwin

build-arm64:
	cargo build --release --target aarch64-unknown-linux-gnu

# Quick aliases
r: run
b: build
br: build-release
t: test
c: check
f: fmt
w: wasm