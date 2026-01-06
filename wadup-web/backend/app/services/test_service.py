"""Test service for running module tests in Docker containers."""
import asyncio
import json
import subprocess
import threading
from datetime import datetime
from pathlib import Path
from typing import AsyncGenerator
import docker

from app.config import settings
from app.models.sample import TestRun, TestStatus
from app.database import SessionLocal


class TestService:
    """Service for running module tests."""

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
        """Run the test in a Docker container using wadup CLI."""
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

            self._add_output(run_id, f"Testing with sample: {sample.filename}")
            self._add_output(run_id, f"WASM module: {wasm_path.name}")

            try:
                # Run wadup test command
                # Note: This assumes wadup CLI has a 'test' command that outputs JSON
                # This will need to be implemented in the wadup CLI
                result = subprocess.run(
                    [
                        "wadup",
                        "test",
                        "--wasm", str(wasm_path),
                        "--input", str(sample_path),
                        "--output", "json",
                    ],
                    capture_output=True,
                    text=True,
                    timeout=settings.test_timeout,
                )

                # Parse output
                if result.returncode == 0:
                    try:
                        output = json.loads(result.stdout)
                        test_run.metadata_output = output.get("tables", [])
                        test_run.stdout = output.get("stdout", "")
                        test_run.stderr = output.get("stderr", "")
                        test_run.status = TestStatus.SUCCESS
                        self._add_output(run_id, "Test completed successfully!")
                    except json.JSONDecodeError:
                        test_run.stdout = result.stdout
                        test_run.stderr = result.stderr
                        test_run.status = TestStatus.SUCCESS
                        self._add_output(run_id, "Test completed (output not JSON)")
                else:
                    test_run.status = TestStatus.FAILED
                    test_run.error_message = result.stderr or f"Exit code: {result.returncode}"
                    test_run.stdout = result.stdout
                    test_run.stderr = result.stderr
                    self._add_output(run_id, f"Test failed: {test_run.error_message}")

            except subprocess.TimeoutExpired:
                test_run.status = TestStatus.FAILED
                test_run.error_message = f"Test timed out after {settings.test_timeout} seconds"
                self._add_output(run_id, test_run.error_message)
            except FileNotFoundError:
                # wadup CLI not installed - provide helpful message
                test_run.status = TestStatus.FAILED
                test_run.error_message = "wadup CLI not found. Please ensure wadup is installed and in PATH."
                self._add_output(run_id, test_run.error_message)
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
