import Editor from '@monaco-editor/react'

interface MonacoEditorProps {
  value: string
  language: string
  onChange?: (value: string | undefined) => void
  readOnly?: boolean
}

// Catppuccin Macchiato theme colors for Monaco
const catppuccinMacchiato = {
  base: 'vs-dark',
  inherit: true,
  rules: [
    { token: 'comment', foreground: '6e738d', fontStyle: 'italic' },
    { token: 'keyword', foreground: 'c6a0f6' },
    { token: 'string', foreground: 'a6da95' },
    { token: 'number', foreground: 'f5a97f' },
    { token: 'type', foreground: 'eed49f' },
    { token: 'function', foreground: '8aadf4' },
    { token: 'variable', foreground: 'cad3f5' },
    { token: 'operator', foreground: '91d7e3' },
    { token: 'delimiter', foreground: '939ab7' },
  ],
  colors: {
    'editor.background': '#24273a',
    'editor.foreground': '#cad3f5',
    'editor.lineHighlightBackground': '#363a4f',
    'editor.selectionBackground': '#494d64',
    'editor.inactiveSelectionBackground': '#363a4f',
    'editorCursor.foreground': '#f4dbd6',
    'editorWhitespace.foreground': '#494d64',
    'editorLineNumber.foreground': '#6e738d',
    'editorLineNumber.activeForeground': '#cad3f5',
    'editorIndentGuide.background': '#363a4f',
    'editorIndentGuide.activeBackground': '#494d64',
    'editor.wordHighlightBackground': '#494d6480',
    'editor.findMatchBackground': '#f5a97f40',
    'editor.findMatchHighlightBackground': '#f5a97f20',
  },
}

export default function MonacoEditor({
  value,
  language,
  onChange,
  readOnly = false,
}: MonacoEditorProps) {
  const handleEditorWillMount = (monaco: typeof import('monaco-editor')) => {
    // Define the Catppuccin theme
    monaco.editor.defineTheme('catppuccin-macchiato', catppuccinMacchiato as never)
  }

  const handleEditorDidMount = (
    editor: import('monaco-editor').editor.IStandaloneCodeEditor,
    monaco: typeof import('monaco-editor')
  ) => {
    // Set the theme after mount
    monaco.editor.setTheme('catppuccin-macchiato')

    // Configure editor settings
    editor.updateOptions({
      fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, monospace",
      fontSize: 14,
      lineHeight: 22,
      minimap: { enabled: true, scale: 1 },
      scrollBeyondLastLine: false,
      automaticLayout: true,
      tabSize: 4,
      insertSpaces: true,
      wordWrap: 'off',
      renderWhitespace: 'selection',
      cursorBlinking: 'smooth',
      cursorSmoothCaretAnimation: 'on',
      smoothScrolling: true,
      padding: { top: 8, bottom: 8 },
    })
  }

  return (
    <Editor
      height="100%"
      language={language}
      value={value}
      onChange={onChange}
      beforeMount={handleEditorWillMount}
      onMount={handleEditorDidMount}
      options={{
        readOnly,
        domReadOnly: readOnly,
      }}
      theme="catppuccin-macchiato"
    />
  )
}
