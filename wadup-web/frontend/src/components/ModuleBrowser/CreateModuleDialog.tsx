import { useState } from 'react'
import { X } from 'lucide-react'
import { useModuleStore } from '../../stores/moduleStore'
import type { Language } from '../../types'

interface CreateModuleDialogProps {
  onClose: () => void
  onCreated: (moduleId: number) => void
}

export default function CreateModuleDialog({ onClose, onCreated }: CreateModuleDialogProps) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [language, setLanguage] = useState<Language>('rust')
  const [isCreating, setIsCreating] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const { createModule } = useModuleStore()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return

    setIsCreating(true)
    setError(null)

    try {
      const module = await createModule(name.trim(), language, description.trim() || undefined)
      onCreated(module.id)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create module')
      setIsCreating(false)
    }
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <h2>Create New Module</h2>
          <button className="btn btn-ghost btn-icon" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="dialog-body">
          <div className="form-group">
            <label htmlFor="name">Module Name</label>
            <input
              id="name"
              type="text"
              className="input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-awesome-module"
              autoFocus
              disabled={isCreating}
            />
          </div>

          <div className="form-group">
            <label htmlFor="description">Description (optional)</label>
            <input
              id="description"
              type="text"
              className="input"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="A brief description of your module"
              disabled={isCreating}
            />
          </div>

          <div className="form-group">
            <label>Language</label>
            <div className="language-picker">
              <button
                type="button"
                className={`language-option ${language === 'rust' ? 'active' : ''}`}
                onClick={() => setLanguage('rust')}
                disabled={isCreating}
              >
                <span className="language-icon">🦀</span>
                <span>Rust</span>
              </button>
              <button
                type="button"
                className={`language-option ${language === 'go' ? 'active' : ''}`}
                onClick={() => setLanguage('go')}
                disabled={isCreating}
              >
                <span className="language-icon">🔷</span>
                <span>Go</span>
              </button>
              <button
                type="button"
                className={`language-option ${language === 'python' ? 'active' : ''}`}
                onClick={() => setLanguage('python')}
                disabled={isCreating}
              >
                <span className="language-icon">🐍</span>
                <span>Python</span>
              </button>
            </div>
          </div>

          {error && <p className="error-message">{error}</p>}

          <div className="dialog-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={onClose}
              disabled={isCreating}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={isCreating || !name.trim()}
            >
              {isCreating ? 'Creating...' : 'Create Module'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
