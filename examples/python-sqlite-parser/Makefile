# Makefile for python-sqlite-parser WASM module
# This is a thin wrapper around the shared build script

# Detect module name from directory
MODULE_NAME := $(notdir $(CURDIR))

# Find workspace root (go up from examples/MODULE_NAME)
WADUP_ROOT := $(shell cd $(CURDIR)/../.. && pwd)

# Paths
BUILD_SCRIPT := $(WADUP_ROOT)/scripts/build-python-module.sh
SCRIPT_PY := $(CURDIR)/src/script.py
TARGET_DIR := $(CURDIR)/target
TARGET := $(TARGET_DIR)/$(subst -,_,$(MODULE_NAME)).wasm

# Default target
all: $(TARGET)

# Build using shared script
$(TARGET): $(SCRIPT_PY)
	@echo "Building $(MODULE_NAME) WASM module..."
	@mkdir -p $(TARGET_DIR)
	$(BUILD_SCRIPT) $(MODULE_NAME) $(SCRIPT_PY) $(TARGET_DIR)

# Clean build artifacts
clean:
	rm -f $(TARGET)
	@echo "✓ Cleaned $(MODULE_NAME)"

# Compatibility: allow 'make python-check' to succeed
python-check:
	@if [ ! -f "$(WADUP_ROOT)/build/python-wasi/lib/libpython3.13.a" ]; then \
		echo "ERROR: CPython not built. Run ./scripts/build-python-wasi.sh first"; \
		exit 1; \
	fi

.PHONY: all clean python-check
