# Makefile for python-counter WASM module

# Configuration
PYTHON_VERSION = 3.13
PYTHON_DIR = $(CURDIR)/python-wasi

# Detect platform
UNAME_S := $(shell uname -s | tr '[:upper:]' '[:lower:]')
UNAME_M := $(shell uname -m)

ifeq ($(UNAME_S),darwin)
    WASI_SDK_OS = macos
else ifeq ($(UNAME_S),linux)
    WASI_SDK_OS = linux
else
    $(error Unsupported OS: $(UNAME_S))
endif

WASI_SDK_VERSION = 24.0
WASI_SDK_NAME = wasi-sdk-$(WASI_SDK_VERSION)-$(UNAME_M)-$(WASI_SDK_OS)
WASI_SDK_PATH = /tmp/$(WASI_SDK_NAME)

# Tools
CC = $(WASI_SDK_PATH)/bin/clang
AR = $(WASI_SDK_PATH)/bin/ar
WASM_LD = $(WASI_SDK_PATH)/bin/wasm-ld

# Compiler flags
CFLAGS = -O2 \
         -D_WASI_EMULATED_PROCESS_CLOCKS \
         -I$(PYTHON_DIR)/include \
         -fvisibility=default

# Linker flags
LDFLAGS = -Wl,--allow-undefined \
          -Wl,--export=process \
          -Wl,--initial-memory=134217728 \
          -Wl,--max-memory=268435456 \
          -Wl,--no-entry

# Source files
SRCS = src/main.c src/wadup_module.c src/signal_stubs.c
OBJS = $(SRCS:.c=.o)
TARGET = target/python_counter.wasm

# Python library paths
PYTHON_LIB = $(PYTHON_DIR)/lib/libpython$(PYTHON_VERSION).a
PYTHON_CONFIG_DIR = $(PYTHON_DIR)/lib/python$(PYTHON_VERSION)/config-$(PYTHON_VERSION)-wasm32-wasi

# Default target
all: python-check $(TARGET)

# Check if Python is built
python-check:
	@if [ ! -f "$(PYTHON_LIB)" ]; then \
		echo "ERROR: CPython not built. Run ./build-python.sh first"; \
		exit 1; \
	fi

# Embed Python script as header
src/script.py.h: src/script.py embed_script.sh
	./embed_script.sh

# Compile C files to object files
%.o: %.c src/script.py.h
	@echo "Compiling $<..."
	$(CC) $(CFLAGS) -c $< -o $@

# Link into WASM module
$(TARGET): $(OBJS) $(PYTHON_LIB)
	@echo "Linking WASM module..."
	@mkdir -p target
	$(CC) $(CFLAGS) $(OBJS) -o $@ \
		-L$(PYTHON_DIR)/lib \
		-lpython$(PYTHON_VERSION) \
		$(PYTHON_DIR)/lib/libmpdec.a \
		$(PYTHON_DIR)/lib/libexpat.a \
		$(PYTHON_DIR)/lib/libsqlite3.a \
		$(wildcard $(PYTHON_DIR)/lib/libHacl_*.a) \
		-lm \
		$(LDFLAGS)
	@echo "✓ Build successful: $(TARGET)"
	@ls -lh $(TARGET)

# Clean build artifacts
clean:
	rm -f $(OBJS) src/script.py.h
	rm -f $(TARGET)

# Full clean including Python build
distclean: clean
	rm -rf python-wasi

.PHONY: all clean distclean python-check
