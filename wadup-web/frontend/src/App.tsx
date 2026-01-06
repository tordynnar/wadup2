import { Routes, Route, Navigate } from 'react-router-dom'
import { useAuthStore } from './stores/authStore'
import { useEffect } from 'react'
import Layout from './components/Layout/Layout'
import LoginForm from './components/Auth/LoginForm'
import ModuleBrowser from './components/ModuleBrowser/ModuleBrowser'
import ModuleEditor from './components/Editor/ModuleEditor'
import SamplesPage from './components/Samples/SamplesPage'

function App() {
  const { user, checkAuth, isLoading } = useAuthStore()

  useEffect(() => {
    checkAuth()
  }, [checkAuth])

  if (isLoading) {
    return (
      <div className="loading-screen">
        <div className="loading-spinner" />
        <p>Loading...</p>
      </div>
    )
  }

  if (!user) {
    return <LoginForm />
  }

  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Navigate to="/modules" replace />} />
        <Route path="/modules" element={<ModuleBrowser />} />
        <Route path="/modules/:moduleId" element={<ModuleEditor />} />
        <Route path="/samples" element={<SamplesPage />} />
      </Routes>
    </Layout>
  )
}

export default App
