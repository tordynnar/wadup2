"""Sample and TestRun models."""
from datetime import datetime
from enum import Enum as PyEnum
from sqlalchemy import Column, Integer, String, Text, DateTime, ForeignKey, Enum, JSON
from sqlalchemy.orm import relationship

from app.database import Base


class TestStatus(str, PyEnum):
    """Test run status values."""
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"


class Sample(Base):
    """A test sample file uploaded by a user."""

    __tablename__ = "samples"

    id = Column(Integer, primary_key=True, index=True)
    owner_id = Column(Integer, ForeignKey("users.id"), nullable=False)
    filename = Column(String(255), nullable=False)
    file_path = Column(String(500), nullable=False)
    file_size = Column(Integer, nullable=False)
    content_type = Column(String(100), nullable=True)
    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)

    # Relationships
    owner = relationship("User", back_populates="samples")
    test_runs = relationship("TestRun", back_populates="sample", cascade="all, delete-orphan")


class TestRun(Base):
    """A test execution of a module against a sample."""

    __tablename__ = "test_runs"

    id = Column(Integer, primary_key=True, index=True)
    module_version_id = Column(Integer, ForeignKey("module_versions.id"), nullable=False)
    sample_id = Column(Integer, ForeignKey("samples.id"), nullable=False)

    # Execution status
    status = Column(Enum(TestStatus), default=TestStatus.PENDING, nullable=False)
    stdout = Column(Text, nullable=True)
    stderr = Column(Text, nullable=True)
    metadata_output = Column(JSON, nullable=True)  # Parsed metadata tables
    error_message = Column(Text, nullable=True)

    # Timestamps
    started_at = Column(DateTime, nullable=True)
    completed_at = Column(DateTime, nullable=True)
    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)

    # Relationships
    module_version = relationship("ModuleVersion", back_populates="test_runs")
    sample = relationship("Sample", back_populates="test_runs")
