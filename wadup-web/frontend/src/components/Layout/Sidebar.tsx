import { NavLink } from 'react-router-dom'
import { Package, Upload, Settings } from 'lucide-react'

export default function Sidebar() {
  return (
    <aside className="sidebar">
      <nav className="sidebar-nav">
        <NavLink
          to="/modules"
          className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
        >
          <Package size={18} />
          <span>Modules</span>
        </NavLink>

        <NavLink
          to="/samples"
          className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
        >
          <Upload size={18} />
          <span>Test Samples</span>
        </NavLink>
      </nav>

      <div className="sidebar-footer">
        <button className="sidebar-link" title="Settings">
          <Settings size={18} />
          <span>Settings</span>
        </button>
      </div>
    </aside>
  )
}
