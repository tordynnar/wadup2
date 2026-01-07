import { ReactNode, useState } from 'react'
import Header from './Header'
import Sidebar from './Sidebar'
import StatusBar from './StatusBar'
import './Layout.css'

interface LayoutProps {
  children: ReactNode
}

export default function Layout({ children }: LayoutProps) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false)

  return (
    <div className="layout">
      <Header
        onToggleSidebar={() => setSidebarCollapsed(!sidebarCollapsed)}
        sidebarCollapsed={sidebarCollapsed}
      />
      <div className="layout-body">
        <Sidebar collapsed={sidebarCollapsed} />
        <main className="layout-main">
          {children}
        </main>
      </div>
      <StatusBar />
    </div>
  )
}
