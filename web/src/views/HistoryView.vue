<template>
  <section class="page">
    <header class="page-head">
      <p>历史议题</p>
      <h1>已创建的合议</h1>
    </header>

    <div class="search-bar">
      <input v-model="searchQuery" type="search" placeholder="搜索议题标题…" />
      <select v-model="filterPhase">
        <option value="">全部阶段</option>
        <option v-for="(label, phase) in phaseLabels" :key="phase" :value="phase">{{ label }}</option>
      </select>
      <select v-model="filterMajority">
        <option value="">全部状态</option>
        <option value="yes">形成多数</option>
        <option value="no">未形成多数</option>
      </select>
      <select v-model="filterFailed">
        <option value="">全部结果</option>
        <option value="failed">仅失败/取消</option>
        <option value="completed">仅完成</option>
      </select>
      <button class="icon" title="刷新" @click="store.loadHistory()">
        <RotateCw :size="18" />
      </button>
    </div>

    <ApiErrorState :message="store.error" />
    <p v-if="store.loading" class="muted" style="padding: var(--space-md) 0">加载中…</p>

    <div v-if="filteredSessions.length === 0 && !store.loading" class="panel">
      <p class="muted">暂无匹配的议题</p>
    </div>

    <div v-else class="list">
      <RouterLink
        v-for="session in filteredSessions"
        :key="session.id"
        class="list-row"
        :to="`/sessions/${session.id}`"
      >
        <strong>{{ session.title }}</strong>
        <span class="badge flat">{{ modeLabels[session.mode] }}</span>
        <span :class="['badge', phaseBadge(session.phase)]">
          {{ phaseLabels[session.phase] }}
        </span>
        <span :class="['badge', session.has_majority ? 'ok' : 'warn']">
          {{ session.has_majority ? '形成多数' : '未形成多数' }}
        </span>
        <time>{{ new Date(session.created_at).toLocaleString() }}</time>
      </RouterLink>
    </div>

    <section class="panel" v-if="failedSessions.length > 0">
      <h2>失败与取消记录</h2>
      <p class="muted">以下议题执行未正常完成：</p>
      <div class="list" style="margin-top: 12px">
        <RouterLink
          v-for="session in failedSessions"
          :key="session.id"
          class="list-row"
          :to="`/sessions/${session.id}`"
        >
          <strong>{{ session.title }}</strong>
          <span class="badge warn">{{ phaseLabels[session.phase] }}</span>
          <time>{{ new Date(session.created_at).toLocaleString() }}</time>
        </RouterLink>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RotateCw } from '@lucide/vue'
import ApiErrorState from '../components/ApiErrorState.vue'
import { modeLabels, phaseLabels, type SessionPhase } from '../domain/session'
import { useSessionStore } from '../stores/sessionStore'

const store = useSessionStore()
const searchQuery = ref('')
const filterPhase = ref('')
const filterMajority = ref('')
const filterFailed = ref('')

const failedPhases: SessionPhase[] = ['failed', 'cancelled']

function phaseBadge(phase: SessionPhase) {
  if (phase === 'completed') return 'ok'
  if (phase === 'failed' || phase === 'cancelled') return 'warn'
  return ''
}

const filteredSessions = computed(() => {
  let items = store.sessions
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.trim().toLowerCase()
    items = items.filter((s) => s.title.toLowerCase().includes(q))
  }
  if (filterPhase.value) {
    items = items.filter((s) => s.phase === filterPhase.value)
  }
  if (filterMajority.value === 'yes') {
    items = items.filter((s) => s.has_majority)
  } else if (filterMajority.value === 'no') {
    items = items.filter((s) => !s.has_majority)
  }
  if (filterFailed.value === 'failed') {
    items = items.filter((s) => failedPhases.includes(s.phase))
  } else if (filterFailed.value === 'completed') {
    items = items.filter((s) => s.phase === 'completed')
  }
  return items.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
})

const failedSessions = computed(() => {
  return store.sessions
    .filter((s) => failedPhases.includes(s.phase))
    .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
})

onMounted(() => store.loadHistory())
</script>
