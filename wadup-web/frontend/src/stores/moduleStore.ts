import { create } from 'zustand'
import type { Module, FileTreeNode, Language } from '../types'
import { modulesApi } from '../api/modules'

interface OpenTab {
  path: string
  content: string
  isDirty: boolean
}

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

  // Open tabs
  openTabs: OpenTab[]
  currentFile: string | null
  fileContent: string
  isLoadingFile: boolean
  isDirty: boolean

  // Actions
  loadModules: (filter?: string, search?: string, page?: number) => Promise<void>
  loadModule: (id: number) => Promise<void>
  createModule: (name: string, language: Language, description?: string) => Promise<Module>
  deleteModule: (id: number) => Promise<void>
  loadFileTree: (moduleId: number, version?: 'draft' | 'published') => Promise<void>
  loadFile: (moduleId: number, path: string, version?: 'draft' | 'published') => Promise<void>
  saveFile: (moduleId: number, path: string, content: string) => Promise<void>
  setFileContent: (content: string) => void
  createFile: (moduleId: number, path: string, content?: string) => Promise<void>
  createFolder: (moduleId: number, path: string) => Promise<void>
  deleteFile: (moduleId: number, path: string) => Promise<void>
  renameFile: (moduleId: number, oldPath: string, newPath: string) => Promise<void>
  closeTab: (path: string) => void
  switchTab: (path: string) => void
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

  openTabs: [],
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

  loadFileTree: async (moduleId: number, version: 'draft' | 'published' = 'draft') => {
    try {
      const tree = await modulesApi.listFiles(moduleId, version)
      set({ fileTree: tree })
    } catch (error) {
      console.error('Failed to load file tree:', error)
    }
  },

  loadFile: async (moduleId: number, path: string, version: 'draft' | 'published' = 'draft') => {
    // For published version, don't use tabs - just load the file directly (read-only)
    if (version === 'published') {
      set({ isLoadingFile: true, currentFile: path })
      try {
        const file = await modulesApi.getFile(moduleId, path, 'published')
        set({
          fileContent: file.content,
          isLoadingFile: false,
          isDirty: false,
        })
      } catch (error) {
        set({ isLoadingFile: false })
        console.error('Failed to load file:', error)
      }
      return
    }

    // Draft version uses tabs
    // Save current file state to tab if dirty
    const { currentFile, isDirty, fileContent, openTabs } = get()
    if (currentFile && isDirty) {
      const updatedTabs = openTabs.map((tab) =>
        tab.path === currentFile ? { ...tab, content: fileContent, isDirty: true } : tab
      )
      set({ openTabs: updatedTabs })
    }

    // Check if file is already open in a tab
    const existingTab = openTabs.find((tab) => tab.path === path)
    if (existingTab) {
      set({
        currentFile: path,
        fileContent: existingTab.content,
        isDirty: existingTab.isDirty,
        isLoadingFile: false,
      })
      return
    }

    set({ isLoadingFile: true, currentFile: path })
    try {
      const file = await modulesApi.getFile(moduleId, path)
      const newTab: OpenTab = { path, content: file.content, isDirty: false }
      set({
        fileContent: file.content,
        isLoadingFile: false,
        isDirty: false,
        openTabs: [...get().openTabs, newTab],
      })
    } catch (error) {
      set({ isLoadingFile: false })
      console.error('Failed to load file:', error)
    }
  },

  saveFile: async (moduleId: number, path: string, content: string) => {
    await modulesApi.saveFile(moduleId, path, content)
    // Update tab state
    const updatedTabs = get().openTabs.map((tab) =>
      tab.path === path ? { ...tab, content, isDirty: false } : tab
    )
    set({ isDirty: false, openTabs: updatedTabs })
    // Reload module to get updated timestamp
    await get().loadModule(moduleId)
  },

  setFileContent: (content: string) => {
    const { currentFile, openTabs } = get()
    // Update current tab's content
    const updatedTabs = openTabs.map((tab) =>
      tab.path === currentFile ? { ...tab, content, isDirty: true } : tab
    )
    set({ fileContent: content, isDirty: true, openTabs: updatedTabs })
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

  renameFile: async (moduleId: number, oldPath: string, newPath: string) => {
    await modulesApi.renameFile(moduleId, oldPath, newPath)
    await get().loadFileTree(moduleId)
    // Update tabs if file was renamed
    const updatedTabs = get().openTabs.map((tab) =>
      tab.path === oldPath ? { ...tab, path: newPath } : tab
    )
    set({ openTabs: updatedTabs })
    // Update current file if it was renamed
    if (get().currentFile === oldPath) {
      set({ currentFile: newPath })
    }
  },

  closeTab: (path: string) => {
    const { openTabs, currentFile } = get()
    const tabIndex = openTabs.findIndex((tab) => tab.path === path)
    if (tabIndex === -1) return

    const newTabs = openTabs.filter((tab) => tab.path !== path)

    // If closing the current tab, switch to another
    if (currentFile === path) {
      if (newTabs.length === 0) {
        set({ openTabs: [], currentFile: null, fileContent: '', isDirty: false })
      } else {
        // Switch to the previous tab, or the next one if closing the first
        const newIndex = Math.min(tabIndex, newTabs.length - 1)
        const newCurrentTab = newTabs[newIndex]
        set({
          openTabs: newTabs,
          currentFile: newCurrentTab.path,
          fileContent: newCurrentTab.content,
          isDirty: newCurrentTab.isDirty,
        })
      }
    } else {
      set({ openTabs: newTabs })
    }
  },

  switchTab: (path: string) => {
    const { openTabs, currentFile, fileContent, isDirty } = get()
    const tab = openTabs.find((t) => t.path === path)
    if (!tab) return

    // Save current state to current tab before switching
    if (currentFile) {
      const updatedTabs = openTabs.map((t) =>
        t.path === currentFile ? { ...t, content: fileContent, isDirty } : t
      )
      set({ openTabs: updatedTabs })
    }

    set({
      currentFile: path,
      fileContent: tab.content,
      isDirty: tab.isDirty,
    })
  },

  clearCurrentModule: () => {
    set({
      currentModule: null,
      fileTree: null,
      openTabs: [],
      currentFile: null,
      fileContent: '',
      isDirty: false,
      moduleError: null,
    })
  },
}))
