import { ref, watch, type Ref } from 'vue'
import type { RouteLocationNormalizedLoaded, Router } from 'vue-router'

export type SessionViewMode = 'workbench' | 'report'

const STORAGE_KEY = 'wenyuan:session-view-mode'

function normalizeViewMode(value: unknown): SessionViewMode | null {
  const raw = Array.isArray(value) ? value[0] : value
  return raw === 'report' || raw === 'workbench' ? raw : null
}

function readStoredViewMode(storage: Storage | null): SessionViewMode | null {
  try {
    return normalizeViewMode(storage?.getItem(STORAGE_KEY))
  } catch {
    return null
  }
}

export function hasStoredViewMode(storage: Storage | null): boolean {
  return readStoredViewMode(storage) !== null
}

export function readInitialViewMode(
  route: RouteLocationNormalizedLoaded,
  storage: Storage | null,
  defaultMode: SessionViewMode = 'workbench',
): SessionViewMode {
  return normalizeViewMode(route.query.view) ?? readStoredViewMode(storage) ?? defaultMode
}

export function useViewMode(options: {
  route: RouteLocationNormalizedLoaded
  router: Router
  defaultMode?: SessionViewMode
  storage?: Storage | null
}): {
  viewMode: Ref<SessionViewMode>
  setViewMode: (mode: SessionViewMode) => void
} {
  const storage = options.storage ?? (typeof window === 'undefined' ? null : window.localStorage)
  const viewMode = ref<SessionViewMode>(readInitialViewMode(options.route, storage, options.defaultMode))

  function setViewMode(mode: SessionViewMode) {
    viewMode.value = mode
  }

  watch(viewMode, (mode) => {
    try {
      storage?.setItem(STORAGE_KEY, mode)
    } catch {
      // localStorage can be unavailable in restricted browser contexts.
    }

    const query = { ...options.route.query }
    if (mode === 'report') {
      query.view = 'report'
    } else {
      delete query.view
    }
    options.router.replace({ query }).catch(() => {})
  })

  watch(
    () => options.route.query.view,
    (value) => {
      const next = normalizeViewMode(value) ?? 'workbench'
      if (next !== viewMode.value) viewMode.value = next
    },
  )

  return { viewMode, setViewMode }
}
