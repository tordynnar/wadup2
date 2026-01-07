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
  const [dragOverPath, setDragOverPath] = useState<string | null>(null)

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
  }

  const handleDragOver = (e: React.DragEvent, path: string, isDirectory: boolean) => {
    if (isDirectory) {
      e.preventDefault()
      e.dataTransfer.dropEffect = 'move'
      setDragOverPath(path)
    }
  }

  const handleDragLeave = () => {
    setDragOverPath(null)
  }

  const handleDrop = (e: React.DragEvent, targetPath: string) => {
    e.preventDefault()
    setDragOverPath(null)

    const sourcePath = e.dataTransfer.getData('text/plain')
    if (!sourcePath || sourcePath === targetPath) return

    // Don't allow dropping into own subdirectory
    if (targetPath.startsWith(sourcePath + '/')) return

    // Calculate new path
    const fileName = sourcePath.split('/').pop()
    const newPath = targetPath ? `${targetPath}/${fileName}` : fileName!

    if (onMove) {
      onMove(sourcePath, newPath)
    }
  }

  return (
    <div className="file-tree" onDragLeave={handleDragLeave}>
      {tree.children?.map((child) => (
        <FileTreeItem
          key={child.name}
          node={child}
          currentFile={currentFile}
          onFileSelect={onFileSelect}
          onContextMenu={handleContextMenu}
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
          dragOverPath={dragOverPath}
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
  onDragOver: (e: React.DragEvent, path: string, isDirectory: boolean) => void
  onDrop: (e: React.DragEvent, targetPath: string) => void
  dragOverPath: string | null
  depth: number
}

function FileTreeItem({
  node,
  currentFile,
  onFileSelect,
  onContextMenu,
  onDragStart,
  onDragOver,
  onDrop,
  dragOverPath,
  depth,
}: FileTreeItemProps) {
  const [isExpanded, setIsExpanded] = useState(depth < 2)
  const isDirectory = node.type === 'directory'
  const isActive = node.path === currentFile
  const isDragOver = dragOverPath === node.path

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
      onDragOver(e, node.path, isDirectory)
    }
  }

  const handleDrop = (e: React.DragEvent) => {
    if (node.path && isDirectory) {
      onDrop(e, node.path)
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
    <div className="file-tree-item">
      <div
        className={`file-tree-row ${isActive ? 'active' : ''} ${isDragOver ? 'drag-over' : ''}`}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        draggable
        onDragStart={handleDragStart}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
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
              onDragOver={onDragOver}
              onDrop={onDrop}
              dragOverPath={dragOverPath}
              depth={depth + 1}
            />
          ))}
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
