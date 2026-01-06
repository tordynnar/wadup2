import { useState } from 'react'
import { ChevronRight, ChevronDown, File, Folder, FolderOpen } from 'lucide-react'
import type { FileTreeNode } from '../../types'

interface FileTreeProps {
  tree: FileTreeNode
  currentFile: string | null
  onFileSelect: (path: string) => void
}

export default function FileTree({ tree, currentFile, onFileSelect }: FileTreeProps) {
  return (
    <div className="file-tree">
      {tree.children?.map((child) => (
        <FileTreeItem
          key={child.name}
          node={child}
          currentFile={currentFile}
          onFileSelect={onFileSelect}
          depth={0}
        />
      ))}
    </div>
  )
}

interface FileTreeItemProps {
  node: FileTreeNode
  currentFile: string | null
  onFileSelect: (path: string) => void
  depth: number
}

function FileTreeItem({ node, currentFile, onFileSelect, depth }: FileTreeItemProps) {
  const [isExpanded, setIsExpanded] = useState(depth < 2)
  const isDirectory = node.type === 'directory'
  const isActive = node.path === currentFile

  const handleClick = () => {
    if (isDirectory) {
      setIsExpanded(!isExpanded)
    } else if (node.path) {
      onFileSelect(node.path)
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
        className={`file-tree-row ${isActive ? 'active' : ''}`}
        onClick={handleClick}
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
