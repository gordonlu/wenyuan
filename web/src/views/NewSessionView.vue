<template>
  <section class="page">
    <header class="page-head">
      <p>新建议题</p>
      <h1>三席独议，交相批议</h1>
    </header>
    <form class="form-panel" @submit.prevent="submit">
      <label>
        标题
        <input v-model="title" required placeholder="是否为桌面助手加入多 Agent 讨论能力" />
      </label>
      <label>
        议题正文
        <textarea v-model="topic" required rows="7" placeholder="写下完整议题、目标和需要权衡的问题" />
      </label>
      <label>
        补充背景
        <textarea v-model="context" rows="5" placeholder="可为空" />
      </label>
      <label>
        运行模式
        <select v-model="mode">
          <option value="three_seat">三席合议（谋远、经世、持正）</option>
          <option value="single_agent">单 Agent（初稿→自我批评→修订）</option>
        </select>
      </label>
      <details v-if="showModelConfig" class="model-config" :open="hasModelConfig">
        <summary>模型配置（可选，不填则用全局默认）</summary>
        <div v-for="seat in activeSeats" :key="seat.key" class="seat-config">
          <label>
            <strong>{{ seat.label }}</strong>
            <select v-if="seat.models.length" v-model="seat.model">
              <option value="">使用全局默认</option>
              <option v-for="m in seat.models" :key="m.value" :value="m.value">{{ m.label }}</option>
            </select>
            <input v-else v-model="seat.model" placeholder="模型名称" />
          </label>
        </div>
      </details>
      <ApiErrorState :message="error" />
      <button class="primary" :disabled="loading">
        <Send :size="18" />
        创建并开议
      </button>
    </form>
  </section>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { Send } from '@lucide/vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'

const router = useRouter()
const title = ref('')
const topic = ref('')
const context = ref('')
const mode = ref<'three_seat' | 'single_agent'>('three_seat')
const loading = ref(false)
const error = ref('')
const seatModelsMap = ref<Record<string, Array<{ value: string; label: string }>>>({})

const seatConfigs = ref<Array<{ key: string; label: string; model: string; models: Array<{ value: string; label: string }> }>>([
  { key: 'mouyuan', label: '谋远席', model: '', models: [] },
  { key: 'jingshi', label: '经世席', model: '', models: [] },
  { key: 'chizheng', label: '持正席', model: '', models: [] },
])

onMounted(async () => {
  try {
    const config = await api.configStatus()
    seatModelsMap.value = config.seat_available_models ?? {}
    const globalFallback = config.available_models ?? []
    // Apply per-seat models or global fallback
    for (const s of seatConfigs.value) {
      const key = s.key.toUpperCase()
      s.models = seatModelsMap.value[key]?.length
        ? seatModelsMap.value[key]
        : globalFallback
    }
  } catch { /* ignore */ }
})

const showModelConfig = computed(() => {
  return seatConfigs.value.some(s => s.models.length)
})

const hasModelConfig = computed(() =>
  seatConfigs.value.some(s => s.model)
)

const activeSeats = computed(() =>
  mode.value === 'single_agent'
    ? seatConfigs.value.slice(0, 1)
    : seatConfigs.value
)

async function submit() {
  loading.value = true
  error.value = ''
  try {
    const model_config: Record<string, { model: string }> = {}
    for (const s of seatConfigs.value) {
      if (s.model) model_config[s.key] = { model: s.model }
    }
    const session = await api.createSession({
      title: title.value,
      topic: topic.value,
      context: context.value,
      mode: mode.value,
      model_config: Object.keys(model_config).length > 0 ? model_config : undefined,
    })
    await api.startSession(session.id)
    router.push(`/sessions/${session.id}`)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '创建失败'
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.model-config {
  margin-top: 16px;
  border: 1px solid #ddd;
  border-radius: 6px;
  padding: 12px;
}
.model-config summary {
  cursor: pointer;
  font-weight: 600;
  user-select: none;
}
.seat-config {
  margin-top: 8px;
  padding: 8px 0;
  display: flex;
  align-items: center;
  gap: 12px;
}
.seat-config strong {
  min-width: 60px;
}
.seat-config select {
  flex: 1;
  padding: 6px 8px;
}
</style>
