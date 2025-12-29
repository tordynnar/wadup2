# Makefile for python-module-test WASM module
# Builds a Python WADUP module from pyproject.toml

# Find workspace root (go up from examples/MODULE_NAME)
WADUP_ROOT := $(shell cd $(CURDIR)/../.. && pwd)

# Paths
BUILD_SCRIPT := $(WADUP_ROOT)/scripts/build-python-project.py
PROJECT_DIR := $(CURDIR)
TARGET_DIR := $(CURDIR)/target

# Default target
all: build

# Build using project build script
build:
	@echo "Building python-module-test WASM module..."
	$(BUILD_SCRIPT) $(PROJECT_DIR)

# Clean build artifacts
clean:
	rm -rf $(TARGET_DIR)
	@echo "✓ Cleaned python-module-test"

# Compatibility: allow 'make python-check' to succeed
python-check:
	@if [ ! -f "$(WADUP_ROOT)/build/python-wasi/lib/libpython3.13.a" ]; then \
		echo "ERROR: CPython not built. Run ./scripts/build-python-wasi.sh first"; \
		exit 1; \
	fi

.PHONY: all build clean python-check
