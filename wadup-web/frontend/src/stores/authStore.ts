import { create } from 'zustand'
import type { User } from '../types'
import { authApi } from '../api/auth'

interface AuthState {
  user: User | null
  isLoading: boolean
  error: string | null
  login: (username: string) => Promise<void>
  logout: () => Promise<void>
  checkAuth: () => Promise<void>
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  isLoading: true,
  error: null,

  login: async (username: string) => {
    set({ isLoading: true, error: null })
    try {
      const user = await authApi.login(username)
      set({ user, isLoading: false })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Login failed',
        isLoading: false
      })
      throw error
    }
  },

  logout: async () => {
    try {
      await authApi.logout()
    } finally {
      set({ user: null })
    }
  },

  checkAuth: async () => {
    set({ isLoading: true })
    try {
      const user = await authApi.getMe()
      set({ user, isLoading: false })
    } catch {
      set({ user: null, isLoading: false })
    }
  },
}))
