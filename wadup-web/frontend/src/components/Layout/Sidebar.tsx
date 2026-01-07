import { NavLink } from 'react-router-dom'
import { Package, Upload } from 'lucide-react'

interface SidebarProps {
  collapsed: boolean
}

export default function Sidebar({ collapsed }: SidebarProps) {
  return (
    <aside className={`sidebar ${collapsed ? 'collapsed' : ''}`}>
      <nav className="sidebar-nav">
        <NavLink
          to="/modules"
          className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
          title="Modules"
        >
          <Package size={18} />
          {!collapsed && <span>Modules</span>}
        </NavLink>

        <NavLink
          to="/samples"
          className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
          title="Test Samples"
        >
          <Upload size={18} />
          {!collapsed && <span>Test Samples</span>}
        </NavLink>
      </nav>
    </aside>
  )
}
