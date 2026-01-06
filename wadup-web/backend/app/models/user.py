"""User model."""
from datetime import datetime
from sqlalchemy import Column, Integer, String, DateTime
from sqlalchemy.orm import relationship

from app.database import Base


class User(Base):
    """User account."""

    __tablename__ = "users"

    id = Column(Integer, primary_key=True, index=True)
    username = Column(String(100), unique=True, nullable=False, index=True)
    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)

    # Relationships
    modules = relationship("Module", back_populates="author", cascade="all, delete-orphan")
    samples = relationship("Sample", back_populates="owner", cascade="all, delete-orphan")
