import { useModuleStore } from '../../stores/moduleStore'

export default function StatusBar() {
  const { currentModule, currentFile, isDirty } = useModuleStore()

  return (
    <footer className="statusbar">
      <div className="statusbar-left">
        {currentModule && (
          <span className="statusbar-item">
            {currentModule.name}
            {currentFile && ` / ${currentFile}`}
            {isDirty && ' *'}
          </span>
        )}
      </div>

      <div className="statusbar-right">
        <span className="statusbar-item">WADUP v0.1.0</span>
      </div>
    </footer>
  )
}
