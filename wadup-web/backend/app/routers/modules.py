"""Modules router."""
from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session
from sqlalchemy import or_
from typing import Optional
from math import ceil

from app.database import get_db
from app.models.user import User
from app.models.module import Module, ModuleVersion, Language, VersionType
from app.schemas.module import ModuleCreate, ModuleResponse, ModuleListResponse, ModuleVersionResponse
from app.routers.auth import require_user
from app.services.module_service import ModuleService

router = APIRouter(prefix="/api/modules", tags=["modules"])


def module_to_response(module: Module) -> ModuleResponse:
    """Convert a Module to ModuleResponse."""
    return ModuleResponse(
        id=module.id,
        name=module.name,
        description=module.description,
        language=module.language,
        author_id=module.author_id,
        author_username=module.author.username if module.author else None,
        is_published=module.is_published,
        published_at=module.published_at,
        created_at=module.created_at,
        updated_at=module.updated_at,
        draft_version=ModuleVersionResponse.model_validate(module.draft_version)
        if module.draft_version
        else None,
        published_version=ModuleVersionResponse.model_validate(module.published_version)
        if module.published_version
        else None,
    )


@router.get("", response_model=ModuleListResponse)
def list_modules(
    filter: str = Query("all", pattern="^(mine|published|all)$"),
    search: Optional[str] = None,
    language: Optional[Language] = None,
    page: int = Query(1, ge=1),
    limit: int = Query(20, ge=1, le=100),
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """List modules with filtering and pagination."""
    query = db.query(Module)

    # Apply filter
    if filter == "mine":
        query = query.filter(Module.author_id == user.id)
    elif filter == "published":
        query = query.filter(Module.is_published == True)
    # "all" shows user's own modules + all published modules
    elif filter == "all":
        query = query.filter(
            or_(Module.author_id == user.id, Module.is_published == True)
        )

    # Apply language filter
    if language:
        query = query.filter(Module.language == language)

    # Apply search
    if search:
        search_term = f"%{search}%"
        query = query.filter(Module.name.ilike(search_term))

    # Get total count
    total = query.count()

    # Calculate pagination
    pages = ceil(total / limit) if total > 0 else 1
    offset = (page - 1) * limit

    # Get paginated results
    modules = query.order_by(Module.updated_at.desc()).offset(offset).limit(limit).all()

    return ModuleListResponse(
        items=[module_to_response(m) for m in modules],
        total=total,
        page=page,
        limit=limit,
        pages=pages,
    )


@router.post("", response_model=ModuleResponse)
def create_module(
    request: ModuleCreate,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Create a new module."""
    # Create module
    module = Module(
        name=request.name,
        description=request.description,
        language=request.language,
        author_id=user.id,
    )
    db.add(module)
    db.flush()  # Get the module ID

    # Create draft version
    source_path = f"modules/{module.id}/draft"
    draft_version = ModuleVersion(
        module_id=module.id,
        version_type=VersionType.DRAFT,
        source_path=source_path,
    )
    db.add(draft_version)
    db.commit()
    db.refresh(module)

    # Initialize module files from template
    module_service = ModuleService()
    module_service.initialize_module(module)

    return module_to_response(module)


@router.get("/{module_id}", response_model=ModuleResponse)
def get_module(
    module_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Get a module by ID."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    # Check access: user owns it or it's published
    if module.author_id != user.id and not module.is_published:
        raise HTTPException(status_code=404, detail="Module not found")

    return module_to_response(module)


@router.delete("/{module_id}")
def delete_module(
    module_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Delete a module (owner only)."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to delete this module")

    # Delete module files
    module_service = ModuleService()
    module_service.delete_module(module.id)

    # Delete from database
    db.delete(module)
    db.commit()

    return {"message": "Module deleted"}


@router.post("/{module_id}/publish", response_model=ModuleResponse)
def publish_module(
    module_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Publish a module (requires successful build)."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    if module.author_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to publish this module")

    draft = module.draft_version
    if not draft:
        raise HTTPException(status_code=400, detail="No draft version found")

    if draft.build_status != "success":
        raise HTTPException(status_code=400, detail="Module must be built successfully before publishing")

    # Create or update published version
    module_service = ModuleService()
    published = module.published_version

    if not published:
        source_path = f"modules/{module.id}/published"
        published = ModuleVersion(
            module_id=module.id,
            version_type=VersionType.PUBLISHED,
            source_path=source_path,
        )
        db.add(published)
        db.flush()

    # Copy draft to published
    module_service.copy_version(module.id, "draft", "published")

    # Copy WASM artifact
    if draft.wasm_path:
        wasm_dest = f"artifacts/{module.id}/published/module.wasm"
        module_service.copy_artifact(draft.wasm_path, wasm_dest)
        published.wasm_path = wasm_dest

    # Update published version status
    published.build_status = draft.build_status
    published.built_at = draft.built_at

    # Update module
    from datetime import datetime

    module.is_published = True
    module.published_at = datetime.utcnow()

    db.commit()
    db.refresh(module)

    return module_to_response(module)
