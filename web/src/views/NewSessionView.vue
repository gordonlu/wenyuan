<template>
  <section class="page new-session-page">
    <header class="page-head">
      <p>新建议题</p>
      <h1>启动一次合议</h1>
    </header>
    <form class="form-panel create-session-panel" @submit.prevent="submit">
      <div class="create-main">
        <label>
          <span class="field-title">标题</span>
          <span class="field-caption">一句话说明这次合议要判断什么。</span>
          <input v-model="title" required :placeholder="titlePlaceholder" />
        </label>
        <label>
          <span class="field-title">议题</span>
          <span class="field-caption">写清楚待决策问题、判断标准和必须权衡的取舍。</span>
          <textarea v-model="topic" required rows="8" :placeholder="topicPlaceholder" />
        </label>
        <label>
          <span class="field-title">背景</span>
          <span class="field-caption">补充现状、约束、已有方案和风险边界；没有也可以留空。</span>
          <textarea v-model="context" rows="6" :placeholder="contextPlaceholder" />
        </label>
      </div>

      <aside class="create-side">
        <div class="create-side-head">
          <span>合议设置</span>
          <strong>{{ mode === 'three_seat' ? '三席' : '单席' }}</strong>
        </div>
        <label>
          合议方式
          <select v-model="mode">
            <option value="three_seat">三席合议：谋远、经世、持正分别判断</option>
            <option value="single_agent">单 Agent：初稿、自评、修订</option>
          </select>
        </label>
        <details v-if="showModelConfig" class="model-config" :open="hasModelConfig">
          <summary>席位模型</summary>
          <div v-for="seat in activeSeats" :key="seat.key" class="seat-config">
            <label>
              <strong>{{ seat.label }}</strong>
              <select v-if="seat.models.length" v-model="seat.model">
                <option value="">使用默认模型</option>
                <option v-for="m in seat.models" :key="m.value" :value="m.value">{{ m.label }}</option>
              </select>
              <input v-else v-model="seat.model" placeholder="模型名称" />
            </label>
          </div>
        </details>
        <ApiErrorState :message="error" />
        <button class="primary create-submit" :disabled="loading">
          <Send :size="18" />
          创建并开议
        </button>
      </aside>
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

const titlePlaceholder = '例如：是否在下一版本上线团队模板库'
const topicPlaceholder = `示例：
我们准备在下一版本加入“团队模板库”，允许成员沉淀常用审议模板并共享给同组用户。

需要判断：
1. 这个功能是否应该进入下一版本，而不是继续打磨现有审议流程。
2. 如果上线，优先做轻量收藏、团队共享，还是带权限和版本管理的完整模板库。
3. 成功标准是提升复用效率、降低新用户上手成本，还是促进团队协作。`
const contextPlaceholder = `示例：
当前情况：
- 已有个人提示词模板需求，但团队共享还没有数据验证。
- 开发资源有限，下一版本最多容纳一个中等复杂度功能。
- 现有用户更关注审议质量和结果导出，模板库可能会分散主流程注意力。

约束与风险：
- 不希望引入复杂权限系统。
- 需要避免页面变成“提示词仓库”，偏离多席审议的核心体验。
- 如果只做收藏，可能价值不够明显；如果做团队共享，维护成本会上升。`

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
.create-session-panel {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  gap: 24px;
  align-items: start;
  padding: 24px;
}
.create-main,
.create-side {
  min-width: 0;
}
.create-main {
  display: grid;
  gap: 8px;
}

.create-main label {
  gap: 7px;
  margin-bottom: 14px;
}

.field-title {
  color: var(--color-text);
  font-size: 15px;
  font-weight: 700;
}

.field-caption {
  margin-top: -2px;
  color: var(--color-text-muted);
  font-size: 12px;
  font-weight: 500;
}

.create-main input,
.create-main textarea {
  border-color: rgba(15, 138, 161, 0.18);
  background-color: #ffffff;
  box-shadow: inset 0 1px 0 rgba(18, 20, 23, 0.02);
}

.create-main textarea {
  line-height: 1.72;
}

.create-main input::placeholder,
.create-main textarea::placeholder {
  color: rgba(56, 72, 80, 0.55);
  line-height: 1.72;
}

.create-main input:hover,
.create-main textarea:hover {
  border-color: rgba(15, 138, 161, 0.34);
}

.create-side {
  position: sticky;
  top: 24px;
  display: grid;
  gap: 16px;
  padding: 18px;
  border: 1px solid rgba(141, 219, 209, 0.22);
  border-radius: var(--radius-md);
  background:
    radial-gradient(circle at 18% 0%, rgba(15, 138, 161, 0.24), transparent 34%),
    linear-gradient(145deg, rgba(255, 255, 255, 0.09), transparent 48%),
    linear-gradient(180deg, #111b24, #0a1118);
  color: #f8fafc;
  box-shadow:
    0 22px 54px rgba(0, 0, 0, 0.24),
    inset 0 1px 0 rgba(255, 255, 255, 0.12);
}
.create-side label {
  color: #f8fafc;
}
.create-side select,
.create-side input {
  border-color: rgba(141, 219, 209, 0.24);
  background-color: rgba(5, 12, 18, 0.78);
  color: #f8fafc;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
}
.create-side select {
  background-image:
    linear-gradient(90deg, transparent, transparent calc(100% - 38px), rgba(141, 219, 209, 0.09) calc(100% - 38px)),
    url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='%238ddbd1' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
}
.create-side select:focus,
.create-side input:focus {
  border-color: #8ddbd1;
  background-color: rgba(5, 12, 18, 0.9);
  box-shadow:
    0 0 0 3px rgba(15, 138, 161, 0.28),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
}
.create-side-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 14px;
  border-bottom: 1px solid rgba(212, 226, 236, 0.14);
}
.create-side-head span {
  color: #b7c5cf;
  font-size: 12px;
  font-weight: 700;
}
.create-side-head strong {
  color: #eafffb;
  font-family: var(--font-display);
  font-size: 24px;
  line-height: 1;
}
.model-config {
  border: 1px solid rgba(141, 219, 209, 0.18);
  border-radius: var(--radius-sm);
  padding: 12px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.06), transparent),
    rgba(255, 255, 255, 0.035);
}
.model-config summary {
  cursor: pointer;
  font-weight: 600;
  user-select: none;
  color: #e8f7f4;
}
.seat-config {
  margin-top: 8px;
  padding: 8px 0;
  display: grid;
  gap: 8px;
}
.seat-config strong {
  min-width: 60px;
  color: #d8fff7;
}
.seat-config select {
  flex: 1;
  padding: 6px 8px;
}
.create-submit {
  width: 100%;
  min-height: 42px;
}

@media (max-width: 860px) {
  .create-session-panel {
    grid-template-columns: 1fr;
  }

  .create-side {
    position: static;
  }
}
</style>
