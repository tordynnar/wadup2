import { get, post } from './client'
import type { User } from '../types'

export const authApi = {
  login: (username: string) =>
    post<User>('/api/auth/login', { username }),

  logout: () =>
    post<void>('/api/auth/logout'),

  getMe: () =>
    get<User | null>('/api/auth/me'),
}
