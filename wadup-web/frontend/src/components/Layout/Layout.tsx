import { ReactNode } from 'react'
import Header from './Header'
import Sidebar from './Sidebar'
import StatusBar from './StatusBar'
import './Layout.css'

interface LayoutProps {
  children: ReactNode
}

export default function Layout({ children }: LayoutProps) {
  return (
    <div className="layout">
      <Header />
      <div className="layout-body">
        <Sidebar />
        <main className="layout-main">
          {children}
        </main>
      </div>
      <StatusBar />
    </div>
  )
}
