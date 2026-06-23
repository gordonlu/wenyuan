import type { CodeSearchResponse, ConfigStatus, EvidenceItem, ParseDocumentResponse, ProviderSettings, SessionDetails, SessionRecord, SessionSummary, TestProviderResponse, ToolRun, UserPreferences } from './domain/session'

const base = ''

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${base}${url}`, {
    headers: { 'content-type': 'application/json', ...(options?.headers ?? {}) },
    ...options,
  })
  const text = await response.text()
  if (!response.ok) {
    let error = response.statusText
    try { const payload = JSON.parse(text); error = payload.error ?? error } catch { /* ignore */ }
    throw new Error(error)
  }
  if (!text) return undefined as T
  try {
    return JSON.parse(text) as T
  } catch {
    throw new Error(`server returned non-JSON (status ${response.status})`)
  }
}

export const api = {
  createSession(input: { title: string; topic: string; context: string; mode?: 'three_seat' | 'single_agent'; model_config?: Record<string, { model?: string; reasoning_effort?: string; max_tokens?: number }>; vote_policy?: { allow_self_vote: boolean; strategy: string; min_score_threshold?: number }; scribe_enabled?: boolean; search_enabled?: boolean; external_evidence?: EvidenceItem[]; external_tool_runs?: ToolRun[] }) {
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
  deleteSession(id: string) {
    return request<void>(`/api/sessions/${id}`, { method: 'DELETE' })
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
  preferences() {
    return request<UserPreferences>('/api/preferences')
  },
  updatePreferences(input: UserPreferences) {
    return request<UserPreferences>('/api/preferences', {
      method: 'PUT',
      body: JSON.stringify(input),
    })
  },
  parseDocument(input: { filename: string; mime_type?: string; content_base64: string }) {
    return request<ParseDocumentResponse>('/api/tools/documents/parse', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },
  searchCode(input: { query: string }) {
    return request<CodeSearchResponse>('/api/tools/code/search', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },
  getProviderSettings() {
    return request<ProviderSettings>('/api/settings/provider')
  },
  updateProviderSettings(input: { provider: string; base_url: string; model: string; api_key?: string; clear_api_key?: boolean }) {
    return request<ProviderSettings>('/api/settings/provider', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },
  testProvider(input: { provider: string; base_url: string; model: string; api_key?: string; use_saved_key?: boolean }) {
    return request<TestProviderResponse>('/api/settings/test-provider', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },
}
