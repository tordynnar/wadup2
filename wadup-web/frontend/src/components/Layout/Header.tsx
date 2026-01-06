import { useAuthStore } from '../../stores/authStore'
import { LogOut, User } from 'lucide-react'

export default function Header() {
  const { user, logout } = useAuthStore()

  return (
    <header className="header">
      <div className="header-left">
        <h1 className="header-title">WADUP Web</h1>
      </div>

      <div className="header-right">
        {user && (
          <>
            <div className="header-user">
              <User size={16} />
              <span>{user.username}</span>
            </div>
            <button
              className="btn btn-ghost btn-icon"
              onClick={logout}
              title="Logout"
            >
              <LogOut size={18} />
            </button>
          </>
        )}
      </div>
    </header>
  )
}
