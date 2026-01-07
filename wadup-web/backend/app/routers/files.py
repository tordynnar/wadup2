"""Files router for module file operations."""
from fastapi import APIRouter, Depends, HTTPException, Body
from sqlalchemy.orm import Session
from typing import Optional

from app.database import get_db
from app.models.user import User
from app.models.module import Module
from app.schemas.module import FileTreeNode, FileContent
from app.routers.auth import require_user
from app.services.module_service import ModuleService

router = APIRouter(prefix="/api/modules/{module_id}/files", tags=["files"])


def get_module_with_access(
    module_id: int,
    user: User,
    db: Session,
    require_write: bool = False,
) -> Module:
    """Get a module and check access permissions."""
    module = db.query(Module).filter(Module.id == module_id).first()
    if not module:
        raise HTTPException(status_code=404, detail="Module not found")

    # Check access
    is_owner = module.author_id == user.id
    if require_write and not is_owner:
        raise HTTPException(status_code=403, detail="Not authorized to modify this module")
    if not is_owner and not module.is_published:
        raise HTTPException(status_code=404, detail="Module not found")

    return module


@router.get("", response_model=FileTreeNode)
def list_files(
    module_id: int,
    version: str = "draft",
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """List all files in a module as a tree structure."""
    module = get_module_with_access(module_id, user, db)

    # Non-owners can only see published version
    if module.author_id != user.id:
        version = "published"

    module_service = ModuleService()
    tree = module_service.list_files(module_id, version)
    return tree


@router.get("/{path:path}", response_model=FileContent)
def get_file(
    module_id: int,
    path: str,
    version: str = "draft",
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Get the contents of a file."""
    module = get_module_with_access(module_id, user, db)

    # Non-owners can only see published version
    if module.author_id != user.id:
        version = "published"

    module_service = ModuleService()
    try:
        content = module_service.read_file(module_id, path, version)
    except FileNotFoundError:
        raise HTTPException(status_code=404, detail="File not found")
    except PermissionError:
        raise HTTPException(status_code=403, detail="Access denied")

    # Detect language from extension
    language = detect_language(path)

    return FileContent(path=path, content=content, language=language)


@router.put("/{path:path}")
def update_file(
    module_id: int,
    path: str,
    content: str = Body("", media_type="text/plain"),
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Create or update a file."""
    module = get_module_with_access(module_id, user, db, require_write=True)

    module_service = ModuleService()
    try:
        module_service.write_file(module_id, path, content, "draft")
    except PermissionError:
        raise HTTPException(status_code=403, detail="Access denied")

    # Update module timestamp
    from datetime import datetime

    module.updated_at = datetime.utcnow()
    db.commit()

    return {"message": "File saved"}


@router.delete("/{path:path}")
def delete_file(
    module_id: int,
    path: str,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Delete a file."""
    module = get_module_with_access(module_id, user, db, require_write=True)

    module_service = ModuleService()
    try:
        module_service.delete_file(module_id, path, "draft")
    except FileNotFoundError:
        raise HTTPException(status_code=404, detail="File not found")
    except PermissionError:
        raise HTTPException(status_code=403, detail="Access denied")

    # Update module timestamp
    from datetime import datetime

    module.updated_at = datetime.utcnow()
    db.commit()

    return {"message": "File deleted"}


@router.post("/folders/{path:path}")
def create_folder(
    module_id: int,
    path: str,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Create a folder."""
    module = get_module_with_access(module_id, user, db, require_write=True)

    module_service = ModuleService()
    try:
        module_service.create_folder(module_id, path, "draft")
    except PermissionError:
        raise HTTPException(status_code=403, detail="Access denied")

    return {"message": "Folder created"}


@router.post("/{path:path}/rename")
def rename_file(
    module_id: int,
    path: str,
    new_path: str = Body(..., embed=True),
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Rename a file or folder."""
    module = get_module_with_access(module_id, user, db, require_write=True)

    module_service = ModuleService()
    try:
        module_service.rename_file(module_id, path, new_path, "draft")
    except FileNotFoundError:
        raise HTTPException(status_code=404, detail="File not found")
    except FileExistsError:
        raise HTTPException(status_code=409, detail="Target already exists")
    except PermissionError:
        raise HTTPException(status_code=403, detail="Access denied")

    # Update module timestamp
    from datetime import datetime

    module.updated_at = datetime.utcnow()
    db.commit()

    return {"message": "File renamed"}


def detect_language(path: str) -> Optional[str]:
    """Detect programming language from file extension."""
    ext_map = {
        ".rs": "rust",
        ".go": "go",
        ".py": "python",
        ".toml": "toml",
        ".json": "json",
        ".md": "markdown",
        ".txt": "plaintext",
        ".mod": "go.mod",
        ".sum": "go.sum",
    }
    for ext, lang in ext_map.items():
        if path.endswith(ext):
            return lang
    return None
