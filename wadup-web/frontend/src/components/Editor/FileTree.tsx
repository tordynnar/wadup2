import { useState } from 'react'
import { ChevronRight, ChevronDown, File, Folder, FolderOpen } from 'lucide-react'
import type { FileTreeNode } from '../../types'
import FileContextMenu from './FileContextMenu'

interface ContextMenuState {
  x: number
  y: number
  path: string
  type: 'file' | 'directory'
}

interface DragState {
  targetPath: string
  position: 'inside' | 'before' | 'after'
}

interface FileTreeProps {
  tree: FileTreeNode
  currentFile: string | null
  onFileSelect: (path: string) => void
  onRename?: (path: string) => void
  onDelete?: (path: string) => void
  onCreateFile?: (parentPath: string) => void
  onCreateFolder?: (parentPath: string) => void
  onMove?: (sourcePath: string, targetPath: string) => void
}

export default function FileTree({
  tree,
  currentFile,
  onFileSelect,
  onRename,
  onDelete,
  onCreateFile,
  onCreateFolder,
  onMove,
}: FileTreeProps) {
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)
  const [dragState, setDragState] = useState<DragState | null>(null)
  const [draggingPath, setDraggingPath] = useState<string | null>(null)

  const handleContextMenu = (
    e: React.MouseEvent,
    path: string,
    type: 'file' | 'directory'
  ) => {
    e.preventDefault()
    setContextMenu({ x: e.clientX, y: e.clientY, path, type })
  }

  const handleDragStart = (e: React.DragEvent, path: string) => {
    e.dataTransfer.setData('text/plain', path)
    e.dataTransfer.effectAllowed = 'move'
    setDraggingPath(path)
  }

  const handleDragEnd = () => {
    setDraggingPath(null)
    setDragState(null)
  }

  const handleDragOver = (
    e: React.DragEvent,
    path: string,
    isDirectory: boolean,
    rect: DOMRect
  ) => {
    e.preventDefault()
    e.stopPropagation()
    e.dataTransfer.dropEffect = 'move'

    // Don't allow dropping onto self or into own subdirectory
    if (draggingPath && (path === draggingPath || path.startsWith(draggingPath + '/'))) {
      return
    }

    const y = e.clientY - rect.top
    const height = rect.height

    if (isDirectory) {
      // For directories: top 25% = before, middle 50% = inside, bottom 25% = after
      if (y < height * 0.25) {
        setDragState({ targetPath: path, position: 'before' })
      } else if (y > height * 0.75) {
        setDragState({ targetPath: path, position: 'after' })
      } else {
        setDragState({ targetPath: path, position: 'inside' })
      }
    } else {
      // For files: top 50% = before, bottom 50% = after
      if (y < height * 0.5) {
        setDragState({ targetPath: path, position: 'before' })
      } else {
        setDragState({ targetPath: path, position: 'after' })
      }
    }
  }

  const handleDragLeave = (e: React.DragEvent) => {
    // Only clear if leaving the file tree entirely
    const relatedTarget = e.relatedTarget as HTMLElement
    if (!relatedTarget || !e.currentTarget.contains(relatedTarget)) {
      setDragState(null)
    }
  }

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()

    const sourcePath = e.dataTransfer.getData('text/plain')
    if (!sourcePath || !dragState) {
      setDragState(null)
      setDraggingPath(null)
      return
    }

    const { targetPath, position } = dragState

    // Don't allow dropping onto self
    if (sourcePath === targetPath) {
      setDragState(null)
      setDraggingPath(null)
      return
    }

    // Don't allow dropping into own subdirectory
    if (targetPath.startsWith(sourcePath + '/')) {
      setDragState(null)
      setDraggingPath(null)
      return
    }

    // Calculate the destination path
    const fileName = sourcePath.split('/').pop()!
    let newPath: string

    if (position === 'inside') {
      // Drop into a folder
      newPath = targetPath ? `${targetPath}/${fileName}` : fileName
    } else {
      // Drop before/after - put in the same parent folder
      const parentPath = targetPath.includes('/')
        ? targetPath.substring(0, targetPath.lastIndexOf('/'))
        : ''
      newPath = parentPath ? `${parentPath}/${fileName}` : fileName
    }

    if (onMove && newPath !== sourcePath) {
      onMove(sourcePath, newPath)
    }

    setDragState(null)
    setDraggingPath(null)
  }

  return (
    <div
      className="file-tree"
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onDragOver={(e) => e.preventDefault()}
    >
      {tree.children?.map((child) => (
        <FileTreeItem
          key={child.name}
          node={child}
          currentFile={currentFile}
          onFileSelect={onFileSelect}
          onContextMenu={handleContextMenu}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
          onDragOver={handleDragOver}
          dragState={dragState}
          draggingPath={draggingPath}
          depth={0}
        />
      ))}

      {contextMenu && onRename && onDelete && onCreateFile && onCreateFolder && (
        <FileContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          targetPath={contextMenu.path}
          targetType={contextMenu.type}
          onClose={() => setContextMenu(null)}
          onRename={onRename}
          onDelete={onDelete}
          onCreateFile={onCreateFile}
          onCreateFolder={onCreateFolder}
        />
      )}
    </div>
  )
}

interface FileTreeItemProps {
  node: FileTreeNode
  currentFile: string | null
  onFileSelect: (path: string) => void
  onContextMenu: (e: React.MouseEvent, path: string, type: 'file' | 'directory') => void
  onDragStart: (e: React.DragEvent, path: string) => void
  onDragEnd: () => void
  onDragOver: (e: React.DragEvent, path: string, isDirectory: boolean, rect: DOMRect) => void
  dragState: DragState | null
  draggingPath: string | null
  depth: number
}

function FileTreeItem({
  node,
  currentFile,
  onFileSelect,
  onContextMenu,
  onDragStart,
  onDragEnd,
  onDragOver,
  dragState,
  draggingPath,
  depth,
}: FileTreeItemProps) {
  const [isExpanded, setIsExpanded] = useState(depth < 2)
  const isDirectory = node.type === 'directory'
  const isActive = node.path === currentFile
  const isDragging = draggingPath === node.path

  // Determine drop indicator state
  const showDropBefore = dragState?.targetPath === node.path && dragState?.position === 'before'
  const showDropAfter = dragState?.targetPath === node.path && dragState?.position === 'after'
  const showDropInside = dragState?.targetPath === node.path && dragState?.position === 'inside'

  const handleClick = () => {
    if (isDirectory) {
      setIsExpanded(!isExpanded)
    } else if (node.path) {
      onFileSelect(node.path)
    }
  }

  const handleContextMenu = (e: React.MouseEvent) => {
    if (node.path) {
      onContextMenu(e, node.path, isDirectory ? 'directory' : 'file')
    }
  }

  const handleDragStart = (e: React.DragEvent) => {
    if (node.path) {
      onDragStart(e, node.path)
    }
  }

  const handleDragOver = (e: React.DragEvent) => {
    if (node.path) {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
      onDragOver(e, node.path, isDirectory, rect)
    }
  }

  const getFileIcon = () => {
    if (isDirectory) {
      return isExpanded ? (
        <FolderOpen size={16} className="icon-folder" />
      ) : (
        <Folder size={16} className="icon-folder" />
      )
    }

    const ext = node.name.split('.').pop()?.toLowerCase()
    const iconClass = getFileIconClass(ext)
    return <File size={16} className={iconClass} />
  }

  return (
    <div className={`file-tree-item ${isDragging ? 'dragging' : ''}`}>
      {showDropBefore && <div className="drop-indicator drop-indicator-before" style={{ marginLeft: `${depth * 16 + 8}px` }} />}
      <div
        className={`file-tree-row ${isActive ? 'active' : ''} ${showDropInside ? 'drag-over' : ''}`}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        draggable
        onDragStart={handleDragStart}
        onDragEnd={onDragEnd}
        onDragOver={handleDragOver}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
      >
        {isDirectory && (
          <span className="file-tree-chevron">
            {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </span>
        )}
        {!isDirectory && <span className="file-tree-chevron" />}
        {getFileIcon()}
        <span className="file-tree-name">{node.name}</span>
      </div>
      {showDropAfter && !isExpanded && <div className="drop-indicator drop-indicator-after" style={{ marginLeft: `${depth * 16 + 8}px` }} />}

      {isDirectory && isExpanded && node.children && (
        <div className="file-tree-children">
          {node.children.map((child) => (
            <FileTreeItem
              key={child.name}
              node={child}
              currentFile={currentFile}
              onFileSelect={onFileSelect}
              onContextMenu={onContextMenu}
              onDragStart={onDragStart}
              onDragEnd={onDragEnd}
              onDragOver={onDragOver}
              dragState={dragState}
              draggingPath={draggingPath}
              depth={depth + 1}
            />
          ))}
          {showDropAfter && <div className="drop-indicator drop-indicator-after" style={{ marginLeft: `${(depth + 1) * 16 + 8}px` }} />}
        </div>
      )}
    </div>
  )
}

function getFileIconClass(ext?: string): string {
  const extMap: Record<string, string> = {
    rs: 'icon-rust',
    go: 'icon-go',
    py: 'icon-python',
    toml: 'icon-config',
    json: 'icon-config',
    md: 'icon-markdown',
    mod: 'icon-go',
    sum: 'icon-config',
  }
  return extMap[ext || ''] || 'icon-default'
}
