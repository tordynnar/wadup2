import { useAuthStore } from '../../stores/authStore'
import { LogOut, User, Menu, PanelLeftClose } from 'lucide-react'

interface HeaderProps {
  onToggleSidebar: () => void
  sidebarCollapsed: boolean
}

export default function Header({ onToggleSidebar, sidebarCollapsed }: HeaderProps) {
  const { user, logout } = useAuthStore()

  return (
    <header className="header">
      <div className="header-left">
        <button
          className="btn btn-ghost btn-icon hamburger-btn"
          onClick={onToggleSidebar}
          title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {sidebarCollapsed ? <Menu size={20} /> : <PanelLeftClose size={20} />}
        </button>
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
