import { Clock, User } from 'lucide-react'
import type { Module } from '../../types'

interface ModuleCardProps {
  module: Module
  onClick: () => void
}

export default function ModuleCard({ module, onClick }: ModuleCardProps) {
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
        {getBuildStatusBadge()}
        {module.is_published && (
          <span className="badge badge-info">Published</span>
        )}
      </div>
    </div>
  )
}
