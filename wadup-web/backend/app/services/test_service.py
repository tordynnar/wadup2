"""Test service for running module tests in Docker containers."""
import asyncio
import json
import threading
from datetime import datetime
from pathlib import Path
from typing import AsyncGenerator
import docker
from docker.errors import ContainerError, ImageNotFound

from app.config import settings
from app.models.sample import TestRun, TestStatus
from app.database import SessionLocal


class TestService:
    """Service for running module tests in Docker containers."""

    # Store for active test output (run_id -> list of output lines)
    _test_output: dict[int, list[str]] = {}
    _test_complete: dict[int, bool] = {}

    def __init__(self):
        self.docker_client = docker.from_env()
        self.storage_root = settings.storage_root.resolve()

    def start_test(self, test_run: TestRun) -> None:
        """Start a test run in a background thread."""
        # Initialize output storage
        self._test_output[test_run.id] = []
        self._test_complete[test_run.id] = False

        # Start test thread
        thread = threading.Thread(
            target=self._run_test,
            args=(test_run.id,),
            daemon=True,
        )
        thread.start()

    def _run_test(self, run_id: int) -> None:
        """Run the test in a Docker container."""
        db = SessionLocal()
        try:
            test_run = db.query(TestRun).filter(TestRun.id == run_id).first()
            if not test_run:
                self._add_output(run_id, "ERROR: Test run not found")
                return

            # Update status
            test_run.status = TestStatus.RUNNING
            test_run.started_at = datetime.utcnow()
            db.commit()

            # Get paths
            module_version = test_run.module_version
            sample = test_run.sample

            wasm_path = self.storage_root / module_version.wasm_path
            sample_path = self.storage_root / sample.file_path

            # Convert to host paths for Docker volume mounts
            host_wasm_path = settings.get_host_path(wasm_path)
            host_sample_path = settings.get_host_path(sample_path)

            self._add_output(run_id, f"Testing with sample: {sample.filename}")
            self._add_output(run_id, f"WASM module: {wasm_path.name}")
            self._add_output(run_id, f"Running in Docker container...")

            try:
                # Run the test in Docker container
                # Mount the wasm file and sample file directly
                container = self.docker_client.containers.run(
                    settings.test_runner_image,
                    command=[
                        "--module", "/test/module.wasm",
                        "--sample", "/test/sample.bin",
                        "--filename", sample.filename,
                    ],
                    detach=True,
                    volumes={
                        str(host_wasm_path): {"bind": "/test/module.wasm", "mode": "ro"},
                        str(host_sample_path): {"bind": "/test/sample.bin", "mode": "ro"},
                    },
                    remove=False,
                    mem_limit="512m",
                    cpu_period=100000,
                    cpu_quota=100000,  # 1 CPU
                )

                # Wait for container to complete
                result = container.wait(timeout=settings.test_timeout)
                exit_code = result.get("StatusCode", 1)

                # Get container logs (stdout contains the JSON result)
                logs = container.logs(stdout=True, stderr=True).decode("utf-8", errors="replace")

                # Remove container
                container.remove()

                # Parse the JSON output
                try:
                    output = json.loads(logs)

                    if output.get("success", False):
                        test_run.status = TestStatus.SUCCESS
                        test_run.stdout = output.get("stdout", "")
                        test_run.stderr = output.get("stderr", "")
                        test_run.metadata_output = output.get("metadata")
                        test_run.subcontent_output = output.get("subcontent")
                        self._add_output(run_id, "Test completed successfully!")

                        if test_run.stdout:
                            self._add_output(run_id, f"stdout: {test_run.stdout[:200]}")
                        if test_run.metadata_output:
                            self._add_output(run_id, f"Metadata: {json.dumps(test_run.metadata_output)[:200]}")
                        if test_run.subcontent_output:
                            self._add_output(run_id, f"Subcontent: {len(test_run.subcontent_output)} file(s)")
                    else:
                        test_run.status = TestStatus.FAILED
                        test_run.error_message = output.get("error", f"Exit code: {output.get('exit_code', exit_code)}")
                        test_run.stdout = output.get("stdout", "")
                        test_run.stderr = output.get("stderr", "")
                        test_run.subcontent_output = output.get("subcontent")
                        self._add_output(run_id, f"Test failed: {test_run.error_message}")
                        if test_run.stderr:
                            self._add_output(run_id, f"stderr: {test_run.stderr[:500]}")

                except json.JSONDecodeError:
                    # Output wasn't JSON, treat as raw output
                    if exit_code == 0:
                        test_run.status = TestStatus.SUCCESS
                        test_run.stdout = logs
                        self._add_output(run_id, "Test completed (non-JSON output)")
                    else:
                        test_run.status = TestStatus.FAILED
                        test_run.error_message = f"Exit code: {exit_code}"
                        test_run.stderr = logs
                        self._add_output(run_id, f"Test failed: {test_run.error_message}")

            except ImageNotFound:
                test_run.status = TestStatus.FAILED
                test_run.error_message = f"Test runner image not found: {settings.test_runner_image}. Run: docker build -t wadup-test-runner:latest ./docker/test"
                self._add_output(run_id, test_run.error_message)
            except ContainerError as e:
                test_run.status = TestStatus.FAILED
                test_run.error_message = f"Container error: {e}"
                self._add_output(run_id, f"ERROR: {e}")
            except Exception as e:
                test_run.status = TestStatus.FAILED
                test_run.error_message = str(e)
                self._add_output(run_id, f"ERROR: {e}")

            # Update completion time
            test_run.completed_at = datetime.utcnow()
            db.commit()

        finally:
            self._test_complete[run_id] = True
            db.close()

    def _add_output(self, run_id: int, line: str) -> None:
        """Add an output line for a test run."""
        if run_id not in self._test_output:
            self._test_output[run_id] = []
        self._test_output[run_id].append(line)

    async def stream_output(self, run_id: int) -> AsyncGenerator[str, None]:
        """Stream test output as Server-Sent Events."""
        last_index = 0

        while True:
            # Get new output
            output = self._test_output.get(run_id, [])
            new_output = output[last_index:]

            for line in new_output:
                event = {
                    "type": "output",
                    "content": line,
                }
                yield f"data: {json.dumps(event)}\n\n"
                last_index += 1

            # Check if test is complete
            if self._test_complete.get(run_id, False):
                # Get final status from database
                db = SessionLocal()
                try:
                    test_run = db.query(TestRun).filter(TestRun.id == run_id).first()
                    status = "unknown"
                    result = None
                    if test_run:
                        status = test_run.status.value
                        result = {
                            "stdout": test_run.stdout,
                            "stderr": test_run.stderr,
                            "metadata": test_run.metadata_output,
                            "error": test_run.error_message,
                        }
                finally:
                    db.close()

                event = {
                    "type": "complete",
                    "status": status,
                    "result": result,
                }
                yield f"data: {json.dumps(event)}\n\n"

                # Clean up
                self._test_output.pop(run_id, None)
                self._test_complete.pop(run_id, None)
                break

            await asyncio.sleep(0.1)
