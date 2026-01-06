"""Authentication router."""
from fastapi import APIRouter, Depends, HTTPException, Response, Cookie
from sqlalchemy.orm import Session
from typing import Optional

from app.database import get_db
from app.models.user import User
from app.schemas.user import LoginRequest, UserResponse

router = APIRouter(prefix="/api/auth", tags=["auth"])


def get_current_user(
    user_id: Optional[str] = Cookie(default=None, alias="wadup_user_id"),
    db: Session = Depends(get_db),
) -> Optional[User]:
    """Get the current logged-in user from cookie."""
    if not user_id:
        return None
    try:
        uid = int(user_id)
    except ValueError:
        return None
    return db.query(User).filter(User.id == uid).first()


def require_user(
    user: Optional[User] = Depends(get_current_user),
) -> User:
    """Require a logged-in user."""
    if not user:
        raise HTTPException(status_code=401, detail="Not authenticated")
    return user


@router.post("/login", response_model=UserResponse)
def login(request: LoginRequest, response: Response, db: Session = Depends(get_db)):
    """Login or create a user with the given username."""
    # Check if user exists
    user = db.query(User).filter(User.username == request.username).first()

    if not user:
        # Create new user
        user = User(username=request.username)
        db.add(user)
        db.commit()
        db.refresh(user)

    # Set cookie for authentication
    response.set_cookie(
        key="wadup_user_id",
        value=str(user.id),
        httponly=True,
        samesite="lax",
        max_age=60 * 60 * 24 * 30,  # 30 days
    )

    return user


@router.get("/me", response_model=Optional[UserResponse])
def get_me(user: Optional[User] = Depends(get_current_user)):
    """Get the current logged-in user."""
    return user


@router.post("/logout")
def logout(response: Response):
    """Logout the current user."""
    response.delete_cookie(key="wadup_user_id")
    return {"message": "Logged out"}
