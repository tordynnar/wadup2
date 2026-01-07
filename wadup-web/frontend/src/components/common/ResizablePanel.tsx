import { useState, useRef, useCallback, ReactNode } from 'react'
import './ResizablePanel.css'

interface ResizablePanelProps {
  children: ReactNode
  defaultWidth: number
  minWidth: number
  maxWidth: number
  side: 'left' | 'right'
  className?: string
}

export default function ResizablePanel({
  children,
  defaultWidth,
  minWidth,
  maxWidth,
  side,
  className = '',
}: ResizablePanelProps) {
  const [width, setWidth] = useState(defaultWidth)
  const isResizing = useRef(false)
  const startX = useRef(0)
  const startWidth = useRef(0)

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    isResizing.current = true
    startX.current = e.clientX
    startWidth.current = width
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'

    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current) return

      const delta = side === 'left'
        ? e.clientX - startX.current
        : startX.current - e.clientX

      const newWidth = Math.min(maxWidth, Math.max(minWidth, startWidth.current + delta))
      setWidth(newWidth)
    }

    const handleMouseUp = () => {
      isResizing.current = false
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }, [width, minWidth, maxWidth, side])

  return (
    <div className={`resizable-panel ${className}`} style={{ width }}>
      {children}
      <div
        className={`resize-handle resize-handle-${side}`}
        onMouseDown={handleMouseDown}
      />
    </div>
  )
}
