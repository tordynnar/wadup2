/**
 * Base API client with error handling
 */

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message)
    this.name = 'ApiError'
  }
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let message = `HTTP error ${response.status}`
    try {
      const data = await response.json()
      message = data.detail || message
    } catch {
      // Ignore JSON parse errors
    }
    throw new ApiError(response.status, message)
  }

  // Handle empty responses
  const text = await response.text()
  if (!text) {
    return {} as T
  }
  return JSON.parse(text)
}

export async function get<T>(url: string): Promise<T> {
  const response = await fetch(url, {
    credentials: 'include',
  })
  return handleResponse<T>(response)
}

export async function post<T>(url: string, body?: unknown): Promise<T> {
  const response = await fetch(url, {
    method: 'POST',
    credentials: 'include',
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  })
  return handleResponse<T>(response)
}

export async function put<T>(url: string, body: string, contentType = 'text/plain'): Promise<T> {
  const response = await fetch(url, {
    method: 'PUT',
    credentials: 'include',
    headers: { 'Content-Type': contentType },
    body,
  })
  return handleResponse<T>(response)
}

export async function del<T>(url: string): Promise<T> {
  const response = await fetch(url, {
    method: 'DELETE',
    credentials: 'include',
  })
  return handleResponse<T>(response)
}

export async function upload<T>(url: string, file: File): Promise<T> {
  const formData = new FormData()
  formData.append('file', file)

  const response = await fetch(url, {
    method: 'POST',
    credentials: 'include',
    body: formData,
  })
  return handleResponse<T>(response)
}
