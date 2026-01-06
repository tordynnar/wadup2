"""Initial migration

Revision ID: 001
Revises:
Create Date: 2024-01-01 00:00:00.000000

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa


revision: str = '001'
down_revision: Union[str, None] = None
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    # Users table
    op.create_table(
        'users',
        sa.Column('id', sa.Integer(), nullable=False),
        sa.Column('username', sa.String(length=100), nullable=False),
        sa.Column('created_at', sa.DateTime(), nullable=False),
        sa.PrimaryKeyConstraint('id')
    )
    op.create_index(op.f('ix_users_id'), 'users', ['id'], unique=False)
    op.create_index(op.f('ix_users_username'), 'users', ['username'], unique=True)

    # Modules table
    op.create_table(
        'modules',
        sa.Column('id', sa.Integer(), nullable=False),
        sa.Column('name', sa.String(length=200), nullable=False),
        sa.Column('description', sa.Text(), nullable=True),
        sa.Column('language', sa.Enum('RUST', 'GO', 'PYTHON', name='language'), nullable=False),
        sa.Column('author_id', sa.Integer(), nullable=False),
        sa.Column('created_at', sa.DateTime(), nullable=False),
        sa.Column('updated_at', sa.DateTime(), nullable=False),
        sa.Column('is_published', sa.Boolean(), nullable=False),
        sa.Column('published_at', sa.DateTime(), nullable=True),
        sa.ForeignKeyConstraint(['author_id'], ['users.id'], ),
        sa.PrimaryKeyConstraint('id')
    )
    op.create_index(op.f('ix_modules_id'), 'modules', ['id'], unique=False)
    op.create_index(op.f('ix_modules_name'), 'modules', ['name'], unique=False)

    # Module versions table
    op.create_table(
        'module_versions',
        sa.Column('id', sa.Integer(), nullable=False),
        sa.Column('module_id', sa.Integer(), nullable=False),
        sa.Column('version_type', sa.Enum('DRAFT', 'PUBLISHED', name='versiontype'), nullable=False),
        sa.Column('build_status', sa.Enum('PENDING', 'BUILDING', 'SUCCESS', 'FAILED', name='buildstatus'), nullable=False),
        sa.Column('build_log', sa.Text(), nullable=True),
        sa.Column('built_at', sa.DateTime(), nullable=True),
        sa.Column('source_path', sa.String(length=500), nullable=False),
        sa.Column('wasm_path', sa.String(length=500), nullable=True),
        sa.Column('created_at', sa.DateTime(), nullable=False),
        sa.ForeignKeyConstraint(['module_id'], ['modules.id'], ),
        sa.PrimaryKeyConstraint('id')
    )
    op.create_index(op.f('ix_module_versions_id'), 'module_versions', ['id'], unique=False)

    # Samples table
    op.create_table(
        'samples',
        sa.Column('id', sa.Integer(), nullable=False),
        sa.Column('owner_id', sa.Integer(), nullable=False),
        sa.Column('filename', sa.String(length=255), nullable=False),
        sa.Column('file_path', sa.String(length=500), nullable=False),
        sa.Column('file_size', sa.Integer(), nullable=False),
        sa.Column('content_type', sa.String(length=100), nullable=True),
        sa.Column('created_at', sa.DateTime(), nullable=False),
        sa.ForeignKeyConstraint(['owner_id'], ['users.id'], ),
        sa.PrimaryKeyConstraint('id')
    )
    op.create_index(op.f('ix_samples_id'), 'samples', ['id'], unique=False)

    # Test runs table
    op.create_table(
        'test_runs',
        sa.Column('id', sa.Integer(), nullable=False),
        sa.Column('module_version_id', sa.Integer(), nullable=False),
        sa.Column('sample_id', sa.Integer(), nullable=False),
        sa.Column('status', sa.Enum('PENDING', 'RUNNING', 'SUCCESS', 'FAILED', name='teststatus'), nullable=False),
        sa.Column('stdout', sa.Text(), nullable=True),
        sa.Column('stderr', sa.Text(), nullable=True),
        sa.Column('metadata_output', sa.JSON(), nullable=True),
        sa.Column('error_message', sa.Text(), nullable=True),
        sa.Column('started_at', sa.DateTime(), nullable=True),
        sa.Column('completed_at', sa.DateTime(), nullable=True),
        sa.Column('created_at', sa.DateTime(), nullable=False),
        sa.ForeignKeyConstraint(['module_version_id'], ['module_versions.id'], ),
        sa.ForeignKeyConstraint(['sample_id'], ['samples.id'], ),
        sa.PrimaryKeyConstraint('id')
    )
    op.create_index(op.f('ix_test_runs_id'), 'test_runs', ['id'], unique=False)


def downgrade() -> None:
    op.drop_index(op.f('ix_test_runs_id'), table_name='test_runs')
    op.drop_table('test_runs')
    op.drop_index(op.f('ix_samples_id'), table_name='samples')
    op.drop_table('samples')
    op.drop_index(op.f('ix_module_versions_id'), table_name='module_versions')
    op.drop_table('module_versions')
    op.drop_index(op.f('ix_modules_name'), table_name='modules')
    op.drop_index(op.f('ix_modules_id'), table_name='modules')
    op.drop_table('modules')
    op.drop_index(op.f('ix_users_username'), table_name='users')
    op.drop_index(op.f('ix_users_id'), table_name='users')
    op.drop_table('users')
