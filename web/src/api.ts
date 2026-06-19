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
  createSession(input: { title: string; topic: string; context: string; mode?: 'three_seat' | 'single_agent'; model_config?: Record<string, { model: string }>; vote_policy?: { allow_self_vote: boolean; strategy: string; min_score_threshold?: number }; scribe_enabled?: boolean; search_enabled?: boolean }) {
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
  pauseSession(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}/pause`, { method: 'POST' })
  },
  resumeSession(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}/resume`, { method: 'POST' })
  },
  updateContext(id: string, context: string) {
    return request<SessionDetails>(`/api/sessions/${id}/context`, {
      method: 'POST',
      body: JSON.stringify({ context }),
    })
  },
  retrySeat(id: string, seat: string) {
    return request<SessionDetails>(`/api/sessions/${id}/retry-seat/${seat}`, { method: 'POST' })
  },
  retryPhase(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}/retry-phase`, { method: 'POST' })
  },
  manualRevision(id: string) {
    return request<SessionDetails>(`/api/sessions/${id}/manual-revision`, { method: 'POST' })
  },
  phaseTrajectory(id: string) {
    return request<Array<{ id: number; session_id: string; event_type: string; payload: unknown; created_at: string }>>(`/api/sessions/${id}/trajectory`)
  },
  testTopics() {
    return request<Array<{ category: string; topic: string; context: string }>>('/api/test-topics')
  },
  configStatus() {
    return request<ConfigStatus>('/api/config/status')
  },
}
