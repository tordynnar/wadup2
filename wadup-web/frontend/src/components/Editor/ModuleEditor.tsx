import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Play, Upload, FlaskConical } from 'lucide-react'
import { useModuleStore } from '../../stores/moduleStore'
import { modulesApi } from '../../api/modules'
import FileTree from './FileTree'
import MonacoEditor from './MonacoEditor'
import BuildPanel from '../Build/BuildPanel'
import TestPanel from '../Test/TestPanel'
import './ModuleEditor.css'

export default function ModuleEditor() {
  const { moduleId } = useParams<{ moduleId: string }>()
  const navigate = useNavigate()
  const {
    currentModule,
    fileTree,
    currentFile,
    fileContent,
    isLoadingModule,
    isLoadingFile,
    isDirty,
    loadModule,
    loadFile,
    setFileContent,
    saveFile,
    clearCurrentModule,
  } = useModuleStore()
  const [showBuildPanel, setShowBuildPanel] = useState(false)
  const [showTestPanel, setShowTestPanel] = useState(false)
  const [isPublishing, setIsPublishing] = useState(false)

  useEffect(() => {
    if (moduleId) {
      loadModule(parseInt(moduleId))
    }
    return () => clearCurrentModule()
  }, [moduleId, loadModule, clearCurrentModule])

  const handleFileSelect = (path: string) => {
    if (moduleId && currentModule) {
      loadFile(parseInt(moduleId), path)
    }
  }

  const handleContentChange = (value: string | undefined) => {
    if (value !== undefined) {
      setFileContent(value)
    }
  }

  const handleSave = async () => {
    if (moduleId && currentFile && isDirty) {
      await saveFile(parseInt(moduleId), currentFile, fileContent)
    }
  }

  // Auto-save on Ctrl+S / Cmd+S
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault()
        handleSave()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleSave])

  if (isLoadingModule && !currentModule) {
    return (
      <div className="editor-loading">
        <div className="loading-spinner" />
        <p>Loading module...</p>
      </div>
    )
  }

  if (!currentModule) {
    return (
      <div className="editor-error">
        <p>Module not found</p>
        <button className="btn btn-secondary" onClick={() => navigate('/modules')}>
          Back to Modules
        </button>
      </div>
    )
  }

  const isOwner = true // TODO: Check against current user

  const handlePublish = async () => {
    if (!moduleId) return
    setIsPublishing(true)
    try {
      await modulesApi.publish(parseInt(moduleId))
      await loadModule(parseInt(moduleId))
    } catch (error) {
      console.error('Publish failed:', error)
    } finally {
      setIsPublishing(false)
    }
  }

  const canTest = currentModule.draft_version?.build_status === 'success'
  const canPublish = canTest && !currentModule.is_published

  return (
    <div className="module-editor">
      <div className="editor-toolbar">
        <div className="editor-toolbar-left">
          <button className="btn btn-ghost" onClick={() => navigate('/modules')}>
            <ArrowLeft size={18} />
            Back
          </button>
          <div className="editor-module-info">
            <h2>{currentModule.name}</h2>
            <span className={`badge badge-${currentModule.language}`}>
              {currentModule.language}
            </span>
            {currentModule.is_published && (
              <span className="badge badge-info">Published</span>
            )}
          </div>
        </div>

        {isOwner && (
          <div className="editor-toolbar-right">
            <button
              className="btn btn-secondary"
              onClick={() => { setShowBuildPanel(!showBuildPanel); setShowTestPanel(false) }}
            >
              <Play size={18} />
              Build
            </button>
            {canTest && (
              <button
                className="btn btn-secondary"
                onClick={() => { setShowTestPanel(!showTestPanel); setShowBuildPanel(false) }}
              >
                <FlaskConical size={18} />
                Test
              </button>
            )}
            {canPublish && (
              <button
                className="btn btn-primary"
                onClick={handlePublish}
                disabled={isPublishing}
              >
                <Upload size={18} />
                {isPublishing ? 'Publishing...' : 'Publish'}
              </button>
            )}
          </div>
        )}
      </div>

      <div className="editor-body">
        <div className="editor-sidebar">
          <div className="editor-sidebar-header">
            <span>Files</span>
          </div>
          {fileTree && (
            <FileTree
              tree={fileTree}
              currentFile={currentFile}
              onFileSelect={handleFileSelect}
            />
          )}
        </div>

        <div className="editor-main">
          {currentFile ? (
            <>
              <div className="editor-tab-bar">
                <div className="editor-tab active">
                  <span>{currentFile.split('/').pop()}</span>
                  {isDirty && <span className="dirty-indicator">*</span>}
                </div>
              </div>
              <div className="editor-content">
                {isLoadingFile ? (
                  <div className="editor-loading">
                    <div className="loading-spinner" />
                  </div>
                ) : (
                  <MonacoEditor
                    value={fileContent}
                    language={getLanguageFromPath(currentFile)}
                    onChange={handleContentChange}
                    readOnly={!isOwner}
                  />
                )}
              </div>
            </>
          ) : (
            <div className="editor-placeholder">
              <p>Select a file from the sidebar to start editing</p>
            </div>
          )}
        </div>

        {showBuildPanel && (
          <BuildPanel
            moduleId={currentModule.id}
            onClose={() => setShowBuildPanel(false)}
          />
        )}

        {showTestPanel && (
          <TestPanel
            moduleId={currentModule.id}
            onClose={() => setShowTestPanel(false)}
          />
        )}
      </div>
    </div>
  )
}

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase()
  const langMap: Record<string, string> = {
    rs: 'rust',
    go: 'go',
    py: 'python',
    toml: 'toml',
    json: 'json',
    md: 'markdown',
    mod: 'go',
  }
  return langMap[ext || ''] || 'plaintext'
}
