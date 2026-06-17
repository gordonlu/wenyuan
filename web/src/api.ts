import type { SessionDetails, SessionSummary, SessionRecord, ConfigStatus } from './domain/session'

const base = ''

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${base}${url}`, {
    headers: { 'content-type': 'application/json', ...(options?.headers ?? {}) },
    ...options,
  })
  if (!response.ok) {
    const payload = await response.json().catch(() => ({ error: response.statusText }))
    throw new Error(payload.error ?? response.statusText)
  }
  return response.json() as Promise<T>
}

export const api = {
  createSession(input: { title: string; topic: string; context: string }) {
    return request<SessionRecord>('/api/sessions', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },
  listSessions() {
    return request<SessionSummary[]>('/api/sessions')
  },
  getSession(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}`)
  },
  startSession(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}/start`, { method: 'POST' })
  },
  retrySession(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}/retry`, { method: 'POST' })
  },
  cancelSession(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}/cancel`, { method: 'POST' })
  },
  configStatus() {
    return request<ConfigStatus>('/api/config/status')
  },
}
