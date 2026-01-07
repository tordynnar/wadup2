#!/usr/bin/env python3
"""Python WASM module builder for WADUP (Docker version).

Builds Python projects with pyproject.toml into WASM modules.
"""

import argparse
import compileall
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

# Try tomllib (Python 3.11+), fall back to tomli
try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print("ERROR: tomllib not available.", file=sys.stderr)
        sys.exit(1)

# Add extensions to path
sys.path.insert(0, "/wadup")
from extensions import (
    get_all_modules,
    get_all_libraries,
    get_all_python_dirs,
    get_all_python_files,
)

# Paths
DEPS_DIR = Path("/wadup/deps")
PYTHON_DIR = DEPS_DIR / "wasi-python"
GUEST_DIR = Path("/wadup/guest/python")
WASI_STUBS = Path("/wadup/wasi-stubs/python")
WASI_SDK_PATH = Path(os.environ.get("WASI_SDK_PATH", "/opt/wasi-sdk"))
OUTPUT_DIR = Path("/build/output")


def print_info(msg: str) -> None:
    print(f"[INFO] {msg}")


def print_success(msg: str) -> None:
    print(f"[OK] {msg}")


def print_error(msg: str) -> None:
    print(f"[ERROR] {msg}", file=sys.stderr)


def parse_pyproject(project_dir: Path) -> tuple[str, str, list[str]]:
    """Parse pyproject.toml and return (name, entry_point, dependencies)."""
    pyproject_path = project_dir / "pyproject.toml"

    with open(pyproject_path, 'rb') as f:
        data = tomllib.load(f)

    project = data.get('project', {})
    wadup = data.get('tool', {}).get('wadup', {})

    name = project.get('name', '')
    entry_point = wadup.get('entry-point', '')
    dependencies = project.get('dependencies', [])

    if not name:
        print_error("[project].name not found in pyproject.toml")
        sys.exit(1)

    if not entry_point:
        # Default to module name with underscores
        entry_point = name.replace('-', '_')

    return name, entry_point, dependencies


def generate_main_bundled_c(template_path: Path, output_path: Path) -> None:
    """Generate main_bundled.c from template with extension registrations."""
    with open(template_path, 'r') as f:
        template = f.read()

    # Get all modules to register (lxml + pydantic)
    modules = get_all_modules()

    # Generate extern declarations
    extern_lines = []
    for module_name, init_func in modules:
        extern_lines.append(f"extern PyObject* {init_func}(void);")
    extern_declarations = "\n".join(extern_lines)

    # Generate registration code
    register_lines = []
    for module_name, init_func in modules:
        register_lines.append(f'    if (PyImport_AppendInittab("{module_name}", {init_func}) == -1) {{')
        register_lines.append(f'        fprintf(stderr, "Failed to register {module_name}\\n");')
        register_lines.append('        return 1;')
        register_lines.append('    }')
    register_extensions = "\n".join(register_lines)

    # Replace placeholders
    output = template.replace("// {{EXTERN_DECLARATIONS}}", extern_declarations)
    output = output.replace("// {{REGISTER_EXTENSIONS}}", register_extensions)

    with open(output_path, 'w') as f:
        f.write(output)


def generate_bundle_header(bundle_zip: Path, entry_module: str, output_path: Path) -> int:
    """Generate bundle.h with embedded zip data. Returns bundle size."""
    with open(bundle_zip, 'rb') as f:
        data = f.read()

    with open(output_path, 'w') as f:
        f.write('// Auto-generated bundle header\n')
        f.write('// Contains embedded Python modules as a zip file\n\n')
        f.write(f'#define ENTRY_MODULE "{entry_module}"\n\n')
        f.write(f'static const size_t BUNDLE_SIZE = {len(data)};\n\n')
        f.write('static const unsigned char BUNDLE_DATA[] = {\n')

        # Write bytes in rows of 16
        for i in range(0, len(data), 16):
            chunk = data[i:i+16]
            hex_vals = ', '.join(f'0x{b:02x}' for b in chunk)
            f.write(f'    {hex_vals},\n')

        f.write('};\n')

    return len(data)


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a Python WADUP module")
    parser.add_argument("project_dir", help="Path to the project directory")
    args = parser.parse_args()

    project_dir = Path(args.project_dir).resolve()
    python_version = "3.13"

    # Validate project directory
    if not (project_dir / "pyproject.toml").exists():
        print_error(f"pyproject.toml not found in {project_dir}")
        return 1

    print_info(f"Building Python WADUP module from: {project_dir}")

    # Parse pyproject.toml
    print_info("Parsing pyproject.toml...")
    project_name, entry_module, dependencies = parse_pyproject(project_dir)

    print_success(f"Project: {project_name}")
    print_success(f"Entry point: {entry_module}")

    # Convert project name to WASM filename (hyphens to underscores)
    wasm_name = project_name.replace('-', '_')

    # Create build directory
    with tempfile.TemporaryDirectory(prefix=f"wadup-python-build-") as build_dir_str:
        build_dir = Path(build_dir_str)
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

        print_info(f"Build directory: {build_dir}")

        # Create bundle directory structure
        bundle_dir = build_dir / "bundle"
        bundle_dir.mkdir()

        # Copy wadup library
        print_info("Bundling wadup library...")
        shutil.copytree(GUEST_DIR / "wadup", bundle_dir / "wadup")

        # Copy project source - try both src/ layout and flat layout
        print_info("Bundling project source...")
        source_dir = project_dir / "src" / entry_module
        if source_dir.is_dir():
            shutil.copytree(source_dir, bundle_dir / entry_module)
        elif (project_dir / entry_module).is_dir():
            shutil.copytree(project_dir / entry_module, bundle_dir / entry_module)
        else:
            print_error(f"Source directory not found: {source_dir}")
            return 1

        # Install pure Python dependencies if any
        if dependencies:
            print_info(f"Installing dependencies: {dependencies}")
            deps_dir = build_dir / "pip_deps"
            deps_dir.mkdir()
            pip_cmd = [
                sys.executable, "-m", "pip", "install",
                "--target", str(deps_dir),
                "--quiet",
                *dependencies
            ]
            result = subprocess.run(pip_cmd, capture_output=True, text=True)
            if result.returncode != 0:
                print_error(f"Failed to install dependencies: {result.stderr}")
                return 1

            # Bundle installed packages (skip metadata directories)
            for item in deps_dir.iterdir():
                if item.is_dir() and not item.name.endswith('.dist-info') and not item.name.endswith('.egg-info'):
                    dest = bundle_dir / item.name
                    if not dest.exists():
                        print_info(f"Bundling dependency: {item.name}")
                        shutil.copytree(item, dest)
                elif item.is_file() and item.suffix == '.py':
                    dest = bundle_dir / item.name
                    if not dest.exists():
                        print_info(f"Bundling dependency file: {item.name}")
                        shutil.copy(item, dest)

        # Bundle C extension Python files (lxml + pydantic always included)
        ext_python_dirs = get_all_python_dirs()
        for ext_python_dir in ext_python_dirs:
            src_path = DEPS_DIR / ext_python_dir
            pkg_name = src_path.name
            if src_path.exists():
                print_info(f"Bundling {pkg_name} Python files...")
                shutil.copytree(src_path, bundle_dir / pkg_name)

        # Copy single-file Python modules
        ext_python_files = get_all_python_files()
        for ext_python_file in ext_python_files:
            src_path = DEPS_DIR / ext_python_file
            if src_path.exists():
                print_info(f"Bundling {src_path.name}...")
                shutil.copy(src_path, bundle_dir / src_path.name)

        # Copy WASI Python stubs
        if WASI_STUBS.is_dir():
            for stub_file in WASI_STUBS.glob("*.py"):
                shutil.copy(stub_file, bundle_dir / stub_file.name)
            for stub_pkg in WASI_STUBS.iterdir():
                if stub_pkg.is_dir():
                    dest_pkg = bundle_dir / stub_pkg.name
                    if dest_pkg.exists():
                        for item in stub_pkg.rglob('*'):
                            if item.is_file():
                                rel_path = item.relative_to(stub_pkg)
                                dest_file = dest_pkg / rel_path
                                dest_file.parent.mkdir(parents=True, exist_ok=True)
                                shutil.copy(item, dest_file)
                    else:
                        shutil.copytree(stub_pkg, dest_pkg)

        # Pre-compile all Python files to .pyc
        print_info("Pre-compiling Python files...")
        compileall.compile_dir(bundle_dir, force=True, quiet=1, legacy=True)

        # Count compiled files
        pyc_count = len(list(bundle_dir.rglob('*.pyc')))
        print_success(f"Pre-compiled {pyc_count} Python files")

        # Remove .py files to force Python to use .pyc files
        print_info("Removing .py source files (keeping only .pyc)...")
        py_removed = 0
        for py_file in list(bundle_dir.rglob('*.py')):
            pyc_file = py_file.with_suffix('.pyc')
            if pyc_file.exists():
                py_file.unlink()
                py_removed += 1
        print_success(f"Removed {py_removed} .py files")

        # Create zip bundle
        print_info("Creating bundle.zip...")
        bundle_zip = build_dir / "bundle.zip"
        with zipfile.ZipFile(bundle_zip, 'w', zipfile.ZIP_DEFLATED) as zf:
            for file_path in bundle_dir.rglob('*'):
                if file_path.is_file():
                    arcname = file_path.relative_to(bundle_dir)
                    zf.write(file_path, arcname)

        bundle_size = bundle_zip.stat().st_size
        print_success(f"Bundle size: {bundle_size} bytes")

        # Generate bundle.h
        print_info("Generating bundle.h...")
        bundle_h = build_dir / "bundle.h"
        generate_bundle_header(bundle_zip, entry_module, bundle_h)

        # Compile
        print_info("Compiling...")
        cc = WASI_SDK_PATH / "bin" / "clang"
        wasi_sysroot = WASI_SDK_PATH / "share" / "wasi-sysroot"

        cflags = [
            "-O2",
            "-D_WASI_EMULATED_SIGNAL",
            "-D_WASI_EMULATED_GETPID",
            "-D_WASI_EMULATED_PROCESS_CLOCKS",
            f"-I{PYTHON_DIR}/include",
            f"-I{build_dir}",
            "-fvisibility=default"
        ]
        ldflags = [
            "-Wl,--allow-undefined",
            "-Wl,--export=process",
            "-Wl,--initial-memory=134217728",  # 128 MB
            "-Wl,--max-memory=268435456",      # 256 MB
            "-Wl,--no-entry",
            "-z", "stack-size=8388608",        # 8 MB stack
            "-Wl,--stack-first",
        ]
        wasi_emu_libs = wasi_sysroot / "lib" / "wasm32-wasip1"

        # Generate main_bundled.c from template
        main_c_template = GUEST_DIR / "src" / "main_bundled_template.c"
        main_c_dst = build_dir / "main_bundled.c"
        generate_main_bundled_c(main_c_template, main_c_dst)

        # Compile object file
        compile_cmd = [str(cc)] + cflags + ["-c", str(main_c_dst), "-o", str(build_dir / "main_bundled.o")]
        result = subprocess.run(compile_cmd, cwd=build_dir)
        if result.returncode != 0:
            print_error("Compilation failed")
            return 1

        # Find Hacl library files
        hacl_libs = list((PYTHON_DIR / "lib").glob("libHacl_*.a"))

        # Build list of C extension libraries (lxml + pydantic always included)
        ext_libs = []
        ext_lib_paths = get_all_libraries()
        for lib_path in ext_lib_paths:
            full_path = DEPS_DIR / lib_path
            if full_path.exists():
                ext_libs.append(str(full_path))

        print_info("Linking...")
        link_cmd = [
            str(cc),
            *cflags,
            str(build_dir / "main_bundled.o"),
            "-o", str(build_dir / "module.wasm"),
            f"-L{PYTHON_DIR}/lib",
            f"-lpython{python_version}",
            str(PYTHON_DIR / "lib" / "libmpdec.a"),
            str(PYTHON_DIR / "lib" / "libexpat.a"),
            str(PYTHON_DIR / "lib" / "libsqlite3.a"),
            *[str(lib) for lib in hacl_libs],
            *ext_libs,
            str(DEPS_DIR / "wasi-zlib" / "lib" / "libz.a"),
            str(DEPS_DIR / "wasi-bzip2" / "lib" / "libbz2.a"),
            str(DEPS_DIR / "wasi-xz" / "lib" / "liblzma.a"),
            str(wasi_emu_libs / "libwasi-emulated-signal.a"),
            str(wasi_emu_libs / "libwasi-emulated-getpid.a"),
            str(wasi_emu_libs / "libwasi-emulated-process-clocks.a"),
            "-lm",
            *ldflags
        ]

        result = subprocess.run(link_cmd, cwd=build_dir)
        if result.returncode != 0:
            print_error("Linking failed")
            return 1

        # Copy to output directory
        wasm_output = build_dir / "module.wasm"
        final_output = OUTPUT_DIR / "module.wasm"
        shutil.copy(wasm_output, final_output)

    print_success("Build successful!")
    print_success(f"Output: {final_output}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
