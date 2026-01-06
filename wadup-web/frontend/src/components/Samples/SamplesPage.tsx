import { useState, useEffect, useCallback } from 'react'
import { Upload, Trash2, File, HardDrive } from 'lucide-react'
import { samplesApi } from '../../api/samples'
import type { Sample } from '../../types'
import './SamplesPage.css'

export default function SamplesPage() {
  const [samples, setSamples] = useState<Sample[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isUploading, setIsUploading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isDragging, setIsDragging] = useState(false)

  useEffect(() => {
    loadSamples()
  }, [])

  const loadSamples = async () => {
    setIsLoading(true)
    try {
      const data = await samplesApi.list()
      setSamples(data)
      setError(null)
    } catch (err) {
      setError('Failed to load samples')
    } finally {
      setIsLoading(false)
    }
  }

  const handleUpload = async (files: FileList | null) => {
    if (!files || files.length === 0) return

    setIsUploading(true)
    setError(null)

    try {
      const uploaded: Sample[] = []
      for (const file of Array.from(files)) {
        const sample = await samplesApi.upload(file)
        uploaded.push(sample)
      }
      setSamples((prev) => [...uploaded, ...prev])
    } catch (err) {
      setError('Failed to upload one or more files')
    } finally {
      setIsUploading(false)
    }
  }

  const handleDelete = async (sampleId: number) => {
    try {
      await samplesApi.delete(sampleId)
      setSamples((prev) => prev.filter((s) => s.id !== sampleId))
    } catch (err) {
      setError('Failed to delete sample')
    }
  }

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
    handleUpload(e.dataTransfer.files)
  }, [])

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }, [])

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
  }, [])

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  const totalSize = samples.reduce((acc, s) => acc + s.file_size, 0)

  return (
    <div className="samples-page">
      <div className="samples-header">
        <div>
          <h2>Test Samples</h2>
          <p className="samples-subtitle">
            Upload files to test your WADUP modules against
          </p>
        </div>
        <label className="btn btn-primary">
          <Upload size={18} />
          {isUploading ? 'Uploading...' : 'Upload Files'}
          <input
            type="file"
            multiple
            onChange={(e) => handleUpload(e.target.files)}
            disabled={isUploading}
            style={{ display: 'none' }}
          />
        </label>
      </div>

      {error && <div className="samples-error">{error}</div>}

      <div
        className={`samples-drop-zone ${isDragging ? 'dragging' : ''}`}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
      >
        {isLoading ? (
          <div className="samples-loading">
            <div className="loading-spinner" />
            <p>Loading samples...</p>
          </div>
        ) : samples.length === 0 ? (
          <div className="samples-empty">
            <HardDrive size={48} strokeWidth={1} />
            <h3>No samples uploaded</h3>
            <p>Drag and drop files here or click Upload to add test samples</p>
          </div>
        ) : (
          <>
            <div className="samples-stats">
              <span>{samples.length} sample{samples.length !== 1 ? 's' : ''}</span>
              <span>{formatSize(totalSize)} total</span>
            </div>
            <div className="samples-list">
              {samples.map((sample) => (
                <div key={sample.id} className="sample-card">
                  <div className="sample-card-icon">
                    <File size={24} />
                  </div>
                  <div className="sample-card-info">
                    <span className="sample-card-name">{sample.filename}</span>
                    <div className="sample-card-meta">
                      <span>{formatSize(sample.file_size)}</span>
                      <span>{formatDate(sample.created_at)}</span>
                    </div>
                  </div>
                  <button
                    className="btn btn-ghost btn-icon"
                    onClick={() => handleDelete(sample.id)}
                    title="Delete sample"
                  >
                    <Trash2 size={18} />
                  </button>
                </div>
              ))}
            </div>
          </>
        )}

        {isDragging && (
          <div className="drop-overlay">
            <Upload size={48} />
            <p>Drop files to upload</p>
          </div>
        )}
      </div>
    </div>
  )
}
