# WADUP Web

A VS Code-like web application for developing, building, testing, and publishing WADUP modules.

## Overview

WADUP Web provides a browser-based IDE for creating WebAssembly modules that extract metadata from files. It features:

- **Monaco Editor** with syntax highlighting (Catppuccin Macchiato theme)
- **Multi-language support**: Rust, Go, and Python modules
- **File management**: Tree view with drag-and-drop, rename, create, delete
- **Tabbed editing**: Multiple open files with unsaved changes tracking
- **Docker-based builds**: Compile modules to WebAssembly with real-time log streaming
- **Test samples**: Upload files to test modules against
- **Module publishing**: Draft/published version management, share modules with other users

## Architecture

```
wadup-web/
├── backend/           # FastAPI Python backend
│   ├── app/
│   │   ├── models/    # SQLAlchemy ORM models
│   │   ├── routers/   # API endpoints
│   │   ├── schemas/   # Pydantic schemas
│   │   ├── services/  # Business logic (build, test)
│   │   └── templates/ # Module templates (Rust, Go, Python)
│   ├── alembic/       # Database migrations
│   └── requirements.txt
├── frontend/          # React + TypeScript + Vite
│   ├── src/
│   │   ├── api/       # API client functions
│   │   ├── components/# React components
│   │   ├── stores/    # Zustand state management
│   │   ├── styles/    # Global styles (Catppuccin theme)
│   │   └── types/     # TypeScript type definitions
│   └── package.json
└── storage/           # Runtime data (gitignored)
    ├── modules/       # Module source files
    ├── artifacts/     # Built WASM files
    └── samples/       # Uploaded test samples

# Docker build images are at the project root:
../docker/
├── rust/              # Rust → wasm32-wasip1
├── go/                # Go → wasip1
├── python/            # Python → wasm32-wasi (CPython bundled)
└── test/              # WADUP test runner
```

## Prerequisites

- **Python 3.11+** with pip
- **Node.js 18+** with npm
- **Docker** (for building modules)

## Quick Start

### 1. Build Docker Images

From the **project root** (not wadup-web/):

```bash
cd /path/to/wadup2
./docker/build-images.sh
```

This creates four images:
- `wadup-build-rust:latest` - Rust compiler with wasm32-wasip1 target
- `wadup-build-go:latest` - Go compiler with WASI support
- `wadup-build-python:latest` - CPython 3.13 + WASI SDK + pre-built C extensions
- `wadup-test-runner:latest` - WADUP runtime for testing modules

### 2. Start Backend

```bash
cd backend
python -m venv .venv
source .venv/bin/activate  # or .venv\Scripts\activate on Windows
pip install -r requirements.txt

uvicorn app.main:app --host 0.0.0.0 --port 8080 --reload
```

### 3. Start Frontend

```bash
cd frontend
npm install
npm run dev
```

### 4. Open the App

Navigate to http://localhost:5173 in your browser.

## API Endpoints

### Authentication
- `POST /api/auth/login` - Login with username
- `POST /api/auth/logout` - Logout
- `GET /api/auth/me` - Get current user

### Modules
- `GET /api/modules` - List modules (supports filtering, pagination)
- `POST /api/modules` - Create new module
- `GET /api/modules/{id}` - Get module details
- `DELETE /api/modules/{id}` - Delete module
- `POST /api/modules/{id}/publish` - Publish module

### Files
- `GET /api/modules/{id}/files` - List module files as tree
- `GET /api/modules/{id}/files/{path}` - Get file content
- `PUT /api/modules/{id}/files/{path}` - Create or update file
- `DELETE /api/modules/{id}/files/{path}` - Delete file
- `POST /api/modules/{id}/files/folders/{path}` - Create folder
- `POST /api/modules/{id}/files/{path}/rename` - Rename file or folder

### Build
- `POST /api/modules/{id}/build` - Start build
- `GET /api/modules/{id}/build/status` - Get build status
- `GET /api/modules/{id}/build/stream` - Stream build logs (SSE)

### Samples
- `GET /api/samples` - List samples
- `POST /api/samples` - Upload sample file
- `DELETE /api/samples/{id}` - Delete sample

### Test
- `POST /api/modules/{id}/test` - Start test run with samples
- `GET /api/modules/{id}/test/{run_id}` - Get test run status and results
- `GET /api/modules/{id}/test/{run_id}/stream` - Stream test output (SSE)

## Module Development

### Rust Module

```rust
use wadup_guest::*;

#[no_mangle]
pub extern "C" fn process() -> i32 {
    if let Err(_) = run() { return 1; }
    0
}

fn run() -> Result<(), String> {
    let table = TableBuilder::new("my_output")
        .column("filename", DataType::String)
        .column("size_bytes", DataType::Int64)
        .build()?;

    let path = Content::path();
    let metadata = std::fs::metadata(&path)
        .map_err(|e| e.to_string())?;
    let filename = std::env::var("WADUP_FILENAME")
        .unwrap_or_else(|_| "unknown".to_string());

    table.insert(&[
        Value::String(filename),
        Value::Int64(metadata.len() as i64),
    ])?;

    flush()?;
    Ok(())
}
```

### Go Module

```go
package main

import (
    "os"
    wadup "github.com/user/wadup-guest-go"
)

//go:wasmexport process
func process() int32 {
    table, err := wadup.NewTableBuilder("my_output").
        Column("filename", wadup.String).
        Column("size_bytes", wadup.Int64).
        Build()
    if err != nil { return 1 }

    info, err := os.Stat("/data.bin")
    if err != nil { return 1 }

    filename := os.Getenv("WADUP_FILENAME")
    if filename == "" { filename = "unknown" }

    table.InsertRow([]wadup.Value{
        wadup.NewString(filename),
        wadup.NewInt64(info.Size()),
    })

    wadup.Flush()
    return 0
}

func main() {}
```

### Python Module

```python
import os
import wadup

def main():
    """Entry point called by WADUP for each file."""
    wadup.define_table("my_output", [
        ("filename", "String"),
        ("size_bytes", "Int64"),
    ])

    size = os.path.getsize("/data.bin")
    filename = os.environ.get("WADUP_FILENAME", "unknown")

    wadup.insert_row("my_output", [filename, size])
    wadup.flush()
```

## Configuration

Environment variables (can be set in `.env`):

| Variable | Default | Description |
|----------|---------|-------------|
| `WADUP_DATABASE_URL` | `sqlite:///storage/wadup.db` | Database connection string |
| `WADUP_STORAGE_ROOT` | `storage/` | Root storage directory |
| `WADUP_HOST` | `0.0.0.0` | Backend host |
| `WADUP_PORT` | `8080` | Backend port |
| `WADUP_DEBUG` | `false` | Enable debug mode |
| `WADUP_DOCKER_SOCKET` | `/var/run/docker.sock` | Docker socket path |
| `WADUP_RUST_BUILD_IMAGE` | `wadup-build-rust:latest` | Rust build image |
| `WADUP_GO_BUILD_IMAGE` | `wadup-build-go:latest` | Go build image |
| `WADUP_PYTHON_BUILD_IMAGE` | `wadup-build-python:latest` | Python build image |
| `WADUP_TEST_RUNNER_IMAGE` | `wadup-test-runner:latest` | Test runner image |
| `WADUP_BUILD_TIMEOUT` | `600` | Build timeout in seconds |
| `WADUP_TEST_TIMEOUT` | `300` | Test timeout in seconds |

## Development

### Frontend Development

```bash
cd frontend
npm run dev      # Start dev server with HMR
npm run build    # Production build
npm run lint     # Run ESLint
npm run preview  # Preview production build
```

### Backend Development

```bash
cd backend
uvicorn app.main:app --reload  # Auto-reload on changes
```

### Adding a New Language

1. Create a new directory in the project root `docker/` with:
   - `Dockerfile` - Build environment with WASM toolchain
   - `build.sh` - Script to compile source to `module.wasm`
   - Guest library with WADUP API bindings

2. Add template files in `backend/app/templates/{language}/`

3. Update `Language` enum in `backend/app/models/module.py`

4. Add image name to `backend/app/config.py`

5. Update `docker/build-images.sh` to build the new image

## License

See the main WADUP repository for license information.
