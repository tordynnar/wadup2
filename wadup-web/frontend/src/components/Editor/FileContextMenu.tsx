import { useEffect, useRef } from 'react'
import { FilePlus, FolderPlus, Pencil, Trash2 } from 'lucide-react'
import './FileContextMenu.css'

interface FileContextMenuProps {
  x: number
  y: number
  targetPath: string
  targetType: 'file' | 'directory'
  onClose: () => void
  onRename: (path: string) => void
  onDelete: (path: string) => void
  onCreateFile: (parentPath: string) => void
  onCreateFolder: (parentPath: string) => void
  showRenameDelete?: boolean
}

export default function FileContextMenu({
  x,
  y,
  targetPath,
  targetType,
  onClose,
  onRename,
  onDelete,
  onCreateFile,
  onCreateFolder,
  showRenameDelete = true,
}: FileContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleKeyDown)

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [onClose])

  // Adjust position to keep menu in viewport
  useEffect(() => {
    if (menuRef.current) {
      const rect = menuRef.current.getBoundingClientRect()
      const viewportWidth = window.innerWidth
      const viewportHeight = window.innerHeight

      if (rect.right > viewportWidth) {
        menuRef.current.style.left = `${x - rect.width}px`
      }
      if (rect.bottom > viewportHeight) {
        menuRef.current.style.top = `${y - rect.height}px`
      }
    }
  }, [x, y])

  const getParentPath = () => {
    if (targetType === 'directory') {
      return targetPath
    }
    const parts = targetPath.split('/')
    parts.pop()
    return parts.join('/') || ''
  }

  return (
    <div
      ref={menuRef}
      className="file-context-menu"
      style={{ left: x, top: y }}
    >
      <button
        className="context-menu-item"
        onClick={() => {
          onCreateFile(getParentPath())
          onClose()
        }}
      >
        <FilePlus size={14} />
        <span>New File</span>
      </button>
      <button
        className="context-menu-item"
        onClick={() => {
          onCreateFolder(getParentPath())
          onClose()
        }}
      >
        <FolderPlus size={14} />
        <span>New Folder</span>
      </button>
      {showRenameDelete && (
        <>
          <div className="context-menu-separator" />
          <button
            className="context-menu-item"
            onClick={() => {
              onRename(targetPath)
              onClose()
            }}
          >
            <Pencil size={14} />
            <span>Rename</span>
          </button>
          <button
            className="context-menu-item context-menu-item-danger"
            onClick={() => {
              onDelete(targetPath)
              onClose()
            }}
          >
            <Trash2 size={14} />
            <span>Delete</span>
          </button>
        </>
      )}
    </div>
  )
}
