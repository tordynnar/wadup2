"""Module and ModuleVersion models."""
from datetime import datetime
from enum import Enum as PyEnum
from sqlalchemy import Column, Integer, String, Text, DateTime, Boolean, ForeignKey, Enum
from sqlalchemy.orm import relationship

from app.database import Base


class Language(str, PyEnum):
    """Supported module languages."""
    RUST = "rust"
    GO = "go"
    PYTHON = "python"


class VersionType(str, PyEnum):
    """Module version types."""
    DRAFT = "draft"
    PUBLISHED = "published"


class BuildStatus(str, PyEnum):
    """Build status values."""
    PENDING = "pending"
    BUILDING = "building"
    SUCCESS = "success"
    FAILED = "failed"


class Module(Base):
    """A WADUP module."""

    __tablename__ = "modules"

    id = Column(Integer, primary_key=True, index=True)
    name = Column(String(200), nullable=False, index=True)
    description = Column(Text, nullable=True)
    language = Column(Enum(Language), nullable=False)
    author_id = Column(Integer, ForeignKey("users.id"), nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow, nullable=False)

    # Publishing status
    is_published = Column(Boolean, default=False, nullable=False)
    published_at = Column(DateTime, nullable=True)

    # Relationships
    author = relationship("User", back_populates="modules")
    versions = relationship("ModuleVersion", back_populates="module", cascade="all, delete-orphan")

    @property
    def draft_version(self):
        """Get the draft version of this module."""
        for v in self.versions:
            if v.version_type == VersionType.DRAFT:
                return v
        return None

    @property
    def published_version(self):
        """Get the published version of this module."""
        for v in self.versions:
            if v.version_type == VersionType.PUBLISHED:
                return v
        return None


class ModuleVersion(Base):
    """A specific version (draft or published) of a module."""

    __tablename__ = "module_versions"

    id = Column(Integer, primary_key=True, index=True)
    module_id = Column(Integer, ForeignKey("modules.id"), nullable=False)
    version_type = Column(Enum(VersionType), nullable=False)

    # Build status
    build_status = Column(Enum(BuildStatus), default=BuildStatus.PENDING, nullable=False)
    build_log = Column(Text, nullable=True)
    built_at = Column(DateTime, nullable=True)

    # File paths (relative to storage root)
    source_path = Column(String(500), nullable=False)
    wasm_path = Column(String(500), nullable=True)

    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)

    # Relationships
    module = relationship("Module", back_populates="versions")
    test_runs = relationship("TestRun", back_populates="module_version", cascade="all, delete-orphan")
