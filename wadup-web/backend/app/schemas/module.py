"""Module-related schemas."""
from datetime import datetime
from typing import Optional, List
from pydantic import BaseModel, Field

from app.models.module import Language, BuildStatus, VersionType


class ModuleCreate(BaseModel):
    """Schema for creating a module."""
    name: str = Field(..., min_length=1, max_length=200)
    description: Optional[str] = None
    language: Language


class ModuleVersionResponse(BaseModel):
    """Module version response model."""
    id: int
    version_type: VersionType
    build_status: BuildStatus
    built_at: Optional[datetime] = None
    wasm_path: Optional[str] = None
    created_at: datetime

    class Config:
        from_attributes = True


class ModuleResponse(BaseModel):
    """Module response model."""
    id: int
    name: str
    description: Optional[str] = None
    language: Language
    author_id: int
    author_username: Optional[str] = None
    is_published: bool
    published_at: Optional[datetime] = None
    created_at: datetime
    updated_at: datetime
    draft_version: Optional[ModuleVersionResponse] = None
    published_version: Optional[ModuleVersionResponse] = None

    class Config:
        from_attributes = True


class ModuleListResponse(BaseModel):
    """Paginated list of modules."""
    items: List[ModuleResponse]
    total: int
    page: int
    limit: int
    pages: int


class FileTreeNode(BaseModel):
    """A node in the file tree."""
    name: str
    type: str  # "file" or "directory"
    path: Optional[str] = None  # Relative path for files
    children: Optional[List["FileTreeNode"]] = None


class FileContent(BaseModel):
    """File content response."""
    path: str
    content: str
    language: Optional[str] = None  # Detected language for editor


# Enable forward references for recursive model
FileTreeNode.model_rebuild()
