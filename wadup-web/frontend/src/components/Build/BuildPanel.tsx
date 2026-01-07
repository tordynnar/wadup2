import { useState, useEffect, useRef } from 'react'
import { X, Play, CheckCircle, XCircle, Loader } from 'lucide-react'
import { modulesApi } from '../../api/modules'
import { streamBuildLogs, type BuildEvent } from '../../api/build'
import { useModuleStore } from '../../stores/moduleStore'

interface BuildPanelProps {
  moduleId: number
  onClose: () => void
}

type Status = 'idle' | 'building' | 'success' | 'failed'

export default function BuildPanel({ moduleId, onClose }: BuildPanelProps) {
  const { loadModule } = useModuleStore()
  const [status, setStatus] = useState<Status>('idle')
  const [logs, setLogs] = useState<string[]>([])
  const [error, setError] = useState<string | null>(null)
  const logRef = useRef<HTMLDivElement>(null)
  const cleanupRef = useRef<(() => void) | null>(null)

  // Auto-scroll logs
  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight
    }
  }, [logs])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (cleanupRef.current) {
        cleanupRef.current()
      }
    }
  }, [])

  const startBuild = async () => {
    setStatus('building')
    setLogs([])
    setError(null)

    try {
      // Start the build
      await modulesApi.startBuild(moduleId)

      // Stream the logs
      cleanupRef.current = streamBuildLogs(
        moduleId,
        (event: BuildEvent) => {
          if (event.type === 'log' && event.content) {
            setLogs((prev) => [...prev, event.content!])
          } else if (event.type === 'complete') {
            setStatus(event.status === 'success' ? 'success' : 'failed')
            // Reload module to get updated build status
            loadModule(moduleId)
          }
        },
        (err) => {
          setError(err.message)
          setStatus('failed')
        }
      )
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start build')
      setStatus('failed')
    }
  }

  const getStatusIcon = () => {
    switch (status) {
      case 'building':
        return <Loader size={18} className="animate-spin" />
      case 'success':
        return <CheckCircle size={18} />
      case 'failed':
        return <XCircle size={18} />
      default:
        return null
    }
  }

  const getStatusText = () => {
    switch (status) {
      case 'building':
        return 'Building...'
      case 'success':
        return 'Build successful'
      case 'failed':
        return 'Build failed'
      default:
        return 'Ready to build'
    }
  }

  return (
    <div className="build-panel">
      <div className="build-panel-header">
        <h3>Build</h3>
        <button className="btn btn-ghost btn-icon" onClick={onClose}>
          <X size={18} />
        </button>
      </div>

      <div className="build-panel-content">
        <div className={`build-status ${status}`}>
          {getStatusIcon()}
          <span>{getStatusText()}</span>
        </div>

        {error && (
          <div className="build-error">
            <p>{error}</p>
          </div>
        )}

        {logs.length > 0 && (
          <div className="build-log" ref={logRef}>
            {logs.map((line, i) => (
              <div key={i} className="build-log-line">
                {line}
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="build-panel-actions">
        <button
          className="btn btn-primary w-full"
          onClick={startBuild}
          disabled={status === 'building'}
        >
          <Play size={18} />
          {status === 'building' ? 'Building...' : 'Start Build'}
        </button>
      </div>
    </div>
  )
}
