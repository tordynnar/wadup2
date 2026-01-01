MODULE_NAME := csharp-json-analyzer
WASM_NAME := csharp_json_analyzer
TARGET_DIR := $(CURDIR)/target
SOURCE_WASM := bin/Release/net8.0/wasi-wasm/CSharpJsonAnalyzer.wasm
TARGET := $(TARGET_DIR)/$(WASM_NAME).wasm

# .NET settings
DOTNET := dotnet

.PHONY: all clean restore

all: $(TARGET)

# Build C# WASM module
$(TARGET): Program.cs JsonAnalyzer.cs CSharpJsonAnalyzer.csproj ../../guest/csharp/*.cs
	@echo "Building C# WASM module with .NET $(shell $(DOTNET) --version) and Wasi.Sdk..."
	$(DOTNET) build -c Release
	@mkdir -p $(TARGET_DIR)
	@cp $(SOURCE_WASM) $(TARGET)
	@echo "✓ Build successful: $(TARGET)"
	@ls -lh $(TARGET)

clean:
	rm -rf $(TARGET_DIR) bin obj
	@echo "✓ Cleaned $(MODULE_NAME)"

restore:
	$(DOTNET) restore
