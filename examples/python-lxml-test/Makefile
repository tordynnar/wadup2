# Makefile for python-lxml-test WASM module
# Builds a Python WADUP module with lxml C extension support

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
	@echo "Building python-lxml-test WASM module..."
	$(BUILD_SCRIPT) $(PROJECT_DIR)

# Clean build artifacts
clean:
	rm -rf $(TARGET_DIR)
	@echo "Cleaned python-lxml-test"

# Check that lxml is built
lxml-check:
	@if [ ! -f "$(WADUP_ROOT)/deps/wasi-lxml/lib/liblxml_etree.a" ]; then \
		echo "ERROR: lxml not built. Run ./scripts/build-lxml-wasi.sh first"; \
		exit 1; \
	fi

.PHONY: all build clean lxml-check
