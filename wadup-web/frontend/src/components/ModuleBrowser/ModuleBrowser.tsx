import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Plus, Search, Package } from 'lucide-react'
import { useModuleStore } from '../../stores/moduleStore'
import ModuleCard from './ModuleCard'
import CreateModuleDialog from './CreateModuleDialog'
import './ModuleBrowser.css'

type Filter = 'all' | 'mine' | 'published'

export default function ModuleBrowser() {
  const navigate = useNavigate()
  const { modules, isLoadingList, loadModules } = useModuleStore()
  const [filter, setFilter] = useState<Filter>('all')
  const [search, setSearch] = useState('')
  const [showCreateDialog, setShowCreateDialog] = useState(false)

  useEffect(() => {
    loadModules(filter, search)
  }, [filter, search, loadModules])

  const handleModuleClick = (moduleId: number) => {
    navigate(`/modules/${moduleId}`)
  }

  const handleModuleCreated = (moduleId: number) => {
    setShowCreateDialog(false)
    navigate(`/modules/${moduleId}`)
  }

  return (
    <div className="module-browser">
      <div className="module-browser-header">
        <h2>Modules</h2>
        <button className="btn btn-primary" onClick={() => setShowCreateDialog(true)}>
          <Plus size={18} />
          New Module
        </button>
      </div>

      <div className="module-browser-toolbar">
        <div className="filter-tabs">
          <button
            className={`filter-tab ${filter === 'all' ? 'active' : ''}`}
            onClick={() => setFilter('all')}
          >
            All
          </button>
          <button
            className={`filter-tab ${filter === 'mine' ? 'active' : ''}`}
            onClick={() => setFilter('mine')}
          >
            My Modules
          </button>
          <button
            className={`filter-tab ${filter === 'published' ? 'active' : ''}`}
            onClick={() => setFilter('published')}
          >
            Published
          </button>
        </div>

        <div className="search-box">
          <Search size={16} />
          <input
            type="text"
            placeholder="Search modules..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>

      <div className="module-list">
        {isLoadingList ? (
          <div className="module-list-loading">
            <div className="loading-spinner" />
            <p>Loading modules...</p>
          </div>
        ) : modules.length === 0 ? (
          <div className="module-list-empty">
            <Package size={48} strokeWidth={1} />
            <h3>No modules found</h3>
            <p>
              {filter === 'mine'
                ? "You haven't created any modules yet."
                : 'No modules match your search.'}
            </p>
            <button className="btn btn-primary" onClick={() => setShowCreateDialog(true)}>
              <Plus size={18} />
              Create your first module
            </button>
          </div>
        ) : (
          modules.map((module) => (
            <ModuleCard
              key={module.id}
              module={module}
              onClick={() => handleModuleClick(module.id)}
            />
          ))
        )}
      </div>

      {showCreateDialog && (
        <CreateModuleDialog
          onClose={() => setShowCreateDialog(false)}
          onCreated={handleModuleCreated}
        />
      )}
    </div>
  )
}
