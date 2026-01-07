"""Module service for file management and template initialization."""
import shutil
from pathlib import Path
from typing import Optional
from jinja2 import Environment, FileSystemLoader

from app.config import settings
from app.schemas.module import FileTreeNode
from app.models.module import Module, Language


class ModuleService:
    """Service for managing module files."""

    def __init__(self):
        self.storage_root = settings.storage_root
        self.modules_dir = settings.modules_dir
        self.artifacts_dir = settings.artifacts_dir
        self.templates_dir = Path(__file__).parent.parent / "templates"
        self.jinja_env = Environment(
            loader=FileSystemLoader(str(self.templates_dir)),
            autoescape=False,
        )

    def get_module_path(self, module_id: int, version: str = "draft") -> Path:
        """Get the filesystem path for a module version."""
        return self.modules_dir / str(module_id) / version

    def get_artifact_path(self, module_id: int, version: str = "draft") -> Path:
        """Get the filesystem path for module artifacts."""
        return self.artifacts_dir / str(module_id) / version

    def initialize_module(self, module: Module) -> None:
        """Initialize a new module with template files."""
        module_path = self.get_module_path(module.id, "draft")
        module_path.mkdir(parents=True, exist_ok=True)

        context = {
            "module_name": module.name,
            "module_name_snake": module.name.replace("-", "_").replace(" ", "_").lower(),
        }

        if module.language == Language.RUST:
            self._init_rust_module(module_path, context)
        elif module.language == Language.GO:
            self._init_go_module(module_path, context)
        elif module.language == Language.PYTHON:
            self._init_python_module(module_path, context)

    def _init_rust_module(self, path: Path, context: dict) -> None:
        """Initialize a Rust module from template."""
        # Create Cargo.toml
        cargo_template = self.jinja_env.get_template("rust/Cargo.toml.j2")
        (path / "Cargo.toml").write_text(cargo_template.render(context))

        # Create src directory and lib.rs
        src_dir = path / "src"
        src_dir.mkdir(exist_ok=True)
        lib_template = self.jinja_env.get_template("rust/lib.rs.j2")
        (src_dir / "lib.rs").write_text(lib_template.render(context))

    def _init_go_module(self, path: Path, context: dict) -> None:
        """Initialize a Go module from template."""
        # Create go.mod
        mod_template = self.jinja_env.get_template("go/go.mod.j2")
        (path / "go.mod").write_text(mod_template.render(context))

        # Create main.go
        main_template = self.jinja_env.get_template("go/main.go.j2")
        (path / "main.go").write_text(main_template.render(context))

    def _init_python_module(self, path: Path, context: dict) -> None:
        """Initialize a Python module from template."""
        # Create pyproject.toml
        proj_template = self.jinja_env.get_template("python/pyproject.toml.j2")
        (path / "pyproject.toml").write_text(proj_template.render(context))

        # Create package directory and __init__.py
        pkg_dir = path / context["module_name_snake"]
        pkg_dir.mkdir(exist_ok=True)
        init_template = self.jinja_env.get_template("python/__init__.py.j2")
        (pkg_dir / "__init__.py").write_text(init_template.render(context))

    def delete_module(self, module_id: int) -> None:
        """Delete all files for a module."""
        module_path = self.modules_dir / str(module_id)
        if module_path.exists():
            shutil.rmtree(module_path)

        artifact_path = self.artifacts_dir / str(module_id)
        if artifact_path.exists():
            shutil.rmtree(artifact_path)

    def list_files(self, module_id: int, version: str = "draft") -> FileTreeNode:
        """List all files in a module as a tree structure."""
        root = self.get_module_path(module_id, version)
        if not root.exists():
            return FileTreeNode(name=version, type="directory", children=[])
        return self._build_tree(root, root)

    def _build_tree(self, path: Path, root: Path) -> FileTreeNode:
        """Build a file tree node recursively."""
        # Calculate relative path (empty string for root directory)
        rel_path = str(path.relative_to(root)) if path != root else ""
        if rel_path == ".":
            rel_path = ""

        if path.is_file():
            return FileTreeNode(
                name=path.name,
                type="file",
                path=rel_path,
            )

        children = []
        for item in sorted(path.iterdir()):
            # Skip hidden files and directories
            if item.name.startswith("."):
                continue
            # Skip target directory for Rust
            if item.name == "target":
                continue
            children.append(self._build_tree(item, root))

        return FileTreeNode(
            name=path.name,
            type="directory",
            path=rel_path,
            children=children,
        )

    def read_file(self, module_id: int, file_path: str, version: str = "draft") -> str:
        """Read the contents of a file."""
        full_path = self._validate_path(module_id, file_path, version)
        if not full_path.exists():
            raise FileNotFoundError(f"File not found: {file_path}")
        if not full_path.is_file():
            raise IsADirectoryError(f"Not a file: {file_path}")
        return full_path.read_text()

    def write_file(self, module_id: int, file_path: str, content: str, version: str = "draft") -> None:
        """Write content to a file."""
        full_path = self._validate_path(module_id, file_path, version)
        full_path.parent.mkdir(parents=True, exist_ok=True)
        full_path.write_text(content)

    def delete_file(self, module_id: int, file_path: str, version: str = "draft") -> None:
        """Delete a file."""
        full_path = self._validate_path(module_id, file_path, version)
        if not full_path.exists():
            raise FileNotFoundError(f"File not found: {file_path}")
        if full_path.is_dir():
            shutil.rmtree(full_path)
        else:
            full_path.unlink()

    def create_folder(self, module_id: int, folder_path: str, version: str = "draft") -> None:
        """Create a folder."""
        full_path = self._validate_path(module_id, folder_path, version)
        full_path.mkdir(parents=True, exist_ok=True)

    def rename_file(self, module_id: int, old_path: str, new_path: str, version: str = "draft") -> None:
        """Rename a file or folder."""
        old_full_path = self._validate_path(module_id, old_path, version)
        new_full_path = self._validate_path(module_id, new_path, version)

        if not old_full_path.exists():
            raise FileNotFoundError(f"File not found: {old_path}")
        if new_full_path.exists():
            raise FileExistsError(f"Target already exists: {new_path}")

        new_full_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(old_full_path), str(new_full_path))

    def copy_version(self, module_id: int, from_version: str, to_version: str) -> None:
        """Copy all files from one version to another."""
        src = self.get_module_path(module_id, from_version)
        dst = self.get_module_path(module_id, to_version)

        if dst.exists():
            shutil.rmtree(dst)

        shutil.copytree(src, dst, ignore=shutil.ignore_patterns("target", "*.wasm"))

    def copy_artifact(self, src_path: str, dst_path: str) -> None:
        """Copy a build artifact."""
        src = self.storage_root / src_path
        dst = self.storage_root / dst_path

        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)

    def _validate_path(self, module_id: int, file_path: str, version: str) -> Path:
        """Validate and resolve a file path, preventing directory traversal."""
        root = self.get_module_path(module_id, version)
        full_path = (root / file_path).resolve()

        # Ensure the path is within the module directory
        try:
            full_path.relative_to(root.resolve())
        except ValueError:
            raise PermissionError(f"Access denied: {file_path}")

        return full_path
