import { useState } from 'react'
import { ChevronDown, Table, Binary, FileText } from 'lucide-react'
import type { MetadataOutput, SubcontentItem } from '../../types'
import './TestResultViewer.css'

interface TestResultViewerProps {
  metadata: MetadataOutput | null
  subcontent: SubcontentItem[] | null
}

export default function TestResultViewer({ metadata, subcontent }: TestResultViewerProps) {
  const [selectedTable, setSelectedTable] = useState<string | null>(null)
  const [selectedSubcontent, setSelectedSubcontent] = useState<number | null>(null)

  // Get unique table names from metadata
  const tableNames = metadata?.tables?.map((t) => t.name) || []

  // Auto-select first table if none selected
  if (tableNames.length > 0 && selectedTable === null) {
    setSelectedTable(tableNames[0])
  }

  // Auto-select first subcontent if none selected
  if (subcontent && subcontent.length > 0 && selectedSubcontent === null) {
    setSelectedSubcontent(subcontent[0].index)
  }

  const selectedTableDef = metadata?.tables?.find((t) => t.name === selectedTable)
  const selectedTableRows = metadata?.rows?.filter((r) => r.table_name === selectedTable) || []
  const selectedSubcontentItem = subcontent?.find((s) => s.index === selectedSubcontent)

  // Format cell value for display
  const formatCellValue = (value: Record<string, unknown>): string => {
    const entries = Object.entries(value)
    if (entries.length === 0) return ''
    const [, val] = entries[0]
    if (val === null || val === undefined) return ''
    if (typeof val === 'string') return val
    if (typeof val === 'number') return String(val)
    if (typeof val === 'boolean') return val ? 'true' : 'false'
    return JSON.stringify(val)
  }

  // Format hex dump with ASCII
  const formatHexDump = (hexString: string): string => {
    const bytes: number[] = []
    for (let i = 0; i < hexString.length; i += 2) {
      bytes.push(parseInt(hexString.substring(i, i + 2), 16))
    }

    const lines: string[] = []
    const bytesPerLine = 16

    for (let offset = 0; offset < bytes.length; offset += bytesPerLine) {
      const chunk = bytes.slice(offset, offset + bytesPerLine)

      // Offset
      const offsetStr = offset.toString(16).padStart(8, '0')

      // Hex bytes
      const hexParts: string[] = []
      for (let i = 0; i < bytesPerLine; i++) {
        if (i < chunk.length) {
          hexParts.push(chunk[i].toString(16).padStart(2, '0'))
        } else {
          hexParts.push('  ')
        }
        if (i === 7) hexParts.push(' ')
      }
      const hexStr = hexParts.join(' ')

      // ASCII
      const asciiStr = chunk
        .map((b) => (b >= 32 && b <= 126 ? String.fromCharCode(b) : '.'))
        .join('')
        .padEnd(bytesPerLine, ' ')

      lines.push(`${offsetStr}  ${hexStr}  |${asciiStr}|`)
    }

    return lines.join('\n')
  }

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  return (
    <div className="test-result-viewer">
      {/* Tables Section */}
      {metadata && tableNames.length > 0 && (
        <div className="result-section">
          <div className="result-section-header">
            <Table size={14} />
            <span>Tables</span>
            {tableNames.length > 1 && (
              <div className="result-dropdown">
                <select
                  value={selectedTable || ''}
                  onChange={(e) => setSelectedTable(e.target.value)}
                >
                  {tableNames.map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
                </select>
                <ChevronDown size={14} />
              </div>
            )}
            {tableNames.length === 1 && (
              <span className="result-table-name">{tableNames[0]}</span>
            )}
          </div>

          {selectedTableDef && (
            <div className="result-table-container">
              <table className="result-table">
                <thead>
                  <tr>
                    {selectedTableDef.columns.map((col) => (
                      <th key={col.name}>
                        {col.name}
                        <span className="col-type">{col.data_type}</span>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {selectedTableRows.map((row, rowIdx) => (
                    <tr key={rowIdx}>
                      {selectedTableDef.columns.map((col, colIdx) => (
                        <td key={col.name}>
                          {row.values[colIdx]
                            ? formatCellValue(row.values[colIdx])
                            : ''}
                        </td>
                      ))}
                    </tr>
                  ))}
                  {selectedTableRows.length === 0 && (
                    <tr>
                      <td
                        colSpan={selectedTableDef.columns.length}
                        className="no-data"
                      >
                        No rows
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* Subcontent Section */}
      {subcontent && subcontent.length > 0 && (
        <div className="result-section">
          <div className="result-section-header">
            <Binary size={14} />
            <span>Subcontent</span>
            {subcontent.length > 1 && (
              <div className="result-dropdown">
                <select
                  value={selectedSubcontent ?? ''}
                  onChange={(e) => setSelectedSubcontent(Number(e.target.value))}
                >
                  {subcontent.map((item) => (
                    <option key={item.index} value={item.index}>
                      {item.filename || `File ${item.index}`} ({formatSize(item.size)})
                    </option>
                  ))}
                </select>
                <ChevronDown size={14} />
              </div>
            )}
            {subcontent.length === 1 && (
              <span className="result-table-name">
                {subcontent[0].filename || `File ${subcontent[0].index}`}
                <span className="file-size">({formatSize(subcontent[0].size)})</span>
              </span>
            )}
          </div>

          {selectedSubcontentItem && (
            <div className="result-hexdump-container">
              {selectedSubcontentItem.metadata && (
                <div className="subcontent-metadata">
                  <FileText size={12} />
                  <span>
                    {selectedSubcontentItem.filename || 'Unknown filename'}
                  </span>
                </div>
              )}
              <pre className="hexdump">
                {formatHexDump(selectedSubcontentItem.data_hex)}
              </pre>
              {selectedSubcontentItem.truncated && (
                <div className="truncated-notice">
                  Showing first 4KB of {formatSize(selectedSubcontentItem.size)}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* No output message */}
      {(!metadata || tableNames.length === 0) &&
        (!subcontent || subcontent.length === 0) && (
          <div className="no-output">No metadata or subcontent output</div>
        )}
    </div>
  )
}
