import { get, post, put, del } from './client'
import type { Module, ModuleListResponse, FileTreeNode, FileContent, Language } from '../types'

export interface CreateModuleRequest {
  name: string
  language: Language
  description?: string
}

export const modulesApi = {
  list: (filter = 'all', search = '', page = 1, limit = 20) => {
    const params = new URLSearchParams({
      filter,
      page: String(page),
      limit: String(limit),
    })
    if (search) params.set('search', search)
    return get<ModuleListResponse>(`/api/modules?${params}`)
  },

  get: (id: number) =>
    get<Module>(`/api/modules/${id}`),

  create: (data: CreateModuleRequest) =>
    post<Module>('/api/modules', data),

  delete: (id: number) =>
    del<void>(`/api/modules/${id}`),

  publish: (id: number) =>
    post<Module>(`/api/modules/${id}/publish`),

  // File operations
  listFiles: (moduleId: number, version = 'draft') =>
    get<FileTreeNode>(`/api/modules/${moduleId}/files?version=${version}`),

  getFile: (moduleId: number, path: string, version = 'draft') =>
    get<FileContent>(`/api/modules/${moduleId}/files/${encodeURIComponent(path)}?version=${version}`),

  saveFile: (moduleId: number, path: string, content: string) =>
    put<void>(`/api/modules/${moduleId}/files/${encodeURIComponent(path)}`, content),

  deleteFile: (moduleId: number, path: string) =>
    del<void>(`/api/modules/${moduleId}/files/${encodeURIComponent(path)}`),

  createFolder: (moduleId: number, path: string) =>
    post<void>(`/api/modules/${moduleId}/files/folders/${encodeURIComponent(path)}`),

  // Build operations
  startBuild: (moduleId: number) =>
    post<{ message: string }>(`/api/modules/${moduleId}/build`),

  getBuildStatus: (moduleId: number) =>
    get<{ status: string; built_at: string | null; wasm_path: string | null }>(`/api/modules/${moduleId}/build/status`),
}
