import { useState } from 'react'
import { useAuthStore } from '../../stores/authStore'
import './LoginForm.css'

export default function LoginForm() {
  const [username, setUsername] = useState('')
  const { login, isLoading, error } = useAuthStore()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!username.trim()) return
    await login(username.trim())
  }

  return (
    <div className="login-container">
      <div className="login-card">
        <div className="login-header">
          <h1>WADUP Web</h1>
          <p>WebAssembly Data Unified Processing</p>
        </div>

        <form onSubmit={handleSubmit} className="login-form">
          <div className="form-group">
            <label htmlFor="username">Username</label>
            <input
              id="username"
              type="text"
              className="input"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Enter your username"
              autoFocus
              disabled={isLoading}
            />
          </div>

          {error && <p className="error-message">{error}</p>}

          <button type="submit" className="btn btn-primary w-full" disabled={isLoading || !username.trim()}>
            {isLoading ? 'Signing in...' : 'Continue'}
          </button>

          <p className="login-note">
            Enter any username to get started. A new account will be created if one doesn't exist.
          </p>
        </form>
      </div>
    </div>
  )
}
