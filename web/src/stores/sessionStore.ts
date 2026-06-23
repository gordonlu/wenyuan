import { defineStore } from 'pinia'
import { api } from '../api'
import type { SessionDetails, SessionSummary } from '../domain/session'

export const useSessionStore = defineStore('sessions', {
  state: () => ({
    sessions: [] as SessionSummary[],
    current: null as SessionDetails | null,
    loading: false,
    error: '',
  }),
  actions: {
    async loadHistory() {
      this.loading = true
      this.error = ''
      try {
        this.sessions = await api.listSessions()
      } catch (err) {
        this.error = err instanceof Error ? err.message : '加载失败'
      } finally {
        this.loading = false
      }
    },
    async loadSession(id: string) {
      this.loading = true
      this.error = ''
      try {
        this.current = await api.getSession(id)
      } catch (err) {
        this.error = err instanceof Error ? err.message : '加载失败'
      } finally {
        this.loading = false
      }
    },
    async refreshSession(id: string) {
      try {
        this.current = await api.getSession(id)
      } catch {
        /* silent refresh — keep stale data on error */
      }
    },
    subscribeToEvents(id: string, onEvent: () => void) {
      const source = new EventSource(`/api/sessions/${id}/events`)
      source.onmessage = onEvent
      source.onerror = () => source.close()
      return () => source.close()
    },
    async startSession(id: string) {
      return api.startSession(id)
    },
    async pauseSession(id: string) {
      return api.pauseSession(id)
    },
    async resumeSession(id: string) {
      return api.resumeSession(id)
    },
    async cancelSession(id: string) {
      return api.cancelSession(id)
    },
    async retryPhase(id: string) {
      return api.retryPhase(id)
    },
    async deleteSession(id: string) {
      await api.deleteSession(id)
      this.current = null
    },
    async manualRevision(id: string) {
      this.current = await api.manualRevision(id)
    },
  },
})
