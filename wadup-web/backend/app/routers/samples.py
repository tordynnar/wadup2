"""Samples router for test sample management."""
import os
import uuid
import aiofiles
from fastapi import APIRouter, Depends, HTTPException, UploadFile, File
from sqlalchemy.orm import Session
from typing import List

from app.database import get_db
from app.models.user import User
from app.models.sample import Sample
from app.schemas.sample import SampleResponse
from app.routers.auth import require_user
from app.config import settings

router = APIRouter(prefix="/api/samples", tags=["samples"])

MAX_SAMPLE_SIZE = 100 * 1024 * 1024  # 100 MB


@router.get("", response_model=List[SampleResponse])
def list_samples(
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """List all samples owned by the current user."""
    samples = db.query(Sample).filter(Sample.owner_id == user.id).order_by(Sample.created_at.desc()).all()
    return samples


@router.post("", response_model=SampleResponse)
async def upload_sample(
    file: UploadFile = File(...),
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Upload a sample file."""
    # Validate file size
    content = await file.read()
    if len(content) > MAX_SAMPLE_SIZE:
        raise HTTPException(status_code=400, detail=f"File too large. Maximum size is {MAX_SAMPLE_SIZE // 1024 // 1024} MB")

    # Generate unique filename
    ext = os.path.splitext(file.filename or "")[1]
    unique_name = f"{uuid.uuid4()}{ext}"
    file_path = f"samples/{user.id}/{unique_name}"
    full_path = settings.storage_root / file_path

    # Ensure directory exists
    full_path.parent.mkdir(parents=True, exist_ok=True)

    # Save file
    async with aiofiles.open(full_path, "wb") as f:
        await f.write(content)

    # Create database record
    sample = Sample(
        owner_id=user.id,
        filename=file.filename or "unnamed",
        file_path=file_path,
        file_size=len(content),
        content_type=file.content_type,
    )
    db.add(sample)
    db.commit()
    db.refresh(sample)

    return sample


@router.delete("/{sample_id}")
def delete_sample(
    sample_id: int,
    user: User = Depends(require_user),
    db: Session = Depends(get_db),
):
    """Delete a sample file."""
    sample = db.query(Sample).filter(Sample.id == sample_id).first()
    if not sample:
        raise HTTPException(status_code=404, detail="Sample not found")

    if sample.owner_id != user.id:
        raise HTTPException(status_code=403, detail="Not authorized to delete this sample")

    # Delete file
    full_path = settings.storage_root / sample.file_path
    if full_path.exists():
        full_path.unlink()

    # Delete database record
    db.delete(sample)
    db.commit()

    return {"message": "Sample deleted"}
