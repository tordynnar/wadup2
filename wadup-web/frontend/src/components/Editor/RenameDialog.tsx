import { useState, useEffect, useRef } from 'react'
import { X } from 'lucide-react'
import './RenameDialog.css'

interface RenameDialogProps {
  isOpen: boolean
  currentPath: string
  onClose: () => void
  onRename: (newPath: string) => void
}

export default function RenameDialog({
  isOpen,
  currentPath,
  onClose,
  onRename,
}: RenameDialogProps) {
  const [newName, setNewName] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (isOpen) {
      // Extract just the filename from the path
      const parts = currentPath.split('/')
      const filename = parts.pop() || ''
      setNewName(filename)

      // Focus and select the filename (without extension) after a short delay
      setTimeout(() => {
        if (inputRef.current) {
          inputRef.current.focus()
          const dotIndex = filename.lastIndexOf('.')
          if (dotIndex > 0) {
            inputRef.current.setSelectionRange(0, dotIndex)
          } else {
            inputRef.current.select()
          }
        }
      }, 50)
    }
  }, [isOpen, currentPath])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!newName.trim()) return

    // Construct the new full path
    const parts = currentPath.split('/')
    parts.pop()
    const newPath = [...parts, newName.trim()].join('/')

    onRename(newPath)
    onClose()
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose()
    }
  }

  if (!isOpen) return null

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog rename-dialog"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="dialog-header">
          <h3>Rename</h3>
          <button className="btn btn-ghost btn-icon" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="dialog-body">
            <input
              ref={inputRef}
              type="text"
              className="input"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="Enter new name"
            />
          </div>

          <div className="dialog-footer">
            <button type="button" className="btn btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={!newName.trim() || newName === currentPath.split('/').pop()}
            >
              Rename
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
