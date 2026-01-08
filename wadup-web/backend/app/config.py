"""Application configuration."""
from pathlib import Path
from pydantic_settings import BaseSettings

# Base directory for the web app
BASE_DIR = Path(__file__).parent.parent.parent.resolve()
STORAGE_DIR = BASE_DIR / "storage"


class Settings(BaseSettings):
    """Application settings loaded from environment variables."""

    # Database
    database_url: str = f"sqlite:///{STORAGE_DIR}/wadup.db"

    # Storage paths
    storage_root: Path = STORAGE_DIR
    modules_dir: Path = STORAGE_DIR / "modules"
    artifacts_dir: Path = STORAGE_DIR / "artifacts"
    samples_dir: Path = STORAGE_DIR / "samples"

    # Host storage path (for Docker volume mounts when running inside container)
    # When running in Docker, this should be set to the host path that maps to storage_root
    host_storage_root: Path | None = None

    # Server
    host: str = "0.0.0.0"
    port: int = 8080
    debug: bool = False

    # Docker
    docker_socket: str = "/var/run/docker.sock"
    build_timeout: int = 600  # 10 minutes
    test_timeout: int = 300   # 5 minutes

    # Build images
    rust_build_image: str = "wadup-build-rust:latest"
    go_build_image: str = "wadup-build-go:latest"
    python_build_image: str = "wadup-build-python:latest"

    # Test runner image
    test_runner_image: str = "wadup-test-runner:latest"

    class Config:
        env_prefix = "WADUP_"
        env_file = ".env"

    def get_host_path(self, container_path: Path) -> Path:
        """Convert a container storage path to a host path for Docker volume mounts.

        When running inside a container, volume mounts need the host path, not the
        container path. This method translates paths under storage_root to their
        corresponding paths under host_storage_root.
        """
        if self.host_storage_root is None:
            # Not running in container mode, use paths as-is
            return container_path

        # Make paths absolute and resolve symlinks
        container_path = container_path.resolve()
        storage_root = self.storage_root.resolve()

        # Check if the path is under storage_root
        try:
            relative = container_path.relative_to(storage_root)
            return self.host_storage_root / relative
        except ValueError:
            # Path is not under storage_root, return as-is
            return container_path


settings = Settings()
