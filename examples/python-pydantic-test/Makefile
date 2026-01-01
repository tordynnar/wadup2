# Makefile for python-pydantic-test WASM module
# Builds a Python WADUP module with pydantic_core Rust extension support

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
	@echo "Building python-pydantic-test WASM module..."
	$(BUILD_SCRIPT) $(PROJECT_DIR)

# Clean build artifacts
clean:
	rm -rf $(TARGET_DIR)
	@echo "Cleaned python-pydantic-test"

# Check that pydantic is built
pydantic-check:
	@if [ ! -f "$(WADUP_ROOT)/deps/wasi-pydantic/lib/lib_pydantic_core.a" ]; then \
		echo "ERROR: pydantic_core not built. Run ./scripts/build-pydantic-wasi.sh first"; \
		exit 1; \
	fi

.PHONY: all build clean pydantic-check
