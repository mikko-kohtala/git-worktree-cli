.PHONY: build install clean test lint format check

# Default target
all: build

# Build release binary
build:
	cargo build --release

# Install binary to ~/.cargo/bin (or /usr/local/bin with sudo)
install: build
	@if [ -d "$$HOME/.cargo/bin" ]; then \
		install -m 755 target/release/gwt "$$HOME/.cargo/bin/gwt"; \
		echo "Installed to $$HOME/.cargo/bin/gwt"; \
	else \
		sudo install -m 755 target/release/gwt /usr/local/bin/gwt; \
		echo "Installed to /usr/local/bin/gwt"; \
	fi

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test

# Run linter
lint:
	cargo clippy -- -D warnings

# Format code
format:
	cargo fmt

# Type checking
check:
	cargo check
