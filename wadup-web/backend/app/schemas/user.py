"""User-related schemas."""
from datetime import datetime
from pydantic import BaseModel, Field


class LoginRequest(BaseModel):
    """Request to login or create a user."""
    username: str = Field(..., min_length=1, max_length=100)


class UserCreate(BaseModel):
    """Schema for creating a user."""
    username: str = Field(..., min_length=1, max_length=100)


class UserResponse(BaseModel):
    """User response model."""
    id: int
    username: str
    created_at: datetime

    class Config:
        from_attributes = True
