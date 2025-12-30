#!/usr/bin/env python3
"""Python WASM project builder for WADUP.

Builds Python projects with pyproject.toml into WASM modules.

Usage: ./scripts/build-python-project.py <project-dir>

The project directory must contain:
  - pyproject.toml with [project] and [tool.wadup] sections
  - src/<module_name>/__init__.py entry point

Dependencies listed in pyproject.toml are bundled if they are pure Python.
C extensions (lxml, numpy, pandas) can be enabled via [tool.wadup].c-extensions.
"""

import argparse
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

# Add parent directory to path so we can import extensions registry
sys.path.insert(0, str(Path(__file__).parent.parent))
from extensions import (
    EXTENSIONS,
    get_all_extensions,
    get_all_modules,
    get_all_libraries,
    get_all_python_dirs,
    get_validation_files,
)

# Try tomllib (Python 3.11+), fall back to tomli
try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print("ERROR: tomllib not available. Please use Python 3.11+ or install tomli.", file=sys.stderr)
        sys.exit(1)


# ANSI colors
class Colors:
    RED = '\033[0;31m'
    GREEN = '\033[0;32m'
    YELLOW = '\033[1;33m'
    BLUE = '\033[0;34m'
    NC = '\033[0m'  # No Color


def print_info(msg: str) -> None:
    print(f"{Colors.BLUE}ℹ{Colors.NC} {msg}")


def print_success(msg: str) -> None:
    print(f"{Colors.GREEN}✓{Colors.NC} {msg}")


def print_warning(msg: str) -> None:
    print(f"{Colors.YELLOW}⚠{Colors.NC} {msg}")


def print_error(msg: str) -> None:
    print(f"{Colors.RED}✗{Colors.NC} {msg}")


def parse_pyproject(project_dir: Path) -> tuple[str, str, list[str], list[str]]:
    """Parse pyproject.toml and return (name, entry_point, dependencies, c_extensions)."""
    pyproject_path = project_dir / "pyproject.toml"

    with open(pyproject_path, 'rb') as f:
        data = tomllib.load(f)

    project = data.get('project', {})
    wadup = data.get('tool', {}).get('wadup', {})

    name = project.get('name', '')
    entry_point = wadup.get('entry-point', '')
    dependencies = project.get('dependencies', [])
    c_extensions = wadup.get('c-extensions', [])

    if not name:
        print_error("[project].name not found in pyproject.toml")
        sys.exit(1)

    if not entry_point:
        print_error("[tool.wadup].entry-point not found in pyproject.toml")
        sys.exit(1)

    return name, entry_point, dependencies, c_extensions


def download_dependencies(dependencies: list[str], deps_dir: Path) -> None:
    """Download dependencies as source distributions."""
    result = subprocess.run(
        ["pip", "download", "--no-binary", ":all:", "-d", str(deps_dir)] + dependencies,
        capture_output=True,
        text=True
    )

    if result.returncode != 0:
        print_error("Failed to download dependencies as source distributions")
        print_error("Only pure-Python packages are supported (no C extensions)")
        print_error("Check that all dependencies have sdist packages available on PyPI")
        sys.exit(1)


def extract_archive(archive_path: Path, extract_dir: Path) -> None:
    """Extract a tar.gz or zip archive."""
    if archive_path.suffix == '.gz' and archive_path.stem.endswith('.tar'):
        with tarfile.open(archive_path, 'r:gz') as tf:
            tf.extractall(extract_dir, filter='data')
    elif archive_path.suffix == '.zip':
        with zipfile.ZipFile(archive_path, 'r') as zf:
            zf.extractall(extract_dir)


def copy_package_from_dir(pkg_dir: Path, bundle_dir: Path) -> None:
    """Copy Python packages from an extracted dependency directory."""
    skip_dirs = {'tests', 'test', 'docs', 'examples'}

    # Try src/ layout first (e.g., attrs uses src/attr/)
    src_dir = pkg_dir / "src"
    if src_dir.is_dir():
        for subpkg in src_dir.iterdir():
            if subpkg.is_dir() and (subpkg / "__init__.py").exists():
                dest = bundle_dir / subpkg.name
                if dest.exists():
                    shutil.rmtree(dest)
                shutil.copytree(subpkg, dest)
                print_success(f"  Added: {subpkg.name}")
    else:
        # Try flat layout (e.g., chardet uses chardet-5.2.0/chardet/)
        for subpkg in pkg_dir.iterdir():
            if not subpkg.is_dir():
                continue
            if not (subpkg / "__init__.py").exists():
                continue
            if subpkg.name in skip_dirs or subpkg.name.startswith('.'):
                continue

            dest = bundle_dir / subpkg.name
            if dest.exists():
                shutil.rmtree(dest)
            shutil.copytree(subpkg, dest)
            print_success(f"  Added: {subpkg.name}")


def extract_wheel(wheel_path: Path, bundle_dir: Path, temp_dir: Path) -> None:
    """Extract Python packages from a wheel file."""
    wheel_extract = temp_dir / "wheel_extract"
    wheel_extract.mkdir(exist_ok=True)

    with zipfile.ZipFile(wheel_path, 'r') as zf:
        zf.extractall(wheel_extract)

    for subdir in wheel_extract.iterdir():
        if not subdir.is_dir():
            continue
        # Skip metadata directories
        if subdir.name.endswith('.dist-info') or subdir.name.endswith('.data'):
            continue

        if (subdir / "__init__.py").exists():
            dest = bundle_dir / subdir.name
            if dest.exists():
                shutil.rmtree(dest)
            shutil.copytree(subdir, dest)
            print_success(f"  Added: {subdir.name}")

    shutil.rmtree(wheel_extract)


def generate_main_bundled_c(c_extensions: list[str], template_path: Path, output_path: Path) -> None:
    """Generate main_bundled.c from template with extension registrations."""
    with open(template_path, 'r') as f:
        template = f.read()

    if not c_extensions:
        # No extensions - empty placeholders
        extern_declarations = ""
        register_extensions = ""
    else:
        # Get all modules to register (resolves dependencies)
        modules = get_all_modules(c_extensions)

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
    parser = argparse.ArgumentParser(
        description="Build a Python WADUP module from a project with pyproject.toml"
    )
    parser.add_argument("project_dir", help="Path to the project directory")
    args = parser.parse_args()

    project_dir = Path(args.project_dir).resolve()

    # Detect workspace root
    script_dir = Path(__file__).parent.resolve()
    wadup_root = script_dir.parent
    deps_dir = wadup_root / "deps"

    # Validate project directory
    if not (project_dir / "pyproject.toml").exists():
        print_error(f"pyproject.toml not found in {project_dir}")
        return 1

    print_info(f"Building Python WADUP module from: {project_dir}")
    print()

    # Parse pyproject.toml
    print_info("Parsing pyproject.toml...")
    project_name, entry_module, dependencies, c_extensions = parse_pyproject(project_dir)

    print_success(f"Project: {project_name}")
    print_success(f"Entry point: {entry_module}")
    if dependencies:
        print_success(f"Dependencies: {' '.join(dependencies)}")
    if c_extensions:
        print_success(f"C extensions: {' '.join(c_extensions)}")
    print()

    # Convert project name to WASM filename (hyphens to underscores)
    wasm_name = project_name.replace('-', '_')

    # Set paths
    python_version = "3.13"
    python_dir = wadup_root / "build" / "python-wasi"
    output_dir = project_dir / "target"

    # Detect platform for WASI SDK
    os_name = platform.system().lower()
    arch = platform.machine()

    if os_name == "darwin":
        wasi_sdk_os = "macos"
    elif os_name == "linux":
        wasi_sdk_os = "linux"
    else:
        print_error(f"Unsupported OS: {os_name}")
        return 1

    wasi_sdk_version = "29.0"
    wasi_sdk_path = deps_dir / f"wasi-sdk-{wasi_sdk_version}-{arch}-{wasi_sdk_os}"
    wasi_sysroot = wasi_sdk_path / "share" / "wasi-sysroot"

    # Validate dependencies
    if not (python_dir / "lib" / f"libpython{python_version}.a").exists():
        print_error("CPython not built. Run ./scripts/build-python-wasi.sh first")
        return 1

    if not wasi_sdk_path.is_dir():
        print_error("WASI SDK not found. Run ./scripts/download-deps.sh first")
        return 1

    if not (deps_dir / "wasi-zlib" / "lib" / "libz.a").exists():
        print_error("zlib not found. Run ./scripts/download-deps.sh first")
        return 1

    # Validate C extensions using registry
    if c_extensions:
        validation_files = get_validation_files(c_extensions)
        for ext_name, files in validation_files.items():
            for validation_file in files:
                full_path = deps_dir / validation_file
                if not full_path.exists():
                    print_error(f"{ext_name} not built: {validation_file} not found")
                    print_error(f"Run ./scripts/build-{ext_name}-wasi.sh first")
                    return 1

    # Validate source directory
    source_dir = project_dir / "src" / entry_module
    if not source_dir.is_dir():
        print_error(f"Source directory not found: {source_dir}")
        print_info("Expected structure:")
        print_info(f"  {project_dir}/")
        print_info("  ├── pyproject.toml")
        print_info("  └── src/")
        print_info(f"      └── {entry_module}/")
        print_info("          └── __init__.py")
        return 1

    # Create build directory
    with tempfile.TemporaryDirectory(prefix=f"wadup-python-build-{project_name}-") as build_dir_str:
        build_dir = Path(build_dir_str)
        output_dir.mkdir(parents=True, exist_ok=True)

        print_info(f"Build directory: {build_dir}")
        print()

        # Create bundle directory structure
        bundle_dir = build_dir / "bundle"
        bundle_dir.mkdir()

        # Copy wadup library
        print_info("Bundling wadup library...")
        shutil.copytree(wadup_root / "python-wadup-guest" / "wadup", bundle_dir / "wadup")

        # Copy project source
        print_info("Bundling project source...")
        shutil.copytree(source_dir, bundle_dir / entry_module)

        # Bundle C extension Python files using registry
        if c_extensions:
            ext_python_dirs = get_all_python_dirs(c_extensions)
            for ext_python_dir in ext_python_dirs:
                # ext_python_dir is like "wasi-lxml/python/lxml"
                # We need to copy to bundle_dir using just the package name (last component)
                src_path = deps_dir / ext_python_dir
                pkg_name = src_path.name
                print_info(f"Bundling {pkg_name} Python files...")
                shutil.copytree(src_path, bundle_dir / pkg_name)

        # Handle dependencies (if any)
        if dependencies:
            print_info("Downloading dependencies (including transitive)...")
            deps_temp = build_dir / "deps"
            deps_temp.mkdir()

            print_info(f"  Dependencies: {' '.join(dependencies)}")
            download_dependencies(dependencies, deps_temp)

            # Extract dependencies
            for archive in deps_temp.iterdir():
                if archive.suffix == '.gz' or archive.suffix == '.zip':
                    print_info(f"  Extracting: {archive.name}")
                    extract_archive(archive, deps_temp)

            # Copy extracted packages to bundle
            for pkg_dir in deps_temp.iterdir():
                if not pkg_dir.is_dir():
                    continue
                if pkg_dir.suffix in ('.tar', '.gz', '.zip'):
                    continue

                copy_package_from_dir(pkg_dir, bundle_dir)

            # Also check for wheel files
            for wheel in deps_temp.glob("*.whl"):
                print_info(f"  Extracting wheel: {wheel.name}")
                extract_wheel(wheel, bundle_dir, build_dir)

        print()

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
        print()

        # Compile
        print_info("Compiling...")
        cc = wasi_sdk_path / "bin" / "clang"
        cflags = [
            "-O2",
            "-D_WASI_EMULATED_SIGNAL",
            "-D_WASI_EMULATED_GETPID",
            "-D_WASI_EMULATED_PROCESS_CLOCKS",
            f"-I{python_dir}/include",
            f"-I{build_dir}",
            "-fvisibility=default"
        ]
        ldflags = [
            "-Wl,--allow-undefined",
            "-Wl,--export=process",
            "-Wl,--initial-memory=134217728",
            "-Wl,--max-memory=268435456",
            "-Wl,--no-entry"
        ]
        wasi_emu_libs = wasi_sysroot / "lib" / "wasm32-wasip1"

        # Generate main_bundled.c from template with extension registrations
        main_c_template = wadup_root / "python-wadup-guest" / "src" / "main_bundled_template.c"
        main_c_dst = build_dir / "main_bundled.c"
        generate_main_bundled_c(c_extensions, main_c_template, main_c_dst)

        # Compile object file
        compile_cmd = [str(cc)] + cflags + ["-c", str(main_c_dst), "-o", str(build_dir / "main_bundled.o")]
        result = subprocess.run(compile_cmd, cwd=build_dir)
        if result.returncode != 0:
            print_error("Compilation failed")
            return 1

        # Find Hacl library files
        hacl_libs = list((python_dir / "lib").glob("libHacl_*.a"))

        # Build list of C extension libraries using registry
        ext_libs = []
        if c_extensions:
            ext_lib_paths = get_all_libraries(c_extensions)
            for lib_path in ext_lib_paths:
                full_path = deps_dir / lib_path
                if full_path.exists():
                    ext_libs.append(str(full_path))

        print_info("Linking...")
        link_cmd = [
            str(cc),
            *cflags,
            str(build_dir / "main_bundled.o"),
            "-o", str(build_dir / f"{wasm_name}.wasm"),
            f"-L{python_dir}/lib",
            f"-lpython{python_version}",
            str(python_dir / "lib" / "libmpdec.a"),
            str(python_dir / "lib" / "libexpat.a"),
            str(python_dir / "lib" / "libsqlite3.a"),
            *[str(lib) for lib in hacl_libs],
            *ext_libs,  # C extension libraries (lxml, numpy, pandas, etc.)
            str(deps_dir / "wasi-zlib" / "lib" / "libz.a"),
            str(deps_dir / "wasi-bzip2" / "lib" / "libbz2.a"),
            str(deps_dir / "wasi-xz" / "lib" / "liblzma.a"),
            str(wasi_emu_libs / "libwasi-emulated-signal.a"),
            str(wasi_emu_libs / "libwasi-emulated-getpid.a"),
            str(wasi_emu_libs / "libwasi-emulated-process-clocks.a"),
            "-lm",
            *ldflags
        ]

        # NumPy uses long double formatting which requires extra libc support
        if "numpy" in c_extensions:
            link_cmd.append("-lc-printscan-long-double")
        result = subprocess.run(link_cmd, cwd=build_dir)
        if result.returncode != 0:
            print_error("Linking failed")
            return 1

        # Copy to output directory
        wasm_output = build_dir / f"{wasm_name}.wasm"
        final_output = output_dir / f"{wasm_name}.wasm"
        shutil.copy(wasm_output, final_output)

    print()
    print_success("Build successful!")
    print_success(f"Output: {final_output}")

    # Show file size
    size = final_output.stat().st_size
    if size > 1024 * 1024:
        size_str = f"{size / (1024 * 1024):.1f}M"
    elif size > 1024:
        size_str = f"{size / 1024:.1f}K"
    else:
        size_str = f"{size}"
    print(f"-rw-r--r--  1  {size_str}  {final_output}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
