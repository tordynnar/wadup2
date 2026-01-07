import { X, AlertTriangle } from 'lucide-react'
import './UnsavedChangesDialog.css'

interface UnsavedChangesDialogProps {
  isOpen: boolean
  fileName: string
  onSave: () => void
  onDiscard: () => void
  onCancel: () => void
}

export default function UnsavedChangesDialog({
  isOpen,
  fileName,
  onSave,
  onDiscard,
  onCancel,
}: UnsavedChangesDialogProps) {
  if (!isOpen) return null

  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div
        className="dialog unsaved-changes-dialog"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-header">
          <h3>
            <AlertTriangle size={18} />
            Unsaved Changes
          </h3>
          <button className="btn btn-ghost btn-icon" onClick={onCancel}>
            <X size={18} />
          </button>
        </div>

        <div className="dialog-body">
          <p>
            <strong>{fileName}</strong> has unsaved changes. Would you like to save before closing?
          </p>
        </div>

        <div className="dialog-footer">
          <button className="btn btn-secondary" onClick={onDiscard}>
            Don't Save
          </button>
          <button className="btn btn-secondary" onClick={onCancel}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={onSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  )
}
