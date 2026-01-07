import { Clock, User, Trash2 } from 'lucide-react'
import type { Module } from '../../types'

interface ModuleCardProps {
  module: Module
  onClick: () => void
  onDelete?: () => void
  isOwner?: boolean
}

export default function ModuleCard({ module, onClick, onDelete, isOwner }: ModuleCardProps) {
  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr)
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  }

  const getBuildStatusBadge = () => {
    const status = module.draft_version?.build_status || 'pending'
    const statusMap: Record<string, { label: string; class: string }> = {
      pending: { label: 'Not built', class: 'badge-pending' },
      building: { label: 'Building', class: 'badge-warning' },
      success: { label: 'Built', class: 'badge-success' },
      failed: { label: 'Build failed', class: 'badge-error' },
    }
    const { label, class: className } = statusMap[status] || statusMap.pending
    return <span className={`badge ${className}`}>{label}</span>
  }

  return (
    <div className="module-card" onClick={onClick}>
      <div className="module-card-header">
        <h3 className="module-card-title">{module.name}</h3>
        <span className={`badge badge-${module.language}`}>{module.language}</span>
      </div>

      {module.description && (
        <p className="module-card-description">{module.description}</p>
      )}

      <div className="module-card-meta">
        <div className="module-card-meta-item">
          <User size={14} />
          <span>{module.author_username || 'Unknown'}</span>
        </div>
        <div className="module-card-meta-item">
          <Clock size={14} />
          <span>{formatDate(module.updated_at)}</span>
        </div>
      </div>

      <div className="module-card-footer">
        <div className="module-card-badges">
          {getBuildStatusBadge()}
          {module.is_published && (
            <span className="badge badge-info">Published</span>
          )}
        </div>
        {isOwner && onDelete && (
          <button
            className="btn btn-ghost btn-icon btn-sm module-card-delete"
            onClick={(e) => {
              e.stopPropagation()
              onDelete()
            }}
            title="Delete module"
          >
            <Trash2 size={16} />
          </button>
        )}
      </div>
    </div>
  )
}
