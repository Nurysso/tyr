# Kondo File Organizer - Makefile

BINARY_NAME = kondo
CONFIG_DIR = $(HOME)/.config/kondo
CONFIG_FILE = $(CONFIG_DIR)/kondo.toml
INSTALL_DIR = $(HOME)/.local/bin

.PHONY: help build install clean run test config-edit config-path config-reset uninstall

# Default target
help:
	@echo "Kondo File Organizer - Make Commands"
	@echo "===================================="
	@echo ""
	@echo "Build & Install:"
	@echo "  make build        - Build release binary"
	@echo "  make install      - Build and install to ~/.local/bin"
	@echo "  make uninstall    - Remove installed binary"
	@echo ""
	@echo "Development:"
	@echo "  make run          - Run in current directory"
	@echo "  make test         - Run tests"
	@echo "  make clean        - Clean build artifacts"
	@echo ""
	@echo "Configuration:"
	@echo "  make config-edit  - Edit config in $$EDITOR"
	@echo "  make config-path  - Show config file path"
	@echo "  make config-reset - Reset config to defaults"
	@echo "  make config-backup- Backup current config"
	@echo ""

# Build release binary
build:
	@echo "Building Kondo..."
	@cargo build --release
	@echo "✓ Build complete: target/release/$(BINARY_NAME)"

# Build and install
install: build
	@echo "Installing Kondo..."
	@mkdir -p $(INSTALL_DIR)
	@mkdir -p $(CONFIG_DIR)
	@cp target/release/$(BINARY_NAME) $(INSTALL_DIR)/$(BINARY_NAME)
	@chmod +x $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "✓ Installed to: $(INSTALL_DIR)/$(BINARY_NAME)"
	@echo "✓ Config directory: $(CONFIG_DIR)"
	@echo ""
	@echo "Run 'kondo' to start organizing!"
	@if ! echo "$$PATH" | grep -q "$(INSTALL_DIR)"; then \
		echo ""; \
		echo "Add $(INSTALL_DIR) to your PATH:"; \
		echo "   export PATH=\"\$$HOME/.local/bin:\$$PATH\""; \
	fi

# Uninstall
uninstall:
	@echo "Uninstalling Kondo..."
	@rm -f $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "✓ Binary removed"
	@echo ""
	@echo "Config still exists at: $(CONFIG_DIR)"
	@echo "To remove config: rm -rf $(CONFIG_DIR)"

# Run in current directory
run:
	@cargo run --release

# Run tests
test:
	@echo "Running tests..."
	@cargo test

# Clean build artifacts
clean:
	@echo "Cleaning..."
	@cargo clean
	@echo "✓ Clean complete"

# Edit configuration
config-edit:
	@if [ ! -f "$(CONFIG_FILE)" ]; then \
		echo "Config doesn't exist. Run 'make run' first."; \
		exit 1; \
	fi
	@$${EDITOR:-nano} $(CONFIG_FILE)

# Show config path
config-path:
	@echo "$(CONFIG_FILE)"

# View config
config-view:
	@if [ ! -f "$(CONFIG_FILE)" ]; then \
		echo "Config doesn't exist. Run 'make run' first."; \
		exit 1; \
	fi
	@cat $(CONFIG_FILE)

# Reset config to defaults
config-reset:
	@echo "Resetting config to defaults..."
	@rm -f $(CONFIG_FILE)
	@echo "✓ Config removed. Run 'kondo' to generate new defaults."

# Backup config
config-backup:
	@if [ ! -f "$(CONFIG_FILE)" ]; then \
		echo "No config to backup."; \
		exit 1; \
	fi
	@cp $(CONFIG_FILE) $(CONFIG_FILE).backup-$$(date +%Y%m%d-%H%M%S)
	@echo "✓ Config backed up"

# Check setup
check:
	@echo "Checking setup..."
	@echo ""
	@echo "Rust version:"
	@rustc --version
	@cargo --version
	@echo ""
	@echo "Binary location:"
	@which $(BINARY_NAME) 2>/dev/null || echo "  Not installed (run 'make install')"
	@echo ""
	@echo "Config file:"
	@if [ -f "$(CONFIG_FILE)" ]; then \
		echo "  ✓ $(CONFIG_FILE)"; \
	else \
		echo "  ✗ Not found (will be created on first run)"; \
	fi
	@echo ""
	@echo "PATH includes install dir:"
	@if echo "$$PATH" | grep -q "$(INSTALL_DIR)"; then \
		echo "  ✓ Yes"; \
	else \
		echo "  ✗ No - add 'export PATH=\"\$$HOME/.local/bin:\$$PATH\"'"; \
	fi

# Development build (debug)
dev:
	@cargo build
	@./target/debug/$(BINARY_NAME)

# Format code
fmt:
	@cargo fmt

# Lint code
lint:
	@cargo clippy

# Full quality check
quality: fmt lint test
	@echo "✓ All checks passed"

# Watch for changes and rebuild
watch:
	@cargo watch -x 'build --release'

# Create release package
package: build
	@echo "Creating release package..."
	@mkdir -p dist
	@cp target/release/$(BINARY_NAME) dist/
	@cp README.md dist/ 2>/dev/null || true
	@cp QUICK_REFERENCE.md dist/ 2>/dev/null || true
	@tar -czf dist/$(BINARY_NAME)-$$(uname -s)-$$(uname -m).tar.gz -C dist $(BINARY_NAME)
	@echo "✓ Package created: dist/$(BINARY_NAME)-$$(uname -s)-$$(uname -m).tar.gz"

# Show version info
version:
	@grep '^version' Cargo.toml | head -1 | cut -d'"' -f2
