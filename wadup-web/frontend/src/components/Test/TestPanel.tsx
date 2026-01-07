import { useState, useEffect } from 'react'
import { X, FlaskConical, Upload, Trash2, Play, CheckCircle, XCircle, Loader } from 'lucide-react'
import { samplesApi, testApi } from '../../api/samples'
import type { Sample, TestRun } from '../../types'
import TestResultViewer from './TestResultViewer'
import './TestPanel.css'

interface TestPanelProps {
  moduleId: number
  onClose: () => void
}

export default function TestPanel({ moduleId, onClose }: TestPanelProps) {
  const [samples, setSamples] = useState<Sample[]>([])
  const [selectedSamples, setSelectedSamples] = useState<number[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isUploading, setIsUploading] = useState(false)
  const [isTesting, setIsTesting] = useState(false)
  const [testResults, setTestResults] = useState<TestRun[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    loadSamples()
  }, [])

  const loadSamples = async () => {
    setIsLoading(true)
    try {
      const data = await samplesApi.list()
      setSamples(data)
    } catch (err) {
      setError('Failed to load samples')
    } finally {
      setIsLoading(false)
    }
  }

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    setIsUploading(true)
    setError(null)
    try {
      const sample = await samplesApi.upload(file)
      setSamples((prev) => [sample, ...prev])
    } catch (err) {
      setError('Failed to upload sample')
    } finally {
      setIsUploading(false)
      e.target.value = '' // Reset input
    }
  }

  const handleDelete = async (sampleId: number) => {
    try {
      await samplesApi.delete(sampleId)
      setSamples((prev) => prev.filter((s) => s.id !== sampleId))
      setSelectedSamples((prev) => prev.filter((id) => id !== sampleId))
    } catch (err) {
      setError('Failed to delete sample')
    }
  }

  const toggleSample = (sampleId: number) => {
    setSelectedSamples((prev) =>
      prev.includes(sampleId)
        ? prev.filter((id) => id !== sampleId)
        : [...prev, sampleId]
    )
  }

  const handleRunTest = async () => {
    if (selectedSamples.length === 0) return

    setIsTesting(true)
    setTestResults([])
    setError(null)

    try {
      const runs = await testApi.run(moduleId, selectedSamples)
      setTestResults(runs)

      // Poll for results
      const pollResults = async () => {
        const updatedResults = await Promise.all(
          runs.map((run) => testApi.getResult(moduleId, run.id))
        )
        setTestResults(updatedResults)

        const allComplete = updatedResults.every(
          (r) => r.status === 'success' || r.status === 'failed'
        )
        if (!allComplete) {
          setTimeout(pollResults, 1000)
        } else {
          setIsTesting(false)
        }
      }

      setTimeout(pollResults, 1000)
    } catch (err) {
      setError('Failed to run test')
      setIsTesting(false)
    }
  }

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  return (
    <div className="test-panel">
      <div className="test-panel-header">
        <h3>
          <FlaskConical size={18} />
          Test Module
        </h3>
        <button className="btn btn-ghost btn-icon" onClick={onClose}>
          <X size={18} />
        </button>
      </div>

      <div className="test-panel-content">
        <div className="test-section">
          <div className="test-section-header">
            <h4>Test Samples</h4>
            <label className="btn btn-sm btn-secondary">
              <Upload size={14} />
              {isUploading ? 'Uploading...' : 'Upload'}
              <input
                type="file"
                onChange={handleUpload}
                disabled={isUploading}
                style={{ display: 'none' }}
              />
            </label>
          </div>

          {error && <div className="test-error">{error}</div>}

          {isLoading ? (
            <div className="test-loading">
              <Loader size={20} className="animate-spin" />
            </div>
          ) : samples.length === 0 ? (
            <div className="test-empty">
              <p>No samples uploaded yet.</p>
              <p className="text-muted">Upload a file to test your module.</p>
            </div>
          ) : (
            <div className="sample-list">
              {samples.map((sample) => (
                <div
                  key={sample.id}
                  className={`sample-item ${selectedSamples.includes(sample.id) ? 'selected' : ''}`}
                  onClick={() => toggleSample(sample.id)}
                >
                  <input
                    type="checkbox"
                    checked={selectedSamples.includes(sample.id)}
                    onChange={() => toggleSample(sample.id)}
                    onClick={(e) => e.stopPropagation()}
                  />
                  <div className="sample-info">
                    <span className="sample-name">{sample.filename}</span>
                    <span className="sample-size">{formatSize(sample.file_size)}</span>
                  </div>
                  <button
                    className="btn btn-ghost btn-icon btn-sm"
                    onClick={(e) => {
                      e.stopPropagation()
                      handleDelete(sample.id)
                    }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        {testResults.length > 0 && (
          <div className="test-section">
            <h4>Results</h4>
            <div className="test-results">
              {testResults.map((result) => {
                const sample = samples.find((s) => s.id === result.sample_id)
                return (
                  <div key={result.id} className={`test-result ${result.status}`}>
                    <div className="test-result-header">
                      {result.status === 'pending' || result.status === 'running' ? (
                        <Loader size={16} className="animate-spin" />
                      ) : result.status === 'success' ? (
                        <CheckCircle size={16} />
                      ) : (
                        <XCircle size={16} />
                      )}
                      <span>{sample?.filename || `Sample ${result.sample_id}`}</span>
                      <span className="test-result-status">{result.status}</span>
                    </div>
                    {result.status === 'success' && (
                      <TestResultViewer
                        metadata={result.metadata_output}
                        subcontent={result.subcontent_output}
                      />
                    )}
                    {result.status === 'failed' && result.error_message && (
                      <div className="test-result-error">
                        {result.error_message}
                      </div>
                    )}
                    {result.stderr && (
                      <div className="test-result-stderr">
                        <strong>stderr:</strong>
                        <pre>{result.stderr}</pre>
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          </div>
        )}
      </div>

      <div className="test-panel-actions">
        <button
          className="btn btn-primary w-full"
          onClick={handleRunTest}
          disabled={isTesting || selectedSamples.length === 0}
        >
          <Play size={18} />
          {isTesting ? 'Running...' : `Run Test (${selectedSamples.length} selected)`}
        </button>
      </div>
    </div>
  )
}
