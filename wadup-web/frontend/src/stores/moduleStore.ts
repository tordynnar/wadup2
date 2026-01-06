import { create } from 'zustand'
import type { Module, FileTreeNode, Language } from '../types'
import { modulesApi } from '../api/modules'

interface ModuleState {
  // Module list
  modules: Module[]
  totalModules: number
  currentPage: number
  isLoadingList: boolean
  listError: string | null

  // Current module
  currentModule: Module | null
  fileTree: FileTreeNode | null
  isLoadingModule: boolean
  moduleError: string | null

  // Current file
  currentFile: string | null
  fileContent: string
  isLoadingFile: boolean
  isDirty: boolean

  // Actions
  loadModules: (filter?: string, search?: string, page?: number) => Promise<void>
  loadModule: (id: number) => Promise<void>
  createModule: (name: string, language: Language, description?: string) => Promise<Module>
  deleteModule: (id: number) => Promise<void>
  loadFileTree: (moduleId: number) => Promise<void>
  loadFile: (moduleId: number, path: string) => Promise<void>
  saveFile: (moduleId: number, path: string, content: string) => Promise<void>
  setFileContent: (content: string) => void
  createFile: (moduleId: number, path: string, content?: string) => Promise<void>
  createFolder: (moduleId: number, path: string) => Promise<void>
  deleteFile: (moduleId: number, path: string) => Promise<void>
  clearCurrentModule: () => void
}

export const useModuleStore = create<ModuleState>((set, get) => ({
  // Initial state
  modules: [],
  totalModules: 0,
  currentPage: 1,
  isLoadingList: false,
  listError: null,

  currentModule: null,
  fileTree: null,
  isLoadingModule: false,
  moduleError: null,

  currentFile: null,
  fileContent: '',
  isLoadingFile: false,
  isDirty: false,

  // Actions
  loadModules: async (filter = 'all', search = '', page = 1) => {
    set({ isLoadingList: true, listError: null })
    try {
      const response = await modulesApi.list(filter, search, page)
      set({
        modules: response.items,
        totalModules: response.total,
        currentPage: response.page,
        isLoadingList: false,
      })
    } catch (error) {
      set({
        listError: error instanceof Error ? error.message : 'Failed to load modules',
        isLoadingList: false,
      })
    }
  },

  loadModule: async (id: number) => {
    set({ isLoadingModule: true, moduleError: null })
    try {
      const module = await modulesApi.get(id)
      set({ currentModule: module, isLoadingModule: false })
      // Also load the file tree
      await get().loadFileTree(id)
    } catch (error) {
      set({
        moduleError: error instanceof Error ? error.message : 'Failed to load module',
        isLoadingModule: false,
      })
    }
  },

  createModule: async (name: string, language: Language, description?: string) => {
    const module = await modulesApi.create({ name, language, description })
    // Refresh the list
    await get().loadModules()
    return module
  },

  deleteModule: async (id: number) => {
    await modulesApi.delete(id)
    // Refresh the list
    await get().loadModules()
    // Clear current module if it was deleted
    if (get().currentModule?.id === id) {
      set({ currentModule: null, fileTree: null, currentFile: null, fileContent: '' })
    }
  },

  loadFileTree: async (moduleId: number) => {
    try {
      const tree = await modulesApi.listFiles(moduleId)
      set({ fileTree: tree })
    } catch (error) {
      console.error('Failed to load file tree:', error)
    }
  },

  loadFile: async (moduleId: number, path: string) => {
    // Save current file if dirty
    const { currentFile, isDirty } = get()
    if (isDirty && currentFile) {
      await get().saveFile(moduleId, currentFile, get().fileContent)
    }

    set({ isLoadingFile: true, currentFile: path })
    try {
      const file = await modulesApi.getFile(moduleId, path)
      set({ fileContent: file.content, isLoadingFile: false, isDirty: false })
    } catch (error) {
      set({ isLoadingFile: false })
      console.error('Failed to load file:', error)
    }
  },

  saveFile: async (moduleId: number, path: string, content: string) => {
    await modulesApi.saveFile(moduleId, path, content)
    set({ isDirty: false })
    // Reload module to get updated timestamp
    await get().loadModule(moduleId)
  },

  setFileContent: (content: string) => {
    set({ fileContent: content, isDirty: true })
  },

  createFile: async (moduleId: number, path: string, content = '') => {
    await modulesApi.saveFile(moduleId, path, content)
    await get().loadFileTree(moduleId)
  },

  createFolder: async (moduleId: number, path: string) => {
    await modulesApi.createFolder(moduleId, path)
    await get().loadFileTree(moduleId)
  },

  deleteFile: async (moduleId: number, path: string) => {
    await modulesApi.deleteFile(moduleId, path)
    await get().loadFileTree(moduleId)
    // Clear current file if it was deleted
    if (get().currentFile === path) {
      set({ currentFile: null, fileContent: '', isDirty: false })
    }
  },

  clearCurrentModule: () => {
    set({
      currentModule: null,
      fileTree: null,
      currentFile: null,
      fileContent: '',
      isDirty: false,
      moduleError: null,
    })
  },
}))
