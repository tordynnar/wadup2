import { useState, useEffect, useRef } from 'react'
import { X, FilePlus, FolderPlus } from 'lucide-react'
import './CreateFileDialog.css'

interface CreateFileDialogProps {
  isOpen: boolean
  type: 'file' | 'folder'
  parentPath: string
  onClose: () => void
  onCreate: (path: string) => void
}

export default function CreateFileDialog({
  isOpen,
  type,
  parentPath,
  onClose,
  onCreate,
}: CreateFileDialogProps) {
  const [name, setName] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (isOpen) {
      setName('')
      setTimeout(() => {
        inputRef.current?.focus()
      }, 50)
    }
  }, [isOpen])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return

    const fullPath = parentPath ? `${parentPath}/${name.trim()}` : name.trim()
    onCreate(fullPath)
    onClose()
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose()
    }
  }

  if (!isOpen) return null

  const Icon = type === 'file' ? FilePlus : FolderPlus
  const title = type === 'file' ? 'New File' : 'New Folder'
  const placeholder = type === 'file' ? 'filename.py' : 'folder-name'

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog create-file-dialog"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="dialog-header">
          <h3>
            <Icon size={18} />
            {title}
          </h3>
          <button className="btn btn-ghost btn-icon" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="dialog-body">
            {parentPath && (
              <div className="create-file-parent">
                <span className="text-muted">in</span> {parentPath}/
              </div>
            )}
            <input
              ref={inputRef}
              type="text"
              className="input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={placeholder}
            />
          </div>

          <div className="dialog-footer">
            <button type="button" className="btn btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={!name.trim()}
            >
              Create
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
