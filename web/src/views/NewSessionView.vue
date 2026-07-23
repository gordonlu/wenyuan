<template>
  <section class="page new-session-page">
    <header class="hero">
      <div class="hero-side" aria-hidden="true">
        <span class="v-text hero-side-title">文渊合议</span>
        <span class="hero-side-rule" />
        <span class="v-text hero-side-flow">独议 · 批议 · 复议 · 阁议</span>
      </div>
      <div class="hero-main">
        <p class="hero-kicker">三席 AI 合议工作台 · WENYUAN COUNCIL</p>
        <h1>把一个难题，<br />交给<span class="hero-accent">三位席官</span>合议</h1>
        <p class="hero-sub">谋远找新路径，经世看落地，持正守边界。三席分阶段独立思考、交叉批议、修订策案、投票决策 —— 给你一份经得起推敲的结论。</p>
        <div class="hero-medals" aria-hidden="true">
          <span class="medal mouyuan"><b>谋</b><i>谋远席 · 找新路径</i></span>
          <span class="medal-link" />
          <span class="medal jingshi"><b>经</b><i>经世席 · 看落地</i></span>
          <span class="medal-link" />
          <span class="medal chizheng"><b>持</b><i>持正席 · 守边界</i></span>
        </div>
      </div>
    </header>
    <div class="compose-head">
      <span class="seal-stamp small">奏</span>
      <div>
        <h2>呈递议题</h2>
        <p>写清待决之事，三席即刻开议</p>
      </div>
    </div>
    <section class="template-bar">
      <span>奏折范式：</span>
      <button type="button" v-for="t in templates" :key="t.id" class="template-btn" @click="applyTemplate(t)">
        {{ t.label }}
      </button>
    </section>
    <form class="form-panel create-session-panel" @submit.prevent="submit">
      <div class="create-main">
        <label>
          <span class="field-title">标题 <span class="required-mark">*</span></span>
          <span class="field-caption">一句话说明这次合议要判断什么。</span>
          <input v-model="title" ref="titleRef" :placeholder="titlePlaceholder" />
          <span v-if="errors.title" class="field-error">{{ errors.title }}</span>
        </label>
        <label>
          <span class="field-title">议题 <span class="required-mark">*</span></span>
          <span class="field-caption">写清楚待决策问题、判断标准和必须权衡的取舍。</span>
          <textarea v-model="topic" ref="topicRef" rows="8" :placeholder="topicPlaceholder" />
          <span v-if="errors.topic" class="field-error">{{ errors.topic }}</span>
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
          <summary>报告详细度</summary>
          <div class="vote-policy-body">
            <label class="toggle-row">
              <input type="checkbox" v-model="scribeEnabled" />
              <span>完整报告（含证据、三席详情等）</span>
            </label>
            <p class="scribe-note">关闭时只展示结论、策案对比与投票摘要；开启后在报告中展开所有过程细节。</p>
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
        <details v-if="mode !== 'single_agent'" class="vote-policy-config" :open="votePolicyOpen">
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
import { useConfirm } from '../composables/useConfirm'
import type { EvidenceItem, ToolRun } from '../domain/session'

const router = useRouter()
const { confirm } = useConfirm()
const title = ref('')
const topic = ref('')
const titleRef = ref<HTMLInputElement>()
const topicRef = ref<HTMLTextAreaElement>()
const errors = ref<{ title?: string; topic?: string }>({})
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
  },
  {
    id: 'code',
    label: '代码方案评审',
    title: '评审新API网关的技术方案',
    topic: '团队提交了新的API网关设计方案，需要从架构合理性、资源成本和长期可维护性三个角度评估。\n\n需要判断：\n1. 该方案是否适合当前每秒2000请求的流量规模。\n2. 与现有Nginx+OpenResty方案相比，迁移成本是否合理。\n3. 方案的扩展性设计是否足够支撑未来12个月的增长预期。',
    context: '方案使用Rust重写核心路由层，引入gRPC取代REST内部通信。预计开发周期3个月。团队有2人熟悉Rust。当前基础设施运行在Kubernetes上。',
    suggest_search: false,
  },
  {
    id: 'fact',
    label: '文档事实核验',
    title: '核验产品白皮书中的数据 claims',
    topic: '产品团队提交了Q2产品白皮书初稿，其中包含多项关于市场占有率、用户增长和性能指标的数据声明。需要逐条核验数据来源、时效性和准确性。',
    context: '白皮书引用了第三方报告、内部Dashboard数据和竞品对比。部分数据无明确来源标注。预计将在下月面向客户发布。',
    suggest_search: true,
  },
]

function applyTemplate(t: typeof templates[number]) {
  title.value = t.title
  topic.value = t.topic
  context.value = t.context
  searchEnabled.value = t.suggest_search
  scribeEnabled.value = false
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
  errors.value = {}
  if (!title.value.trim() || !topic.value.trim()) {
    if (!title.value.trim()) {
      errors.value.title = '请填写标题'
      titleRef.value?.focus()
    }
    if (!topic.value.trim()) {
      errors.value.topic = '请填写议题'
      if (!errors.value.title) topicRef.value?.focus()
    }
    return
  }
  if (!(await confirm('确认创建并开始合议？'))) return
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
/* ── Hero · 开阁 ── */
.hero {
  position: relative;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: clamp(24px, 5vw, 72px);
  margin: -6px 0 8px;
  padding: 44px 0 40px;
  border-bottom: 1px solid var(--color-border-light);
  overflow: hidden;
}

.hero::before {
  content: '';
  position: absolute;
  inset: -40px -60px;
  background:
    radial-gradient(circle at 78% 20%, rgba(47, 191, 212, 0.12), transparent 42%),
    radial-gradient(circle at 30% 96%, rgba(232, 163, 61, 0.07), transparent 44%),
    radial-gradient(circle at 96% 88%, rgba(224, 92, 138, 0.06), transparent 40%);
  pointer-events: none;
}

.hero > * {
  position: relative;
}

.hero-side {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 6px 2px;
}

.hero-side-title {
  color: var(--color-text);
  font-family: var(--font-display);
  font-size: 21px;
  font-weight: 700;
  opacity: 0.9;
}

.hero-side-rule {
  width: 1px;
  align-self: stretch;
  background: linear-gradient(180deg, transparent, rgba(192, 58, 43, 0.65), transparent);
}

.hero-side-flow {
  color: var(--color-text-dim);
  font-size: 12.5px;
}

.hero-kicker {
  margin: 0 0 12px;
  color: var(--color-accent-text);
  opacity: 0.85;
  font-family: var(--font-mono);
  font-size: 11.5px;
  letter-spacing: 3px;
}

.hero h1 {
  margin: 0;
  max-width: 720px;
  font-family: var(--font-display);
  font-size: clamp(32px, 4.6vw, 50px);
  font-weight: 700;
  line-height: 1.28;
  letter-spacing: 1.5px;
  color: var(--color-text);
  text-wrap: balance;
}

.hero-accent {
  background: linear-gradient(100deg, #5fd6e6 10%, #2fbfd4 45%, #e8a33d 100%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.hero-sub {
  margin: 16px 0 0;
  max-width: 620px;
  color: var(--color-text-muted);
  font-size: 14.5px;
  line-height: 1.85;
}

/* ── 三席圆印 ── */
.hero-medals {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-top: 30px;
}

.medal {
  display: grid;
  justify-items: center;
  gap: 9px;
}

.medal b {
  display: grid;
  place-items: center;
  width: 64px;
  height: 64px;
  border-radius: 50%;
  font-family: var(--font-display);
  font-size: 27px;
  font-weight: 700;
  color: #fff;
  background:
    radial-gradient(circle at 32% 26%, rgba(255, 255, 255, 0.32), transparent 46%),
    linear-gradient(150deg, var(--medal-main), var(--medal-deep));
  box-shadow:
    0 0 0 4px color-mix(in srgb, var(--medal-main) 26%, transparent),
    0 10px 26px color-mix(in srgb, var(--medal-main) 42%, transparent),
    inset 0 0 0 1.5px rgba(255, 255, 255, 0.4),
    inset 0 -3px 8px rgba(0, 0, 0, 0.28);
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.35);
  transition: transform 220ms ease, box-shadow 220ms ease;
}

.medal:hover b {
  transform: translateY(-3px) scale(1.04);
  box-shadow:
    0 0 0 5px color-mix(in srgb, var(--medal-main) 34%, transparent),
    0 16px 34px color-mix(in srgb, var(--medal-main) 55%, transparent),
    inset 0 0 0 1.5px rgba(255, 255, 255, 0.5),
    inset 0 -3px 8px rgba(0, 0, 0, 0.28);
}

.medal i {
  font-style: normal;
  color: var(--color-text-muted);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.5px;
  white-space: nowrap;
}

.medal.mouyuan { --medal-main: #19a8c0; --medal-deep: #063f55; }
.medal.jingshi { --medal-main: #d18a12; --medal-deep: #6f3f00; }
.medal.chizheng { --medal-main: #c2336e; --medal-deep: #641536; }

.medal-link {
  width: clamp(18px, 3vw, 44px);
  height: 1px;
  margin-bottom: 26px;
  background: linear-gradient(90deg, rgba(148, 178, 199, 0.05), rgba(148, 178, 199, 0.45), rgba(148, 178, 199, 0.05));
}

/* ── 呈递议题（奏折头） ── */
.compose-head {
  display: flex;
  align-items: center;
  gap: 14px;
  margin: 26px 0 4px;
  padding: 0 4px;
}

.compose-head h2 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 21px;
  font-weight: 700;
  letter-spacing: 2px;
  color: var(--color-text);
}

.compose-head p {
  margin: 2px 0 0;
  color: var(--color-text-dim);
  font-size: 12.5px;
}

.create-session-panel {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  gap: 24px;
  align-items: start;
  padding: 24px;
  border-top: 3px solid transparent;
  border-image: linear-gradient(90deg, #c03a2b, rgba(192, 58, 43, 0.15) 55%, transparent) 1;
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
.required-mark {
  color: var(--color-danger);
  font-weight: 700;
}
.field-error {
  font-size: 13px;
  color: var(--color-danger);
  background: var(--color-danger-light);
  border: 1px solid rgba(232, 132, 122, 0.4);
  border-radius: var(--radius-sm);
  padding: 5px 10px;
  display: inline-block;
  margin-top: 2px;
}
.create-main label:has(.field-error) input,
.create-main label:has(.field-error) textarea {
  border-color: var(--color-danger);
  box-shadow: 0 0 0 2px rgba(232, 132, 122, 0.14);
}

.create-main input,
.create-main textarea {
  border-color: rgba(47, 191, 212, 0.2);
  background-color: rgba(9, 14, 20, 0.7);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
}

.create-main textarea {
  line-height: 1.72;
}

.create-main input::placeholder,
.create-main textarea::placeholder {
  color: rgba(157, 172, 187, 0.48);
  line-height: 1.72;
}

.create-main input:hover,
.create-main textarea:hover {
  border-color: rgba(47, 191, 212, 0.42);
}

.create-side {
  position: sticky;
  top: 24px;
  display: grid;
  gap: 16px;
  padding: 18px;
  border: 1px solid rgba(95, 214, 230, 0.22);
  border-radius: var(--radius-md);
  background:
    radial-gradient(circle at 18% 0%, rgba(47, 191, 212, 0.2), transparent 38%),
    radial-gradient(circle at 100% 100%, rgba(232, 163, 61, 0.07), transparent 42%),
    linear-gradient(145deg, rgba(255, 255, 255, 0.06), transparent 48%),
    linear-gradient(180deg, #101a23, #080e14);
  color: #f4f8fb;
  box-shadow:
    0 24px 56px rgba(0, 0, 0, 0.5),
    0 0 32px rgba(47, 191, 212, 0.06),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
}
.create-side label {
  color: #f4f8fb;
}
.create-side select,
.create-side input {
  border-color: rgba(95, 214, 230, 0.24);
  background-color: rgba(4, 9, 14, 0.8);
  color: #f4f8fb;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
}
.create-side select {
  background-image:
    linear-gradient(90deg, transparent, transparent calc(100% - 38px), rgba(95, 214, 230, 0.1) calc(100% - 38px)),
    url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='18' height='18' viewBox='0 0 24 24' fill='none' stroke='%235fd6e6' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
}
.create-side select:focus,
.create-side input:focus {
  border-color: #5fd6e6;
  background-color: rgba(4, 9, 14, 0.92);
  box-shadow:
    0 0 0 3px rgba(47, 191, 212, 0.26),
    inset 0 1px 0 rgba(255, 255, 255, 0.07);
}
.create-side-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 14px;
  border-bottom: 1px solid rgba(148, 178, 199, 0.14);
}
.create-side-head span {
  color: #9dacbb;
  font-size: 12px;
  font-weight: 700;
}
.create-side-head strong {
  color: #d9f4f8;
  font-family: var(--font-display);
  font-size: 24px;
  line-height: 1;
}
.vote-policy-config,
.model-config {
  border: 1px solid rgba(95, 214, 230, 0.16);
  border-radius: var(--radius-sm);
  padding: 12px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.05), transparent),
    rgba(255, 255, 255, 0.03);
}
.vote-policy-config summary,
.model-config summary {
  cursor: pointer;
  font-weight: 600;
  user-select: none;
  color: #d9f4f8;
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
  color: #7d95a3;
  line-height: 1.4;
}
.toggle-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 8px;
  color: #f4f8fb;
}
.toggle-row input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: #2fbfd4;
}
.scribe-note {
  margin: 0;
  font-size: 12px;
  color: #7d95a3;
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
  color: #a5ecf5;
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
  border: 1px solid var(--color-border-light);
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.03);
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: border-color 150ms, background 150ms, color 150ms, box-shadow 150ms;
}

.template-btn:hover {
  border-color: rgba(47, 191, 212, 0.5);
  background: var(--color-accent-light);
  color: var(--color-accent-text);
  box-shadow: 0 0 16px rgba(47, 191, 212, 0.12);
}

/* ── Source area ── */
.source-area {
  margin-top: 4px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  padding: 12px;
  background: rgba(255, 255, 255, 0.02);
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
