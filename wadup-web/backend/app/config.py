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


settings = Settings()
