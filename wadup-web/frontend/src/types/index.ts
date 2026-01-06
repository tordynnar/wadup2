// User types
export interface User {
  id: number
  username: string
  created_at: string
}

// Module types
export type Language = 'rust' | 'go' | 'python'
export type BuildStatus = 'pending' | 'building' | 'success' | 'failed'
export type VersionType = 'draft' | 'published'

export interface ModuleVersion {
  id: number
  version_type: VersionType
  build_status: BuildStatus
  built_at: string | null
  wasm_path: string | null
  created_at: string
}

export interface Module {
  id: number
  name: string
  description: string | null
  language: Language
  author_id: number
  author_username: string | null
  is_published: boolean
  published_at: string | null
  created_at: string
  updated_at: string
  draft_version: ModuleVersion | null
  published_version: ModuleVersion | null
}

export interface ModuleListResponse {
  items: Module[]
  total: number
  page: number
  limit: number
  pages: number
}

// File types
export interface FileTreeNode {
  name: string
  type: 'file' | 'directory'
  path?: string
  children?: FileTreeNode[]
}

export interface FileContent {
  path: string
  content: string
  language?: string
}

// Sample types
export interface Sample {
  id: number
  filename: string
  file_size: number
  content_type: string | null
  created_at: string
}

// Test types
export type TestStatus = 'pending' | 'running' | 'success' | 'failed'

// Metadata output structure from WADUP modules
export interface MetadataColumn {
  name: string
  data_type: string
}

export interface MetadataTable {
  name: string
  columns: MetadataColumn[]
}

export interface MetadataRow {
  table_name: string
  values: Record<string, unknown>[]
}

export interface MetadataOutput {
  tables: MetadataTable[]
  rows: MetadataRow[]
}

export interface SubcontentItem {
  index: number
  filename: string | null
  data_hex: string
  size: number
  truncated: boolean
  metadata: Record<string, unknown> | null
}

export interface TestRun {
  id: number
  module_version_id: number
  sample_id: number
  status: TestStatus
  stdout: string | null
  stderr: string | null
  metadata_output: MetadataOutput | null
  subcontent_output: SubcontentItem[] | null
  error_message: string | null
  started_at: string | null
  completed_at: string | null
  created_at: string
}

// API types
export interface ApiError {
  detail: string
}
