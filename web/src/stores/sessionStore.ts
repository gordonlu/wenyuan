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
      this.error = ''
      this.current = await api.getSession(id)
    },
  },
})
