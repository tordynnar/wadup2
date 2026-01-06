/**
 * Build API with SSE streaming support
 */

export interface BuildEvent {
  type: 'log' | 'complete'
  content?: string
  status?: string
}

export function streamBuildLogs(
  moduleId: number,
  onEvent: (event: BuildEvent) => void,
  onError: (error: Error) => void
): () => void {
  const eventSource = new EventSource(`/api/modules/${moduleId}/build/stream`)

  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data) as BuildEvent
      onEvent(data)

      if (data.type === 'complete') {
        eventSource.close()
      }
    } catch (e) {
      console.error('Failed to parse build event:', e)
    }
  }

  eventSource.onerror = () => {
    onError(new Error('Build stream connection failed'))
    eventSource.close()
  }

  // Return cleanup function
  return () => eventSource.close()
}
