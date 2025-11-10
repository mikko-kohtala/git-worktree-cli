.PHONY: build install clean test lint format check

# Default target
all: build

# Build release binary
build:
	cargo build --release

# Install binary to ~/.cargo/bin
install:
	cargo install --path .

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
