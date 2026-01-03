#!/bin/bash
# Build pydantic_core Rust extension and bundle full pydantic library for WASI
# Dependencies must be downloaded first with ./scripts/download-deps.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPS_DIR="$WADUP_ROOT/deps"

# Versions - pydantic 2.12.5 requires pydantic_core 2.41.5
PYDANTIC_CORE_VERSION="2.41.5"
PYDANTIC_VERSION="2.12.5"
TYPING_EXTENSIONS_VERSION="4.15.0"
ANNOTATED_TYPES_VERSION="0.7.0"
TYPING_INSPECTION_VERSION="0.4.2"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$OS" = "darwin" ]; then
    WASI_SDK_OS="macos"
elif [ "$OS" = "linux" ]; then
    WASI_SDK_OS="linux"
else
    echo "ERROR: Unsupported OS: $OS"
    exit 1
fi

WASI_SDK_VERSION="24.0"
WASI_SDK_PATH="$DEPS_DIR/wasi-sdk-${WASI_SDK_VERSION}-${ARCH}-${WASI_SDK_OS}"

echo "=== Building pydantic_core ${PYDANTIC_CORE_VERSION} for WASI ==="

# Check dependencies
if [ ! -f "$DEPS_DIR/wasi-python/lib/libpython3.13.a" ]; then
    echo "ERROR: Python WASI not built. Run ./scripts/build-python-wasi.sh first"
    exit 1
fi

if [ ! -d "$WASI_SDK_PATH" ]; then
    echo "ERROR: WASI SDK not found. Run ./scripts/download-deps.sh first"
    exit 1
fi

# Check source archives exist
PYDANTIC_CORE_ARCHIVE="$DEPS_DIR/pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
if [ ! -f "$PYDANTIC_CORE_ARCHIVE" ]; then
    echo "ERROR: pydantic_core source not found at $PYDANTIC_CORE_ARCHIVE"
    echo "Run ./scripts/download-deps.sh first"
    exit 1
fi

PYDANTIC_ARCHIVE="$DEPS_DIR/pydantic-${PYDANTIC_VERSION}.tar.gz"
if [ ! -f "$PYDANTIC_ARCHIVE" ]; then
    echo "ERROR: pydantic source not found at $PYDANTIC_ARCHIVE"
    echo "Run ./scripts/download-deps.sh first"
    exit 1
fi

TYPING_EXT_ARCHIVE="$DEPS_DIR/typing_extensions-${TYPING_EXTENSIONS_VERSION}.tar.gz"
if [ ! -f "$TYPING_EXT_ARCHIVE" ]; then
    echo "ERROR: typing_extensions not found at $TYPING_EXT_ARCHIVE"
    echo "Run ./scripts/download-deps.sh first"
    exit 1
fi

ANNOTATED_TYPES_ARCHIVE="$DEPS_DIR/annotated_types-${ANNOTATED_TYPES_VERSION}.tar.gz"
if [ ! -f "$ANNOTATED_TYPES_ARCHIVE" ]; then
    echo "ERROR: annotated_types not found at $ANNOTATED_TYPES_ARCHIVE"
    echo "Run ./scripts/download-deps.sh first"
    exit 1
fi

TYPING_INSPECTION_ARCHIVE="$DEPS_DIR/typing_inspection-${TYPING_INSPECTION_VERSION}.tar.gz"
if [ ! -f "$TYPING_INSPECTION_ARCHIVE" ]; then
    echo "ERROR: typing_inspection not found at $TYPING_INSPECTION_ARCHIVE"
    echo "Run ./scripts/download-deps.sh first"
    exit 1
fi

# Check if already built (check for both library AND full pydantic)
if [ -f "$DEPS_DIR/wasi-pydantic/lib/lib_pydantic_core.a" ] && [ -d "$DEPS_DIR/wasi-pydantic/python/pydantic" ]; then
    echo "pydantic already built"
    exit 0
fi

# Create directories
mkdir -p "$DEPS_DIR/wasi-pydantic/lib"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/pydantic_core"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/pydantic"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/annotated_types"
mkdir -p "$DEPS_DIR/wasi-pydantic/python/typing_inspection"

# Extract all archives
echo "Extracting archives..."
cd "$DEPS_DIR"
rm -rf "pydantic_core-${PYDANTIC_CORE_VERSION}"
rm -rf "pydantic-core-${PYDANTIC_CORE_VERSION}"
rm -rf "pydantic-${PYDANTIC_VERSION}"
rm -rf "typing_extensions-${TYPING_EXTENSIONS_VERSION}"
rm -rf "annotated_types-${ANNOTATED_TYPES_VERSION}"
rm -rf "typing_inspection-${TYPING_INSPECTION_VERSION}"

tar xzf "pydantic_core-${PYDANTIC_CORE_VERSION}.tar.gz"
tar xzf "pydantic-${PYDANTIC_VERSION}.tar.gz"
tar xzf "typing_extensions-${TYPING_EXTENSIONS_VERSION}.tar.gz"
tar xzf "annotated_types-${ANNOTATED_TYPES_VERSION}.tar.gz"
tar xzf "typing_inspection-${TYPING_INSPECTION_VERSION}.tar.gz"

# Handle different folder naming conventions (pydantic-core vs pydantic_core)
if [ -d "pydantic_core-${PYDANTIC_CORE_VERSION}" ]; then
    cd "pydantic_core-${PYDANTIC_CORE_VERSION}"
elif [ -d "pydantic-core-${PYDANTIC_CORE_VERSION}" ]; then
    cd "pydantic-core-${PYDANTIC_CORE_VERSION}"
else
    echo "ERROR: Could not find pydantic_core source directory"
    exit 1
fi

# Patch Cargo.toml for WASI static linking
echo "Patching Cargo.toml for WASI..."
cp Cargo.toml Cargo.toml.orig

# Remove generate-import-lib feature (not needed for static linking)
sed -i.bak 's/"generate-import-lib", //' Cargo.toml

# Change crate-type from cdylib to staticlib
sed -i.bak 's/crate-type = \["cdylib", "rlib"\]/crate-type = ["staticlib", "rlib"]/' Cargo.toml

# Add workspace isolation
echo '' >> Cargo.toml
echo '# Keep out of parent workspace' >> Cargo.toml
echo '[workspace]' >> Cargo.toml

# Create PyO3 config file for cross-compilation
cat > pyo3-wasi-config.txt << EOF
implementation=CPython
version=3.13
shared=false
abi3=false
lib_name=python3.13
lib_dir=$DEPS_DIR/wasi-python/lib
pointer_width=32
build_flags=
suppress_build_script_link_lines=true
EOF

echo "Building Rust library for wasm32-wasip1..."
export PYO3_CONFIG_FILE="$(pwd)/pyo3-wasi-config.txt"
export CARGO_TARGET_WASM32_WASIP1_LINKER="${WASI_SDK_PATH}/bin/wasm-ld"

# Build with cargo
cargo build --target wasm32-wasip1 --release 2>&1

# Check build succeeded
if [ ! -f "target/wasm32-wasip1/release/lib_pydantic_core.a" ]; then
    echo "ERROR: Rust build failed"
    exit 1
fi

echo "Copying library..."
cp "target/wasm32-wasip1/release/lib_pydantic_core.a" "$DEPS_DIR/wasi-pydantic/lib/"

# Copy Python files
echo "Copying Python files..."

# pydantic_core - copy original files and core_schema.py
cp python/pydantic_core/__init__.py "$DEPS_DIR/wasi-pydantic/python/pydantic_core/"
cp python/pydantic_core/core_schema.py "$DEPS_DIR/wasi-pydantic/python/pydantic_core/"

# Create pydantic_core __init__.py that includes core_schema exports
cat > "$DEPS_DIR/wasi-pydantic/python/pydantic_core/__init__.py" << 'PYEOF'
"""pydantic_core - Core validation library for Pydantic V2."""
from __future__ import annotations

import sys as _sys
from typing import Any as _Any

from typing_extensions import Sentinel

from _pydantic_core import (
    ArgsKwargs,
    MultiHostUrl,
    PydanticCustomError,
    PydanticKnownError,
    PydanticOmit,
    PydanticSerializationError,
    PydanticSerializationUnexpectedValue,
    PydanticUndefined,
    PydanticUndefinedType,
    PydanticUseDefault,
    SchemaError,
    SchemaSerializer,
    SchemaValidator,
    Some,
    TzInfo,
    Url,
    ValidationError,
    __version__,
    from_json,
    to_json,
    to_jsonable_python,
)

# Import core_schema for full pydantic compatibility
from . import core_schema
from .core_schema import CoreConfig, CoreSchema, CoreSchemaType, ErrorType

if _sys.version_info < (3, 11):
    from typing_extensions import NotRequired as _NotRequired
else:
    from typing import NotRequired as _NotRequired

if _sys.version_info < (3, 12):
    from typing_extensions import TypedDict as _TypedDict
else:
    from typing import TypedDict as _TypedDict

# Sentinel values
UNSET: Sentinel = Sentinel('UNSET')
MISSING: Sentinel = Sentinel('MISSING')

__all__ = [
    '__version__',
    'MISSING',
    'UNSET',
    'CoreConfig',
    'CoreSchema',
    'CoreSchemaType',
    'SchemaValidator',
    'SchemaSerializer',
    'Some',
    'Url',
    'MultiHostUrl',
    'ArgsKwargs',
    'PydanticUndefined',
    'PydanticUndefinedType',
    'SchemaError',
    'ErrorDetails',
    'InitErrorDetails',
    'ValidationError',
    'PydanticCustomError',
    'PydanticKnownError',
    'PydanticOmit',
    'PydanticUseDefault',
    'PydanticSerializationError',
    'PydanticSerializationUnexpectedValue',
    'TzInfo',
    'to_json',
    'from_json',
    'to_jsonable_python',
    'core_schema',
]


class ErrorDetails(_TypedDict):
    type: str
    loc: tuple[int | str, ...]
    msg: str
    input: _Any
    ctx: _NotRequired[dict[str, _Any]]
    url: _NotRequired[str]


class InitErrorDetails(_TypedDict):
    type: str | PydanticCustomError
    loc: _NotRequired[tuple[int | str, ...]]
    input: _Any
    ctx: _NotRequired[dict[str, _Any]]
PYEOF

# Clean up build artifacts
rm -rf target

# Go back to deps dir for copying Python packages
cd "$DEPS_DIR"

# Copy typing_extensions (single file module)
echo "Copying typing_extensions..."
cp "typing_extensions-${TYPING_EXTENSIONS_VERSION}/src/typing_extensions.py" \
   "$DEPS_DIR/wasi-pydantic/python/"

# Copy annotated_types package
echo "Copying annotated_types..."
cp -r "annotated_types-${ANNOTATED_TYPES_VERSION}/annotated_types/"* \
   "$DEPS_DIR/wasi-pydantic/python/annotated_types/"

# Copy typing_inspection package
echo "Copying typing_inspection..."
cp -r "typing_inspection-${TYPING_INSPECTION_VERSION}/src/typing_inspection/"* \
   "$DEPS_DIR/wasi-pydantic/python/typing_inspection/"

# Copy pydantic package (high-level library)
echo "Copying pydantic..."
cp -r "pydantic-${PYDANTIC_VERSION}/pydantic/"* \
   "$DEPS_DIR/wasi-pydantic/python/pydantic/"

# Apply WASI patches to pydantic
# These patches handle importlib.metadata circular import issues in WASI environments
echo "Applying WASI patches to pydantic..."

# Patch 1: pydantic/plugin/_loader.py - wrap importlib.metadata import
cat > "$DEPS_DIR/wasi-pydantic/python/pydantic/plugin/_loader.py" << 'PYEOF'
from __future__ import annotations

import os
import warnings
from collections.abc import Iterable
from typing import TYPE_CHECKING, Final

# Handle importlib.metadata gracefully for WASI environments
try:
    import importlib.metadata as importlib_metadata
    _HAS_IMPORTLIB_METADATA = True
except (ImportError, ModuleNotFoundError):
    _HAS_IMPORTLIB_METADATA = False
    importlib_metadata = None  # type: ignore

if TYPE_CHECKING:
    from . import PydanticPluginProtocol


PYDANTIC_ENTRY_POINT_GROUP: Final[str] = 'pydantic'

# cache of plugins
_plugins: dict[str, PydanticPluginProtocol] | None = None
# return no plugins while loading plugins to avoid recursion and errors while import plugins
# this means that if plugins use pydantic
_loading_plugins: bool = False


def get_plugins() -> Iterable[PydanticPluginProtocol]:
    """Load plugins for Pydantic.

    Inspired by: https://github.com/pytest-dev/pluggy/blob/1.3.0/src/pluggy/_manager.py#L376-L402
    """
    # In WASI environments, importlib.metadata may not be available
    if not _HAS_IMPORTLIB_METADATA:
        return ()

    disabled_plugins = os.getenv('PYDANTIC_DISABLE_PLUGINS')
    global _plugins, _loading_plugins
    if _loading_plugins:
        # this happens when plugins themselves use pydantic, we return no plugins
        return ()
    elif disabled_plugins in ('__all__', '1', 'true'):
        return ()
    elif _plugins is None:
        _plugins = {}
        # set _loading_plugins so any plugins that use pydantic don't themselves use plugins
        _loading_plugins = True
        try:
            for dist in importlib_metadata.distributions():
                for entry_point in dist.entry_points:
                    if entry_point.group != PYDANTIC_ENTRY_POINT_GROUP:
                        continue
                    if entry_point.value in _plugins:
                        continue
                    if disabled_plugins is not None and entry_point.name in disabled_plugins.split(','):
                        continue
                    try:
                        _plugins[entry_point.value] = entry_point.load()
                    except (ImportError, AttributeError) as e:
                        warnings.warn(
                            f'{e.__class__.__name__} while loading the `{entry_point.name}` Pydantic plugin, '
                            f'this plugin will not be installed.\n\n{e!r}',
                            stacklevel=2,
                        )
        finally:
            _loading_plugins = False

    return _plugins.values()
PYEOF

# Patch 2: pydantic/version.py - wrap importlib.metadata usage
cat > "$DEPS_DIR/wasi-pydantic/python/pydantic/version.py" << 'PYEOF'
"""The `version` module holds the version information for Pydantic."""

from __future__ import annotations as _annotations

import sys

from pydantic_core import __version__ as __pydantic_core_version__

__all__ = 'VERSION', 'version_info'

VERSION = '2.12.5'
"""The version of Pydantic.

This version specifier is guaranteed to be compliant with the [specification],
introduced by [PEP 440].

[specification]: https://packaging.python.org/en/latest/specifications/version-specifiers/
[PEP 440]: https://peps.python.org/pep-0440/
"""

# Keep this in sync with the version constraint in the `pyproject.toml` dependencies:
_COMPATIBLE_PYDANTIC_CORE_VERSION = '2.41.5'


def version_short() -> str:
    """Return the `major.minor` part of Pydantic version.

    It returns '2.1' if Pydantic version is '2.1.1'.
    """
    return '.'.join(VERSION.split('.')[:2])


def version_info() -> str:
    """Return complete version information for Pydantic and its dependencies."""
    import platform
    from pathlib import Path

    import pydantic_core._pydantic_core as pdc

    from ._internal import _git as git

    # get data about packages that are closely related to pydantic, use pydantic or often conflict with pydantic
    package_names = {
        'email-validator',
        'fastapi',
        'mypy',
        'pydantic-extra-types',
        'pydantic-settings',
        'pyright',
        'typing_extensions',
    }
    related_packages = []

    try:
        import importlib.metadata
        for dist in importlib.metadata.distributions():
            name = dist.metadata['Name']
            if name in package_names:
                related_packages.append(f'{name}-{dist.version}')
    except (ImportError, ModuleNotFoundError):
        # In WASI environment, importlib.metadata may not work
        related_packages = ['(unavailable in WASI)']

    pydantic_dir = Path(__file__).parents[1].resolve()
    most_recent_commit = (
        git.git_revision(pydantic_dir) if git.is_git_repo(pydantic_dir) and git.have_git() else 'unknown'
    )

    info = {
        'pydantic version': VERSION,
        'pydantic-core version': __pydantic_core_version__,
        'pydantic-core build': getattr(pdc, 'build_info', None) or pdc.build_profile,  # pyright: ignore[reportPrivateImportUsage]
        'python version': sys.version,
        'platform': platform.platform(),
        'related packages': ' '.join(related_packages),
        'commit': most_recent_commit,
    }
    return '\n'.join('{:>30} {}'.format(k + ':', str(v).replace('\n', ' ')) for k, v in info.items())


def check_pydantic_core_version() -> bool:
    """Check that the installed `pydantic-core` dependency is compatible."""
    return __pydantic_core_version__ == _COMPATIBLE_PYDANTIC_CORE_VERSION


def _ensure_pydantic_core_version() -> None:  # pragma: no cover
    if not check_pydantic_core_version():
        raise_error = True
        # Skip editable mode check in WASI environments where importlib.metadata is broken
        try:
            if sys.version_info >= (3, 13):  # origin property added in 3.13
                from importlib.metadata import distribution

                dist = distribution('pydantic')
                if getattr(getattr(dist.origin, 'dir_info', None), 'editable', False):
                    raise_error = False
        except (ImportError, ModuleNotFoundError):
            # In WASI environment, importlib.metadata may not work
            # Assume not in editable mode - the version check will still be done
            pass

        if raise_error:
            raise SystemError(
                f'The installed pydantic-core version ({__pydantic_core_version__}) is incompatible '
                f'with the current pydantic version, which requires {_COMPATIBLE_PYDANTIC_CORE_VERSION}. '
                "If you encounter this error, make sure that you haven't upgraded pydantic-core manually."
            )


def parse_mypy_version(version: str) -> tuple[int, int, int]:
    """Parse `mypy` string version to a 3-tuple of ints.

    It parses normal version like `1.11.0` and extra info followed by a `+` sign
    like `1.11.0+dev.d6d9d8cd4f27c52edac1f537e236ec48a01e54cb.dirty`.

    Args:
        version: The mypy version string.

    Returns:
        A triple of ints, e.g. `(1, 11, 0)`.
    """
    return tuple(map(int, version.partition('+')[0].split('.')))  # pyright: ignore[reportReturnType]
PYEOF

# Patch 3: pydantic/networks.py - wrap version import at the top
NETWORKS_FILE="$DEPS_DIR/wasi-pydantic/python/pydantic/networks.py"
if [ -f "$NETWORKS_FILE" ]; then
    PATCH_FILE="$NETWORKS_FILE" python3 << 'PYSCRIPT'
import os

file_path = os.environ['PATCH_FILE']

with open(file_path, 'r') as f:
    content = f.read()

# The import line we need to replace
old_import = "from importlib.metadata import version"
new_import = """# Handle importlib.metadata gracefully for WASI environments
def _get_package_version(package: str) -> str:
    \"\"\"Get package version, returning 'unknown' if not available.\"\"\"
    try:
        from importlib.metadata import version as _version
        return _version(package)
    except (ImportError, ModuleNotFoundError):
        return 'unknown'

# For backwards compatibility - code that uses 'version' directly
def version(package: str) -> str:
    \"\"\"Wrapper for importlib.metadata.version that handles WASI environments.\"\"\"
    return _get_package_version(package)"""

if old_import in content:
    content = content.replace(old_import, new_import)
    with open(file_path, 'w') as f:
        f.write(content)
    print("  - Patched networks.py")
else:
    print("  - networks.py already patched or import not found")
PYSCRIPT
fi

echo ""
echo "=== pydantic build complete ==="
echo "Library: $DEPS_DIR/wasi-pydantic/lib/lib_pydantic_core.a ($(du -h "$DEPS_DIR/wasi-pydantic/lib/lib_pydantic_core.a" | cut -f1))"
echo "Python packages bundled:"
echo "  - pydantic_core"
echo "  - pydantic (high-level API)"
echo "  - typing_extensions"
echo "  - annotated_types"
echo "  - typing_inspection"
