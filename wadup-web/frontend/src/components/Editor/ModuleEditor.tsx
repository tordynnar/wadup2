import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Play, Upload, FlaskConical, X, Eye, Code } from 'lucide-react'
import { useModuleStore } from '../../stores/moduleStore'
import { useAuthStore } from '../../stores/authStore'
import { modulesApi } from '../../api/modules'
import FileTree from './FileTree'
import MonacoEditor from './MonacoEditor'
import BuildPanel from '../Build/BuildPanel'
import TestPanel from '../Test/TestPanel'
import ResizablePanel from '../common/ResizablePanel'
import RenameDialog from './RenameDialog'
import CreateFileDialog from './CreateFileDialog'
import UnsavedChangesDialog from './UnsavedChangesDialog'
import './ModuleEditor.css'

interface UnsavedDialogState {
  isOpen: boolean
  path: string
  action: 'close' | 'navigate'
}

interface CreateDialogState {
  isOpen: boolean
  type: 'file' | 'folder'
  parentPath: string
}

export default function ModuleEditor() {
  const { moduleId } = useParams<{ moduleId: string }>()
  const navigate = useNavigate()
  const { user } = useAuthStore()
  const {
    currentModule,
    fileTree,
    openTabs,
    currentFile,
    fileContent,
    isLoadingModule,
    isLoadingFile,
    isDirty,
    loadModule,
    loadFileTree,
    loadFile,
    setFileContent,
    saveFile,
    createFile,
    createFolder,
    deleteFile,
    renameFile,
    closeTab,
    switchTab,
    clearCurrentModule,
  } = useModuleStore()
  const [showBuildPanel, setShowBuildPanel] = useState(false)
  const [showTestPanel, setShowTestPanel] = useState(false)
  const [isPublishing, setIsPublishing] = useState(false)
  const [viewingVersion, setViewingVersion] = useState<'draft' | 'published'>('draft')

  // File operation dialogs
  const [renameDialogPath, setRenameDialogPath] = useState<string | null>(null)
  const [createDialog, setCreateDialog] = useState<CreateDialogState>({
    isOpen: false,
    type: 'file',
    parentPath: '',
  })
  const [unsavedDialog, setUnsavedDialog] = useState<UnsavedDialogState>({
    isOpen: false,
    path: '',
    action: 'close',
  })

  useEffect(() => {
    if (moduleId) {
      loadModule(parseInt(moduleId))
    }
    return () => clearCurrentModule()
  }, [moduleId, loadModule, clearCurrentModule])

  // Warn about unsaved changes when leaving the page
  useEffect(() => {
    const hasUnsavedChanges = openTabs.some((tab) => tab.isDirty)
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (hasUnsavedChanges) {
        e.preventDefault()
        e.returnValue = ''
        return ''
      }
    }
    window.addEventListener('beforeunload', handleBeforeUnload)
    return () => window.removeEventListener('beforeunload', handleBeforeUnload)
  }, [openTabs])

  const handleFileSelect = (path: string) => {
    if (moduleId && currentModule) {
      loadFile(parseInt(moduleId), path, viewingVersion)
    }
  }

  // Handle version switching
  const handleVersionSwitch = (version: 'draft' | 'published') => {
    if (version === viewingVersion) return
    setViewingVersion(version)
    // Reload file tree for the new version
    if (moduleId) {
      loadFileTree(parseInt(moduleId), version)
      // Clear current file when switching versions
      if (currentFile) {
        loadFile(parseInt(moduleId), currentFile, version)
      }
    }
  }

  // Load file tree with correct version on mount and version change
  useEffect(() => {
    if (moduleId && currentModule) {
      loadFileTree(parseInt(moduleId), viewingVersion)
    }
  }, [moduleId, currentModule, viewingVersion, loadFileTree])

  // Non-owners viewing published modules should always see the published version
  useEffect(() => {
    if (currentModule && user) {
      const ownsModule = currentModule.author_id === user.id
      if (!ownsModule && currentModule.is_published && viewingVersion === 'draft') {
        setViewingVersion('published')
      }
    }
  }, [currentModule, user, viewingVersion])

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

  // Check if current user owns this module
  const isOwner = user ? currentModule.author_id === user.id : false

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

  // Determine status tags
  const hasUnbuiltChanges = currentModule.draft_version?.built_at
    ? new Date(currentModule.updated_at) > new Date(currentModule.draft_version.built_at)
    : true
  const hasUnpublishedBuild =
    currentModule.draft_version?.build_status === 'success' &&
    (!currentModule.is_published ||
      (currentModule.published_at &&
        currentModule.draft_version.built_at &&
        new Date(currentModule.draft_version.built_at) > new Date(currentModule.published_at)))

  // Can publish if there's a successful build that hasn't been published yet, and no unbuilt changes
  const canPublish = hasUnpublishedBuild && !hasUnbuiltChanges

  // File operation handlers
  const handleRename = (path: string) => {
    setRenameDialogPath(path)
  }

  const handleRenameSubmit = async (newPath: string) => {
    if (!moduleId || !renameDialogPath) return
    try {
      await renameFile(parseInt(moduleId), renameDialogPath, newPath)
    } catch (error) {
      console.error('Rename failed:', error)
    }
  }

  const handleDelete = async (path: string) => {
    if (!moduleId) return
    if (!confirm(`Are you sure you want to delete "${path}"?`)) return
    try {
      await deleteFile(parseInt(moduleId), path)
    } catch (error) {
      console.error('Delete failed:', error)
    }
  }

  const handleCreateFile = (parentPath: string) => {
    setCreateDialog({ isOpen: true, type: 'file', parentPath })
  }

  const handleCreateFolder = (parentPath: string) => {
    setCreateDialog({ isOpen: true, type: 'folder', parentPath })
  }

  const handleCreateSubmit = async (path: string) => {
    if (!moduleId) return
    try {
      if (createDialog.type === 'file') {
        await createFile(parseInt(moduleId), path)
        // Open the newly created file
        loadFile(parseInt(moduleId), path)
      } else {
        await createFolder(parseInt(moduleId), path)
      }
    } catch (error) {
      console.error('Create failed:', error)
    }
  }

  const handleMove = async (sourcePath: string, targetPath: string) => {
    if (!moduleId) return
    try {
      await renameFile(parseInt(moduleId), sourcePath, targetPath)
    } catch (error) {
      console.error('Move failed:', error)
    }
  }

  // Handle tab close with unsaved changes check
  const handleCloseTab = (path: string) => {
    const tab = openTabs.find((t) => t.path === path)
    if (tab?.isDirty) {
      setUnsavedDialog({ isOpen: true, path, action: 'close' })
    } else {
      closeTab(path)
    }
  }

  const handleUnsavedSave = async () => {
    if (!moduleId || !unsavedDialog.path) return
    const tab = openTabs.find((t) => t.path === unsavedDialog.path)
    if (tab) {
      // Use fileContent if saving the current file (it has the latest changes)
      // Otherwise use the tab's stored content
      const contentToSave = tab.path === currentFile ? fileContent : tab.content
      await saveFile(parseInt(moduleId), tab.path, contentToSave)
    }
    if (unsavedDialog.action === 'close') {
      closeTab(unsavedDialog.path)
    }
    setUnsavedDialog({ isOpen: false, path: '', action: 'close' })
  }

  const handleUnsavedDiscard = () => {
    if (unsavedDialog.action === 'close') {
      closeTab(unsavedDialog.path)
    }
    setUnsavedDialog({ isOpen: false, path: '', action: 'close' })
  }

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
            {hasUnbuiltChanges && viewingVersion === 'draft' && (
              <span className="badge badge-warning">Unbuilt Changes</span>
            )}
            {hasUnpublishedBuild && !hasUnbuiltChanges && viewingVersion === 'draft' && (
              <span className="badge badge-accent">Unpublished Build</span>
            )}
          </div>
          {currentModule.is_published && (
            <div className="version-toggle">
              {/* Only show Draft button to the owner */}
              {isOwner && (
                <button
                  className={`version-btn ${viewingVersion === 'draft' ? 'active' : ''}`}
                  onClick={() => handleVersionSwitch('draft')}
                  title="View working draft"
                >
                  <Code size={16} />
                  Draft
                </button>
              )}
              <button
                className={`version-btn ${viewingVersion === 'published' ? 'active' : ''}`}
                onClick={() => handleVersionSwitch('published')}
                title="View published version"
              >
                <Eye size={16} />
                Published
              </button>
            </div>
          )}
        </div>

        {isOwner && viewingVersion === 'draft' && (
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
        <ResizablePanel
          defaultWidth={240}
          minWidth={160}
          maxWidth={400}
          side="left"
          className="editor-sidebar"
        >
          <div className="editor-sidebar-header">
            <span>Files</span>
          </div>
          {fileTree && (
            <FileTree
              tree={fileTree}
              currentFile={currentFile}
              onFileSelect={handleFileSelect}
              onRename={viewingVersion === 'draft' ? handleRename : undefined}
              onDelete={viewingVersion === 'draft' ? handleDelete : undefined}
              onCreateFile={viewingVersion === 'draft' ? handleCreateFile : undefined}
              onCreateFolder={viewingVersion === 'draft' ? handleCreateFolder : undefined}
              onMove={viewingVersion === 'draft' ? handleMove : undefined}
            />
          )}
        </ResizablePanel>

        <div className="editor-main">
          {viewingVersion === 'published' ? (
            // Published version: single file view, no tabs
            currentFile ? (
              <>
                <div className="editor-tab-bar">
                  <div className="editor-tab active">
                    <span>{currentFile.split('/').pop()}</span>
                    <span className="tab-readonly-badge">Read-only</span>
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
                      readOnly={true}
                    />
                  )}
                </div>
              </>
            ) : (
              <div className="editor-placeholder">
                <p>Select a file from the sidebar to view</p>
              </div>
            )
          ) : openTabs.length > 0 ? (
            // Draft version: tabbed editing
            <>
              <div className="editor-tab-bar">
                {openTabs.map((tab) => (
                  <div
                    key={tab.path}
                    className={`editor-tab ${tab.path === currentFile ? 'active' : ''}`}
                    onClick={() => switchTab(tab.path)}
                  >
                    <span>{tab.path.split('/').pop()}</span>
                    {tab.isDirty && <span className="dirty-indicator">*</span>}
                    <button
                      className="tab-close-btn"
                      onClick={(e) => {
                        e.stopPropagation()
                        handleCloseTab(tab.path)
                      }}
                    >
                      <X size={14} />
                    </button>
                  </div>
                ))}
              </div>
              <div className="editor-content">
                {isLoadingFile ? (
                  <div className="editor-loading">
                    <div className="loading-spinner" />
                  </div>
                ) : currentFile ? (
                  <MonacoEditor
                    value={fileContent}
                    language={getLanguageFromPath(currentFile)}
                    onChange={handleContentChange}
                    readOnly={!isOwner}
                  />
                ) : null}
              </div>
            </>
          ) : (
            <div className="editor-placeholder">
              <p>Select a file from the sidebar to start editing</p>
            </div>
          )}
        </div>

        {showBuildPanel && (
          <ResizablePanel
            defaultWidth={400}
            minWidth={300}
            maxWidth={600}
            side="right"
            className="build-panel-container"
          >
            <BuildPanel
              moduleId={currentModule.id}
              onClose={() => setShowBuildPanel(false)}
            />
          </ResizablePanel>
        )}

        {showTestPanel && (
          <ResizablePanel
            defaultWidth={400}
            minWidth={300}
            maxWidth={600}
            side="right"
            className="test-panel-container"
          >
            <TestPanel
              moduleId={currentModule.id}
              onClose={() => setShowTestPanel(false)}
            />
          </ResizablePanel>
        )}
      </div>

      {/* File operation dialogs */}
      <RenameDialog
        isOpen={!!renameDialogPath}
        currentPath={renameDialogPath || ''}
        onClose={() => setRenameDialogPath(null)}
        onRename={handleRenameSubmit}
      />

      <CreateFileDialog
        isOpen={createDialog.isOpen}
        type={createDialog.type}
        parentPath={createDialog.parentPath}
        onClose={() => setCreateDialog({ ...createDialog, isOpen: false })}
        onCreate={handleCreateSubmit}
      />

      <UnsavedChangesDialog
        isOpen={unsavedDialog.isOpen}
        fileName={unsavedDialog.path.split('/').pop() || ''}
        onSave={handleUnsavedSave}
        onDiscard={handleUnsavedDiscard}
        onCancel={() => setUnsavedDialog({ ...unsavedDialog, isOpen: false })}
      />
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
