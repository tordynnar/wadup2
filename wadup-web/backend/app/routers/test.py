"""Test router for running module tests."""
from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import StreamingResponse
from sqlalchemy.orm import Session
from typing import List

from app.database import get_db
from app.models.user import User
from app.models.module import Module, BuildStatus
from app.models.sample import Sample, TestRun, TestStatus
from app.schemas.sample import TestRunCreate, TestRunResponse
from app.routers.auth import require_user
from app.services.test_service import TestService

router = APIRouter(prefix="/api/modules/{module_id}/test", tags=["test"])


@router.post("", response_model=List[TestRunResponse])
def start_test(
    module_id: int,
    request: TestRunCreate,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Start testing a module with the specified samples."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to test this module")

    draft = module.draft_version
    if not draft:
        raise HTTPException(status_code=400, detail="No draft version found")

    if draft.build_status != BuildStatus.SUCCESS:
        raise HTTPException(status_code=400, detail="Module must be built successfully before testing")

    if not draft.wasm_path:
        raise HTTPException(status_code=400, detail="No WASM artifact found")

    # Validate samples
    samples = db.query(Sample).filter(
        Sample.id.in_(request.sample_ids),
        Sample.owner_id == user.id,
    ).all()

    if len(samples) != len(request.sample_ids):
        raise HTTPException(status_code=400, detail="One or more samples not found")

    # Create test runs
    test_runs = []
    for sample in samples:
        test_run = TestRun(
            module_version_id=draft.id,
            sample_id=sample.id,
            status=TestStatus.PENDING,
        )
        db.add(test_run)
        test_runs.append(test_run)

    db.commit()
    for run in test_runs:
        db.refresh(run)

    # Start tests in background
    test_service = TestService()
    for run in test_runs:
        test_service.start_test(run)

    return test_runs


@router.get("/{run_id}", response_model=TestRunResponse)
def get_test_run(
    module_id: int,
    run_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Get the status and results of a test run."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to view test results")

    test_run = db.query(TestRun).filter(TestRun.id == run_id).first()
    if not test_run:
        raise HTTPException(status_code=404, detail="Test run not found")

    # Verify the test run belongs to this module
    if test_run.module_version.module_id != module_id:
        raise HTTPException(status_code=404, detail="Test run not found")

    return test_run


@router.get("/{run_id}/stream")
async def stream_test_output(
    module_id: int,
    run_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Stream test output via Server-Sent Events."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to view test output")

    test_run = db.query(TestRun).filter(TestRun.id == run_id).first()
    if not test_run:
        raise HTTPException(status_code=404, detail="Test run not found")

    if test_run.module_version.module_id != module_id:
        raise HTTPException(status_code=404, detail="Test run not found")

    test_service = TestService()

    async def event_generator():
        async for event in test_service.stream_output(run_id):
            yield event

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",
        },
    )
