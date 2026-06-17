<template>
  <section v-if="details" class="page workspace">
    <header class="page-head row-head">
      <div>
        <p>{{ phaseLabels[details.session.phase] }}</p>
        <h1>{{ details.session.title }}</h1>
      </div>
      <div class="actions">
        <button class="icon" title="重试" @click="retry">
          <RotateCw :size="18" />
        </button>
        <button class="icon danger" title="取消" @click="cancel">
          <Ban :size="18" />
        </button>
      </div>
    </header>

    <ApiErrorState :message="error" />
    <ApiErrorState v-if="details.session.failure_reason" :message="`失败原因：${details.session.failure_reason}`" />
    <p v-if="details.execution.recovery_state === 'retry_required'" class="notice" role="status">
      上次执行未正常完成，请使用重试继续。
    </p>
    <p v-else-if="details.execution.running" class="notice" role="status">
      当前议题正在执行中，租约到期时间：{{ details.execution.lease_expires_at }}
    </p>
    <SeatStatusStrip :phase="details.session.phase" :events="details.events" />

    <section class="panel">
      <h2>议题</h2>
      <p>{{ details.session.topic }}</p>
      <p v-if="details.session.context" class="muted">{{ details.session.context }}</p>
    </section>

    <section class="panel">
      <h2>Idea 池</h2>
      <div class="item-grid">
        <article v-for="idea in details.artifacts.ideas" :key="idea.id" class="item">
          <span>{{ seatLabels[idea.proposed_by] }}</span>
          <h3>{{ idea.title }}</h3>
          <p>{{ idea.summary }}</p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>批议摘要</h2>
      <div class="item-grid">
        <article v-for="critique in details.artifacts.critiques" :key="`${critique.reviewer}-${critique.target_seat}`" class="item">
          <span>{{ seatLabels[critique.reviewer] }} → {{ seatLabels[critique.target_seat] }}</span>
          <p>{{ critique.challenge }}</p>
          <p class="muted">{{ critique.suggested_improvement }}</p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>策案对比</h2>
      <div class="item-grid proposals">
        <article v-for="proposal in details.artifacts.proposals" :key="proposal.id" class="item">
          <span>{{ seatLabels[proposal.proposed_by] }}</span>
          <h3>{{ proposal.title }}</h3>
          <p>{{ proposal.summary }}</p>
          <p class="muted">{{ proposal.implementation_path }}</p>
        </article>
      </div>
    </section>

    <section class="panel">
      <h2>运行统计</h2>
      <div class="stat-grid">
        <article v-for="stat in seatRunStats(details.artifacts.seat_runs)" :key="stat.seat" class="stat">
          <span>{{ seatLabels[stat.seat] }}</span>
          <strong>{{ stat.tokens }} tokens</strong>
          <p>{{ stat.calls }} 次调用 · {{ stat.durationMs }} ms · {{ stat.failed }} 次失败 · {{ stat.repaired }} 次修复</p>
          <p class="muted">{{ stat.promptVersions || '暂无 Prompt 版本' }}</p>
        </article>
      </div>
    </section>

    <DecisionSummary :decision="details.session.result ?? details.artifacts.decision" />

    <section class="panel">
      <h2>事件时间线</h2>
      <ol class="timeline">
        <li v-for="event in details.events" :key="event.id">
          <span>{{ new Date(event.created_at).toLocaleString() }}</span>
          {{ event.event_type }}
        </li>
      </ol>
    </section>
  </section>
  <section v-else class="page">
    <ApiErrorState :message="error || '加载中'" />
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { Ban, RotateCw } from '@lucide/vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'
import DecisionSummary from '../components/DecisionSummary.vue'
import SeatStatusStrip from '../components/SeatStatusStrip.vue'
import { phaseLabels, seatLabels, seatRunStats, type SessionDetails } from '../domain/session'

const route = useRoute()
const id = computed(() => String(route.params.id))
const details = ref<SessionDetails | null>(null)
const error = ref('')
let source: EventSource | null = null
let timer: number | undefined

async function load() {
  try {
    details.value = await api.getSession(id.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载失败'
  }
}

async function retry() {
  details.value = await api.retrySession(id.value)
  timer = window.setTimeout(load, 500)
}

async function cancel() {
  details.value = await api.cancelSession(id.value)
}

onMounted(async () => {
  await load()
  source = new EventSource(`/api/sessions/${id.value}/events`)
  source.onmessage = () => load()
  timer = window.setInterval(load, 1200)
})

onBeforeUnmount(() => {
  source?.close()
  if (timer) window.clearInterval(timer)
})
</script>
