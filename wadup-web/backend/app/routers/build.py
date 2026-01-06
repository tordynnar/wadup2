"""Build router for module compilation."""
from datetime import datetime
from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import StreamingResponse
from sqlalchemy.orm import Session

from app.database import get_db
from app.models.user import User
from app.models.module import Module, BuildStatus
from app.routers.auth import require_user
from app.services.build_service import BuildService

router = APIRouter(prefix="/api/modules/{module_id}/build", tags=["build"])


@router.post("")
def start_build(
    module_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Start building a module."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to build this module")

    draft = module.draft_version
    if not draft:
        raise HTTPException(status_code=400, detail="No draft version found")

    # Check if already building
    if draft.build_status == BuildStatus.BUILDING:
        raise HTTPException(status_code=400, detail="Build already in progress")

    # Update status to building
    draft.build_status = BuildStatus.BUILDING
    draft.build_log = ""
    db.commit()

    # Start build in background
    build_service = BuildService()
    build_service.start_build(module, draft)

    return {"message": "Build started", "module_id": module_id}


@router.get("/status")
def get_build_status(
    module_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Get the current build status."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id and not module.is_published:
        raise HTTPException(status_code=404, detail="Module not found")

    draft = module.draft_version
    if not draft:
        raise HTTPException(status_code=400, detail="No draft version found")

    return {
        "status": draft.build_status,
        "built_at": draft.built_at,
        "wasm_path": draft.wasm_path,
    }


@router.get("/stream")
async def stream_build_logs(
    module_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Stream build logs via Server-Sent Events."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to view build logs")

    build_service = BuildService()

    async def event_generator():
        async for event in build_service.stream_logs(module_id):
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
