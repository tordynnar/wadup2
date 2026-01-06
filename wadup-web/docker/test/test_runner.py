#!/usr/bin/env python3
"""WADUP Module Test Runner

Executes a WASM module against a sample file and captures the output.
"""
import argparse
import json
import os
import sys
import tempfile
from pathlib import Path

import wasmtime


def run_module(wasm_path: str, sample_path: str, filename: str) -> dict:
    """Run a WASM module against a sample file.

    Args:
        wasm_path: Path to the .wasm module file
        sample_path: Path to the sample file to process
        filename: Original filename to pass as WADUP_FILENAME

    Returns:
        Dict with stdout, stderr, exit_code, and metadata
    """
    # Read the sample file
    with open(sample_path, "rb") as f:
        sample_data = f.read()

    # Create a temporary directory for WASI filesystem
    with tempfile.TemporaryDirectory() as tmpdir:
        # Write sample to /data.bin (where modules expect it)
        data_path = Path(tmpdir) / "data.bin"
        data_path.write_bytes(sample_data)

        # Create directories expected by WADUP modules
        metadata_dir = Path(tmpdir) / "metadata"
        metadata_dir.mkdir(exist_ok=True)

        subcontent_dir = Path(tmpdir) / "subcontent"
        subcontent_dir.mkdir(exist_ok=True)

        # Configure WASI
        wasi_config = wasmtime.WasiConfig()
        wasi_config.env = [("WADUP_FILENAME", filename)]
        # Use inherit_stdin/stdout/stderr and preopens
        wasi_config.inherit_env()

        # Preopen the temp directory with read/write access
        wasi_config.preopen_dir(
            tmpdir, "/",
            wasmtime.DirPerms.READ_WRITE,
            wasmtime.FilePerms.READ_WRITE
        )

        # Capture stdout/stderr
        stdout_path = Path(tmpdir) / "stdout.txt"
        stderr_path = Path(tmpdir) / "stderr.txt"
        wasi_config.stdout_file = str(stdout_path)
        wasi_config.stderr_file = str(stderr_path)

        # Create engine and store
        engine = wasmtime.Engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasi_config)

        # Load and instantiate module
        try:
            module = wasmtime.Module.from_file(engine, wasm_path)
        except Exception as e:
            return {
                "success": False,
                "error": f"Failed to load module: {e}",
                "stdout": "",
                "stderr": "",
                "exit_code": -1,
                "metadata": None,
            }

        # Create linker and add WASI
        linker = wasmtime.Linker(engine)
        linker.define_wasi()

        try:
            instance = linker.instantiate(store, module)
        except Exception as e:
            return {
                "success": False,
                "error": f"Failed to instantiate module: {e}",
                "stdout": "",
                "stderr": "",
                "exit_code": -1,
                "metadata": None,
            }

        # For TinyGo/Go modules, we need to initialize the runtime
        # before calling custom wasmexport functions
        exports = instance.exports(store)

        # Check if this is a TinyGo module with custom exports
        process_func = exports.get("process")

        # Try _initialize first (reactor mode - TinyGo with -buildmode=c-shared)
        initialize_func = exports.get("_initialize")
        if initialize_func is not None:
            try:
                initialize_func(store)
            except Exception as e:
                return {
                    "success": False,
                    "error": f"Runtime initialization (_initialize) failed: {e}",
                    "stdout": "",
                    "stderr": "",
                    "exit_code": -1,
                    "metadata": None,
                }

        # Try _start if no _initialize (command mode)
        start_func = exports.get("_start")
        if initialize_func is None and process_func is not None and start_func is not None:
            try:
                start_func(store)
            except wasmtime.ExitTrap as e:
                # _start exits with code 0 after initialization
                if e.code != 0:
                    return {
                        "success": False,
                        "error": f"Runtime initialization (_start) failed with exit code {e.code}",
                        "stdout": "",
                        "stderr": "",
                        "exit_code": e.code,
                        "metadata": None,
                    }
            except Exception as e:
                # Some initialization errors are OK
                pass

        # If no process function, try _start (for pure WASI modules)
        if process_func is None:
            process_func = exports.get("_start")

        if process_func is None:
            return {
                "success": False,
                "error": "Module does not export 'process' or '_start' function",
                "stdout": "",
                "stderr": "",
                "exit_code": -1,
                "metadata": None,
            }

        # Call the function
        exit_code = 0
        error = None
        try:
            result = process_func(store)
            if result is not None:
                exit_code = int(result)
        except wasmtime.ExitTrap as e:
            exit_code = e.code
        except Exception as e:
            error = str(e)
            exit_code = -1

        # Read outputs
        stdout = stdout_path.read_text() if stdout_path.exists() else ""
        stderr = stderr_path.read_text() if stderr_path.exists() else ""

        # Read metadata from /metadata/*.json files
        metadata = None
        metadata_files = sorted(metadata_dir.glob("*.json"))
        if metadata_files:
            all_metadata = []
            for mf in metadata_files:
                try:
                    content = mf.read_text()
                    if content.strip():
                        all_metadata.append(json.loads(content))
                except json.JSONDecodeError:
                    pass
            if all_metadata:
                # If single metadata object, return it directly; otherwise return list
                metadata = all_metadata[0] if len(all_metadata) == 1 else all_metadata

        # Read subcontent files from /subcontent/
        subcontent = []
        subcontent_data_files = sorted(subcontent_dir.glob("data_*.bin"))
        for data_file in subcontent_data_files:
            # Extract index from filename (data_N.bin)
            try:
                idx = int(data_file.stem.split("_")[1])
            except (IndexError, ValueError):
                continue

            # Read the binary data (truncate if too large)
            max_size = 4096  # 4KB max for hex display
            data = data_file.read_bytes()
            truncated = len(data) > max_size
            if truncated:
                data = data[:max_size]

            # Read corresponding metadata if exists
            meta_file = subcontent_dir / f"metadata_{idx}.json"
            sub_metadata = None
            if meta_file.exists():
                try:
                    sub_metadata = json.loads(meta_file.read_text())
                except json.JSONDecodeError:
                    pass

            subcontent.append({
                "index": idx,
                "filename": sub_metadata.get("filename") if sub_metadata else None,
                "data_hex": data.hex(),
                "size": len(data_file.read_bytes()),
                "truncated": truncated,
                "metadata": sub_metadata,
            })

        return {
            "success": exit_code == 0 and error is None,
            "error": error,
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
            "metadata": metadata,
            "subcontent": subcontent if subcontent else None,
        }


def main():
    parser = argparse.ArgumentParser(description="Run WADUP module against sample")
    parser.add_argument("--module", "-m", required=True, help="Path to WASM module")
    parser.add_argument("--sample", "-s", required=True, help="Path to sample file")
    parser.add_argument("--filename", "-f", default="sample", help="Original filename")
    parser.add_argument("--output", "-o", help="Output file (default: stdout)")

    args = parser.parse_args()

    # Validate inputs
    if not os.path.exists(args.module):
        print(f"Error: Module not found: {args.module}", file=sys.stderr)
        sys.exit(1)

    if not os.path.exists(args.sample):
        print(f"Error: Sample not found: {args.sample}", file=sys.stderr)
        sys.exit(1)

    # Run the module
    result = run_module(args.module, args.sample, args.filename)

    # Output result as JSON
    output = json.dumps(result, indent=2)

    if args.output:
        with open(args.output, "w") as f:
            f.write(output)
    else:
        print(output)

    # Exit with module's exit code
    sys.exit(0 if result["success"] else 1)


if __name__ == "__main__":
    main()
