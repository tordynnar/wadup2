import { useState, useEffect, useCallback, useRef } from 'react'

interface SSEState<T> {
  data: T[]
  status: 'idle' | 'connecting' | 'connected' | 'complete' | 'error'
  error: string | null
}

export function useSSE<T>(url: string | null) {
  const [state, setState] = useState<SSEState<T>>({
    data: [],
    status: 'idle',
    error: null,
  })
  const eventSourceRef = useRef<EventSource | null>(null)

  const connect = useCallback(() => {
    if (!url) return

    setState({ data: [], status: 'connecting', error: null })

    const eventSource = new EventSource(url)
    eventSourceRef.current = eventSource

    eventSource.onopen = () => {
      setState(prev => ({ ...prev, status: 'connected' }))
    }

    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as T

        // Check if this is a completion event
        if ((data as { type?: string }).type === 'complete') {
          setState(prev => ({ ...prev, status: 'complete' }))
          eventSource.close()
          return
        }

        setState(prev => ({
          ...prev,
          data: [...prev.data, data],
        }))
      } catch (e) {
        console.error('Failed to parse SSE data:', e)
      }
    }

    eventSource.onerror = () => {
      setState(prev => ({
        ...prev,
        status: 'error',
        error: 'Connection failed',
      }))
      eventSource.close()
    }
  }, [url])

  const disconnect = useCallback(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close()
      eventSourceRef.current = null
    }
    setState({ data: [], status: 'idle', error: null })
  }, [])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close()
      }
    }
  }, [])

  return {
    ...state,
    connect,
    disconnect,
  }
}
