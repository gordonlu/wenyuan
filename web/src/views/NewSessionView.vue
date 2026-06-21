<template>
  <section class="page new-session-page">
    <header class="page-head">
      <p>新建议题</p>
      <h1>启动一次合议</h1>
    </header>
    <section class="template-bar">
      <span>快速开始：</span>
      <button type="button" v-for="t in templates" :key="t.id" class="template-btn" @click="applyTemplate(t)">
        {{ t.label }}
      </button>
    </section>
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
        <details class="source-area" :open="sourceAreaOpen">
          <summary>
            外部资料
            <span v-if="sourceCount" class="badge flat">{{ sourceCount }} 项</span>
          </summary>
          <div class="source-area-body">
            <DocumentSourcePanel
              v-model="documentContext"
              v-model:evidence="documentEvidence"
              v-model:tool-runs="documentToolRuns"
            />
            <CodeSearchPanel
              v-model="codeContext"
              v-model:evidence="codeEvidence"
              v-model:tool-runs="codeToolRuns"
            />
          </div>
        </details>
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
        <details class="vote-policy-config" :open="scribeOpen">
          <summary>书记官</summary>
          <div class="vote-policy-body">
            <label class="toggle-row">
              <input type="checkbox" v-model="scribeEnabled" />
              <span>启用书记官</span>
            </label>
            <p class="scribe-note">书记官不参与投票，负责整理共识、汇总冲突、生成最终报告</p>
          </div>
        </details>
        <details class="vote-policy-config" :open="true">
          <summary>联网搜索</summary>
          <div class="vote-policy-body">
            <label class="toggle-row">
              <input type="checkbox" v-model="searchEnabled" />
              <span>启用搜索</span>
            </label>
            <p class="scribe-note">在讨论前根据议题内容搜索网络，搜索结果作为证据供各席参考</p>
          </div>
        </details>
        <details class="vote-policy-config" :open="votePolicyOpen">
          <summary>投票策略</summary>
          <div class="vote-policy-body">
            <label>
              <strong>策略</strong>
              <select v-model="voteStrategy">
                <option v-for="s in voteStrategyOptions" :key="s.value" :value="s.value">{{ s.label }}</option>
              </select>
              <span class="vote-policy-hint">{{ voteStrategyHint }}</span>
            </label>
            <label class="toggle-row">
              <input type="checkbox" v-model="allowSelfVote" />
              <span>允许自投</span>
            </label>
          </div>
        </details>
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
import CodeSearchPanel from '../components/CodeSearchPanel.vue'
import DocumentSourcePanel from '../components/DocumentSourcePanel.vue'
import type { EvidenceItem, ToolRun } from '../domain/session'

const router = useRouter()
const title = ref('')
const topic = ref('')
const context = ref('')
const documentContext = ref('')
const documentEvidence = ref<EvidenceItem[]>([])
const documentToolRuns = ref<ToolRun[]>([])
const codeContext = ref('')
const codeEvidence = ref<EvidenceItem[]>([])
const codeToolRuns = ref<ToolRun[]>([])
const mode = ref<'three_seat' | 'single_agent'>('three_seat')
const voteStrategy = ref<'simple_majority' | 'risk_veto' | 'unanimous' | 'conditional_pass' | 'weighted_score'>('simple_majority')
const allowSelfVote = ref(true)
const scribeEnabled = ref(false)
const sourceAreaOpen = ref(true)

const templates = [
  {
    id: 'product',
    label: '产品决策',
    title: '是否优先做企业版还是低代码版本',
    topic: '我们的产品是一个面向开发者的API管理工具，目前月活5万。团队正在争论应该深耕现有用户做企业版，还是横向扩展做一个面向非技术用户的低代码版本。',
    context: '现有用户反馈企业版需求强烈，但新市场可能更大。团队只有10人，资源有限。使用三年数据：API调用量年增40%，免费用户流失率60%。企业用户续费率95%，平均客单价$2000/月。',
    suggest_search: true,
    suggest_scribe: true,
  },
  {
    id: 'code',
    label: '代码方案评审',
    title: '评审新API网关的技术方案',
    topic: '团队提交了新的API网关设计方案，需要从架构合理性、资源成本和长期可维护性三个角度评估。\n\n需要判断：\n1. 该方案是否适合当前每秒2000请求的流量规模。\n2. 与现有Nginx+OpenResty方案相比，迁移成本是否合理。\n3. 方案的扩展性设计是否足够支撑未来12个月的增长预期。',
    context: '方案使用Rust重写核心路由层，引入gRPC取代REST内部通信。预计开发周期3个月。团队有2人熟悉Rust。当前基础设施运行在Kubernetes上。',
    suggest_search: false,
    suggest_scribe: true,
  },
  {
    id: 'fact',
    label: '文档事实核验',
    title: '核验产品白皮书中的数据 claims',
    topic: '产品团队提交了Q2产品白皮书初稿，其中包含多项关于市场占有率、用户增长和性能指标的数据声明。需要逐条核验数据来源、时效性和准确性。',
    context: '白皮书引用了第三方报告、内部Dashboard数据和竞品对比。部分数据无明确来源标注。预计将在下月面向客户发布。',
    suggest_search: true,
    suggest_scribe: false,
  },
]

function applyTemplate(t: typeof templates[number]) {
  title.value = t.title
  topic.value = t.topic
  context.value = t.context
  searchEnabled.value = t.suggest_search
  scribeEnabled.value = t.suggest_scribe
  sourceAreaOpen.value = false
}

const sourceCount = computed(() =>
  documentEvidence.value.length + codeEvidence.value.length
)
const searchEnabled = ref(false)
const votePolicyOpen = ref(true)
const scribeOpen = ref(true)
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
    const [config, preferences] = await Promise.all([
      api.configStatus(),
      api.preferences().catch(() => null),
    ])
    seatModelsMap.value = config.seat_available_models ?? {}
    const globalFallback = config.available_models ?? []
    // Apply per-seat models or global fallback
    for (const s of seatConfigs.value) {
      const key = s.key.toUpperCase()
      s.models = seatModelsMap.value[key]?.length
        ? seatModelsMap.value[key]
        : globalFallback
    }
    if (preferences) {
      mode.value = preferences.defaults.mode
      scribeEnabled.value = preferences.defaults.scribe_enabled
      searchEnabled.value = preferences.defaults.search_enabled
      voteStrategy.value = preferences.defaults.vote_strategy
      allowSelfVote.value = preferences.defaults.allow_self_vote
      for (const s of seatConfigs.value) {
        const preferred = preferences.models[s.key as keyof typeof preferences.models]
        if (preferred) s.model = preferred
      }
    }
  } catch { /* ignore */ }
})

const showModelConfig = computed(() => {
  return seatConfigs.value.some(s => s.models.length)
})

const hasModelConfig = computed(() =>
  seatConfigs.value.some(s => s.model)
)

const voteStrategyOptions = [
  { value: 'simple_majority', label: '普通多数（2/3）' },
  { value: 'risk_veto', label: '风险否决' },
  { value: 'unanimous', label: '全票通过（3/3）' },
  { value: 'conditional_pass', label: '有条件通过' },
  { value: 'weighted_score', label: '加权评分' },
]

const voteStrategyHint = computed(() => {
  const hints: Record<string, string> = {
    simple_majority: '两席以上同意即形成多数',
    risk_veto: '任何一席提出阻塞问题即否决',
    unanimous: '需要三席全部同意',
    conditional_pass: '同普通多数，但增加持续监控条件',
    weighted_score: '按五项评分加权总分决定',
  }
  return hints[voteStrategy.value] ?? ''
})

const activeSeats = computed(() =>
  mode.value === 'single_agent'
    ? seatConfigs.value.slice(0, 1)
    : seatConfigs.value
)

async function submit() {
  loading.value = true
  error.value = ''
  try {
    const model_config: Record<string, { model?: string; reasoning_effort?: string; max_tokens?: number }> = {}
    for (const s of seatConfigs.value) {
      if (s.model) model_config[s.key] = { model: s.model }
    }
    const votePolicy = voteStrategy.value !== 'simple_majority' || !allowSelfVote.value
      ? { strategy: voteStrategy.value, allow_self_vote: allowSelfVote.value }
      : undefined
    const session = await api.createSession({
      title: title.value,
      topic: topic.value,
      context: [context.value.trim(), documentContext.value.trim(), codeContext.value.trim()].filter(Boolean).join('\n\n'),
      mode: mode.value,
      model_config: Object.keys(model_config).length > 0 ? model_config : undefined,
      vote_policy: votePolicy,
      scribe_enabled: scribeEnabled.value || undefined,
      search_enabled: searchEnabled.value || undefined,
      external_evidence: [...documentEvidence.value, ...codeEvidence.value].length ? [...documentEvidence.value, ...codeEvidence.value] : undefined,
      external_tool_runs: [...documentToolRuns.value, ...codeToolRuns.value].length ? [...documentToolRuns.value, ...codeToolRuns.value] : undefined,
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
.vote-policy-config,
.model-config {
  border: 1px solid rgba(141, 219, 209, 0.18);
  border-radius: var(--radius-sm);
  padding: 12px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.06), transparent),
    rgba(255, 255, 255, 0.035);
}
.vote-policy-config summary,
.model-config summary {
  cursor: pointer;
  font-weight: 600;
  user-select: none;
  color: #e8f7f4;
}
.vote-policy-body {
  display: grid;
  gap: 12px;
  margin-top: 8px;
}
.vote-policy-hint {
  display: block;
  margin-top: 4px;
  font-size: 12px;
  color: #8db4b4;
  line-height: 1.4;
}
.toggle-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 8px;
  color: #f8fafc;
}
.toggle-row input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: #8ddbd1;
}
.scribe-note {
  margin: 0;
  font-size: 12px;
  color: #8db4b4;
  line-height: 1.4;
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

/* ── Template bar ── */
.template-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 24px;
  margin-bottom: 12px;
  font-size: 13px;
  color: var(--color-text-muted);
}

.template-btn {
  padding: 5px 14px;
  border: 1px solid var(--color-border);
  border-radius: 20px;
  background: var(--color-surface);
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  cursor: pointer;
  transition: border-color 150ms, background 150ms;
}

.template-btn:hover {
  border-color: var(--color-accent);
  background: var(--color-accent-light);
  color: var(--color-accent-text);
}

/* ── Source area ── */
.source-area {
  margin-top: 4px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  padding: 12px;
  background: var(--color-surface-alt);
}

.source-area summary {
  cursor: pointer;
  font-weight: 600;
  font-size: 14px;
  user-select: none;
  display: flex;
  align-items: center;
  gap: 8px;
}

.source-area-body {
  display: grid;
  gap: 16px;
  margin-top: 12px;
}

.source-area .badge {
  font-size: 11px;
  font-weight: 700;
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
