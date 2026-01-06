"""API routers."""
from app.routers.auth import router as auth_router
from app.routers.modules import router as modules_router
from app.routers.files import router as files_router
from app.routers.build import router as build_router
from app.routers.samples import router as samples_router
from app.routers.test import router as test_router

__all__ = [
    "auth_router",
    "modules_router",
    "files_router",
    "build_router",
    "samples_router",
    "test_router",
]
