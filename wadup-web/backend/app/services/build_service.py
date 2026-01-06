"""Build service for compiling modules in Docker containers."""
import asyncio
import json
import threading
from datetime import datetime
from pathlib import Path
from typing import AsyncGenerator, Optional
import docker
from docker.errors import ContainerError, ImageNotFound

from app.config import settings
from app.models.module import Module, ModuleVersion, Language, BuildStatus
from app.database import SessionLocal


class BuildService:
    """Service for building modules in Docker containers."""

    # Store for active build logs (module_id -> list of log lines)
    _build_logs: dict[int, list[str]] = {}
    _build_complete: dict[int, bool] = {}

    def __init__(self):
        self.docker_client = docker.from_env()
        self.storage_root = settings.storage_root.resolve()

    def get_image_for_language(self, language: Language) -> str:
        """Get the Docker image for a language."""
        images = {
            Language.RUST: settings.rust_build_image,
            Language.GO: settings.go_build_image,
            Language.PYTHON: settings.python_build_image,
        }
        return images[language]

    def start_build(self, module: Module, version: ModuleVersion) -> None:
        """Start a build in a background thread."""
        # Initialize log storage
        self._build_logs[module.id] = []
        self._build_complete[module.id] = False

        # Start build thread
        thread = threading.Thread(
            target=self._run_build,
            args=(module.id, module.language, version.id),
            daemon=True,
        )
        thread.start()

    def _run_build(self, module_id: int, language: Language, version_id: int) -> None:
        """Run the build process in a Docker container."""
        db = SessionLocal()
        try:
            version = db.query(ModuleVersion).filter(ModuleVersion.id == version_id).first()
            if not version:
                self._add_log(module_id, "ERROR: Version not found")
                return

            source_path = self.storage_root / version.source_path
            artifact_path = settings.artifacts_dir / str(module_id) / "draft"
            artifact_path.mkdir(parents=True, exist_ok=True)

            image = self.get_image_for_language(language)
            self._add_log(module_id, f"Starting build with image: {image}")

            # Run Docker container
            try:
                container = self.docker_client.containers.run(
                    image,
                    detach=True,
                    volumes={
                        str(source_path): {"bind": "/build/src", "mode": "ro"},
                        str(artifact_path): {"bind": "/build/output", "mode": "rw"},
                    },
                    user="builder",
                    working_dir="/build/src",
                    remove=False,
                    mem_limit="2g",
                    cpu_period=100000,
                    cpu_quota=100000,  # 1 CPU
                )

                # Stream logs
                for log in container.logs(stream=True, follow=True):
                    line = log.decode("utf-8", errors="replace").rstrip()
                    self._add_log(module_id, line)

                # Wait for container to finish
                result = container.wait()
                exit_code = result.get("StatusCode", 1)

                # Get any remaining logs
                container.remove()

                # Check for WASM file
                wasm_file = artifact_path / "module.wasm"
                if exit_code == 0 and wasm_file.exists():
                    version.build_status = BuildStatus.SUCCESS
                    version.wasm_path = f"artifacts/{module_id}/draft/module.wasm"
                    self._add_log(module_id, "Build completed successfully!")
                else:
                    version.build_status = BuildStatus.FAILED
                    self._add_log(module_id, f"Build failed with exit code: {exit_code}")

            except ImageNotFound:
                version.build_status = BuildStatus.FAILED
                self._add_log(module_id, f"ERROR: Docker image not found: {image}")
            except ContainerError as e:
                version.build_status = BuildStatus.FAILED
                self._add_log(module_id, f"ERROR: Container error: {e}")
            except Exception as e:
                version.build_status = BuildStatus.FAILED
                self._add_log(module_id, f"ERROR: Unexpected error: {e}")

            # Update version
            version.built_at = datetime.utcnow()
            version.build_log = "\n".join(self._build_logs.get(module_id, []))
            db.commit()

        finally:
            self._build_complete[module_id] = True
            db.close()

    def _add_log(self, module_id: int, line: str) -> None:
        """Add a log line for a module build."""
        if module_id not in self._build_logs:
            self._build_logs[module_id] = []
        self._build_logs[module_id].append(line)

    async def stream_logs(self, module_id: int) -> AsyncGenerator[str, None]:
        """Stream build logs as Server-Sent Events."""
        last_index = 0

        while True:
            # Get new logs
            logs = self._build_logs.get(module_id, [])
            new_logs = logs[last_index:]

            for line in new_logs:
                event = {
                    "type": "log",
                    "content": line,
                }
                yield f"data: {json.dumps(event)}\n\n"
                last_index += 1

            # Check if build is complete
            if self._build_complete.get(module_id, False):
                # Get final status from database
                db = SessionLocal()
                try:
                    from app.models.module import Module
                    module = db.query(Module).filter(Module.id == module_id).first()
                    status = "unknown"
                    if module and module.draft_version:
                        status = module.draft_version.build_status.value
                finally:
                    db.close()

                event = {
                    "type": "complete",
                    "status": status,
                }
                yield f"data: {json.dumps(event)}\n\n"

                # Clean up
                self._build_logs.pop(module_id, None)
                self._build_complete.pop(module_id, None)
                break

            await asyncio.sleep(0.1)
