"""Sample and test-related schemas."""
from datetime import datetime
from typing import Optional, List, Any
from pydantic import BaseModel

from app.models.sample import TestStatus


class SampleResponse(BaseModel):
    """Sample file response model."""
    id: int
    filename: str
    file_size: int
    content_type: Optional[str] = None
    created_at: datetime

    class Config:
        from_attributes = True


class TestRunCreate(BaseModel):
    """Request to create a test run."""
    sample_ids: List[int]


class TestRunResponse(BaseModel):
    """Test run response model."""
    id: int
    module_version_id: int
    sample_id: int
    status: TestStatus
    stdout: Optional[str] = None
    stderr: Optional[str] = None
    metadata_output: Optional[Any] = None
    subcontent_output: Optional[Any] = None
    error_message: Optional[str] = None
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    created_at: datetime

    class Config:
        from_attributes = True
