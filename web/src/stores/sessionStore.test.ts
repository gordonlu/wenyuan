import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useSessionStore } from '../stores/sessionStore'
import { api } from '../api'

vi.mock('../api', () => ({
  api: {
    listSessions: vi.fn(),
    getSession: vi.fn(),
    startSession: vi.fn(),
    pauseSession: vi.fn(),
    resumeSession: vi.fn(),
    cancelSession: vi.fn(),
    deleteSession: vi.fn(),
    retryPhase: vi.fn(),
    manualRevision: vi.fn(),
  },
}))

describe('sessionStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('loads history', async () => {
    const mock = [{ id: '1', title: 'test', phase: 'completed', mode: 'three_seat', created_at: '' }]
    vi.mocked(api.listSessions).mockResolvedValue(mock as any)
    const store = useSessionStore()
    await store.loadHistory()
    expect(store.sessions).toEqual(mock)
    expect(store.loading).toBe(false)
  })

  it('loads a session', async () => {
    const mock = { session: { id: '1', title: 'test' }, artifacts: {} }
    vi.mocked(api.getSession).mockResolvedValue(mock as any)
    const store = useSessionStore()
    await store.loadSession('1')
    expect(store.current).toEqual(mock)
  })

  it('handles load failure gracefully', async () => {
    vi.mocked(api.listSessions).mockRejectedValue(new Error('network error'))
    const store = useSessionStore()
    await store.loadHistory()
    expect(store.error).toBe('network error')
    expect(store.loading).toBe(false)
  })

  it('deletes session and clears current', async () => {
    vi.mocked(api.deleteSession).mockResolvedValue(undefined)
    const store = useSessionStore()
    store.current = { session: { id: '1' } } as any
    await store.deleteSession('1')
    expect(store.current).toBeNull()
  })
})
