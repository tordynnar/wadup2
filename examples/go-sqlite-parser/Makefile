# Makefile for go-sqlite-parser WASM module
# Demonstrates using pure Go SQLite (modernc.org/sqlite) with WADUP

MODULE_NAME := go-sqlite-parser
WASM_NAME := go_sqlite_parser
WADUP_ROOT := $(shell cd ../.. && pwd)
TARGET_DIR := $(CURDIR)/target
TARGET := $(TARGET_DIR)/$(WASM_NAME).wasm

# Standard Go build settings for WASI
GO := go
GOOS := wasip1
GOARCH := wasm

.PHONY: all clean

all: $(TARGET)

# Build Go WASM module with standard Go (pure Go SQLite, no CGO)
$(TARGET): main.go wadup/*.go go.mod
	@mkdir -p $(TARGET_DIR)
	@echo "Building Go WASM module with standard Go (modernc.org/sqlite - pure Go)..."
	@echo "  Go version: $(shell $(GO) version)"
	@echo "  Target: $(GOOS)/$(GOARCH)"
	@$(GO) mod download
	GOOS=$(GOOS) GOARCH=$(GOARCH) $(GO) build -o $(TARGET) .
	@echo "✓ Build successful: $(TARGET)"
	@ls -lh $(TARGET)

clean:
	rm -rf $(TARGET_DIR)
	@echo "✓ Cleaned $(MODULE_NAME)"
