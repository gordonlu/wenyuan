<template>
  <section class="page prompt-library">
    <header class="page-head">
      <p>Prompt Library</p>
      <h1>提示词模板库</h1>
    </header>

    <section class="panel library-controls" aria-label="模板筛选">
      <div class="search-bar">
        <input v-model.trim="query" type="search" placeholder="搜索场景、模板名称或正文关键词" />
        <select v-model="selectedCategory" aria-label="类别筛选">
          <option value="">全部类别</option>
          <option v-for="(label, key) in categoryLabels" :key="key" :value="key">{{ label }}</option>
        </select>
        <select v-model="selectedSeat" aria-label="席位筛选">
          <option value="">全部席位</option>
          <option v-for="[seat, label] in seatOptions" :key="seat" :value="seat">{{ label }}</option>
        </select>
      </div>
      <div class="library-summary">
        <span>共 {{ filteredTemplates.length }} / {{ templates.length }} 条模板</span>
        <button v-if="query || selectedCategory || selectedSeat" class="stat-action" @click="resetFilters">
          清空筛选
        </button>
      </div>
    </section>

    <section v-if="filteredTemplates.length" class="template-grid" aria-label="提示词模板">
      <article
        v-for="template in filteredTemplates"
        :key="template.id"
        :class="['template-card', `seat-${template.recommendedSeats[0]}`, { expanded: expandedId === template.id }]"
      >
        <button class="template-main" @click="toggleTemplate(template.id)">
          <span class="template-category">{{ categoryLabels[template.category] }}</span>
          <span class="template-title">{{ template.title }}</span>
          <span class="template-scenario">{{ template.scenario }}</span>
          <span class="template-seats">
            <span v-for="seat in template.recommendedSeats" :key="seat" class="seat-chip">
              {{ seatLabels[seat] }}
            </span>
          </span>
          <ChevronDown :class="['expand-icon', { open: expandedId === template.id }]" :size="18" />
        </button>

        <div v-if="expandedId === template.id" class="template-body">
          <pre>{{ template.content }}</pre>
          <button class="copy-template" @click="copyTemplate(template)">
            <Check v-if="copiedId === template.id" :size="16" />
            <Copy v-else :size="16" />
            {{ copiedId === template.id ? '已复制' : '复制正文' }}
          </button>
        </div>
      </article>
    </section>

    <section v-else class="panel empty-library">
      <h2>没有匹配的模板</h2>
      <p class="muted">换一个关键词，或放宽类别和席位筛选。</p>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { Check, ChevronDown, Copy } from '@lucide/vue'
import { seatLabels, type SeatKind } from '../domain/session'

type TemplateCategory = 'seat_specific' | 'general_review' | 'risk_review' | 'critique'

interface PromptTemplate {
  id: string
  title: string
  scenario: string
  category: TemplateCategory
  recommendedSeats: SeatKind[]
  content: string
}

const categoryLabels: Record<TemplateCategory, string> = {
  seat_specific: '席位专用',
  general_review: '通用审议',
  risk_review: '风险审查',
  critique: '交叉批议',
}

const seatOptions = Object.entries(seatLabels) as Array<[SeatKind, string]>

const templates: PromptTemplate[] = [
  {
    id: 'mouyuan-direction',
    title: '谋远席：打开备选方向',
    scenario: '议题早期需要更多路径，而不是立刻收敛到熟悉方案时使用。',
    category: 'seat_specific',
    recommendedSeats: ['mouyuan'],
    content: `请站在谋远席视角，为当前议题提出 3 个差异明显的方向。

每个方向需要说明：
1. 适合抓住的机会窗口。
2. 与常规做法不同的地方。
3. 最小可验证动作。
4. 可能被经世席或持正席质疑的点。`,
  },
  {
    id: 'jingshi-execution',
    title: '经世席：压实落地路径',
    scenario: '方案看起来可行，但资源、节奏、边界还不清楚时使用。',
    category: 'seat_specific',
    recommendedSeats: ['jingshi'],
    content: `请站在经世席视角，把候选方案拆成可执行计划。

请输出：
1. 首个两周内可以完成的动作。
2. 需要的人、预算、数据或外部条件。
3. 关键依赖和阻塞点。
4. 可以用来判断方案是否继续推进的指标。`,
  },
  {
    id: 'chizheng-risk',
    title: '持正席：识别不可接受代价',
    scenario: '方案即将进入收敛或投票，需要提前排查风险时使用。',
    category: 'risk_review',
    recommendedSeats: ['chizheng'],
    content: `请站在持正席视角，对当前方案做风险审查。

重点检查：
1. 哪些假设缺少证据。
2. 哪些后果一旦发生不可接受。
3. 哪些指标可能被误读或被短期收益掩盖。
4. 需要补充什么证据后才适合进入下一步。`,
  },
  {
    id: 'general-retro',
    title: '审议复盘：找出分歧来源',
    scenario: '多轮讨论之后，需要弄清楚分歧到底来自事实、价值还是执行成本。',
    category: 'general_review',
    recommendedSeats: ['mouyuan', 'jingshi', 'chizheng'],
    content: `请复盘本轮审议，按以下结构整理：

1. 已经形成共识的判断。
2. 仍然存在分歧的判断。
3. 分歧分别来自事实不明、目标不同、风险偏好不同，还是资源约束不同。
4. 下一轮最值得追问的 3 个问题。`,
  },
  {
    id: 'cross-critique',
    title: '交叉批议：互相挑战强弱点',
    scenario: '已经有多个备选方案，需要三席互相指出盲点时使用。',
    category: 'critique',
    recommendedSeats: ['mouyuan', 'jingshi', 'chizheng'],
    content: `请对其他席位的方案做交叉批议。

每条批议包括：
1. 对方方案最强的一点。
2. 最脆弱的一点。
3. 一个具体反例或失败场景。
4. 一个能让方案变得更强的修改建议。`,
  },
  {
    id: 'evidence-check',
    title: '证据核验：把判断落到依据',
    scenario: '审议中出现大量主观判断，需要区分事实、推断和偏好时使用。',
    category: 'risk_review',
    recommendedSeats: ['chizheng', 'jingshi'],
    content: `请把当前结论拆成可核验的证据清单。

请标注：
1. 哪些内容是事实。
2. 哪些内容是推断。
3. 哪些内容只是偏好或经验判断。
4. 哪些关键证据缺失，会直接影响最终选择。`,
  },
]

const query = ref('')
const selectedCategory = ref<TemplateCategory | ''>('')
const selectedSeat = ref<SeatKind | ''>('')
const expandedId = ref<string | null>(templates[0]?.id ?? null)
const copiedId = ref<string | null>(null)

const filteredTemplates = computed(() => {
  const keyword = query.value.toLowerCase()
  return templates.filter((template) => {
    const matchesKeyword =
      !keyword ||
      [template.title, template.scenario, template.content, categoryLabels[template.category]]
        .join('\n')
        .toLowerCase()
        .includes(keyword)
    const matchesCategory = !selectedCategory.value || template.category === selectedCategory.value
    const matchesSeat = !selectedSeat.value || template.recommendedSeats.includes(selectedSeat.value)
    return matchesKeyword && matchesCategory && matchesSeat
  })
})

function toggleTemplate(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

function resetFilters() {
  query.value = ''
  selectedCategory.value = ''
  selectedSeat.value = ''
}

async function copyTemplate(template: PromptTemplate) {
  try {
    await navigator.clipboard.writeText(template.content)
  } catch {
    const textarea = document.createElement('textarea')
    textarea.value = template.content
    textarea.setAttribute('readonly', '')
    textarea.style.position = 'fixed'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)
    textarea.select()
    document.execCommand('copy')
    document.body.removeChild(textarea)
  }
  copiedId.value = template.id
  window.setTimeout(() => {
    if (copiedId.value === template.id) copiedId.value = null
  }, 1600)
}
</script>

<style scoped>
.prompt-library {
  max-width: 1160px;
}

.library-controls {
  margin-bottom: 18px;
}

.library-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--color-text-muted);
  font-size: 13px;
}

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.template-card {
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.86), transparent 56%),
    var(--color-surface);
  box-shadow: var(--shadow-md);
}

.template-card.seat-mouyuan {
  border-top: 4px solid #0f8aa1;
}

.template-card.seat-jingshi {
  border-top: 4px solid #c77a00;
}

.template-card.seat-chizheng {
  border-top: 4px solid #b62662;
}

.template-main {
  position: relative;
  display: grid;
  width: 100%;
  min-height: 178px;
  grid-template-rows: auto auto 1fr auto;
  align-items: start;
  gap: 8px;
  padding: 18px;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--color-text);
  text-align: left;
  box-shadow: none;
}

.template-main:hover {
  background:
    radial-gradient(circle at 20% 0%, rgba(15, 143, 127, 0.1), transparent 32%),
    rgba(255, 255, 255, 0.5);
}

.template-category {
  color: var(--color-text-muted);
  font-size: 12px;
  font-weight: 700;
}

.template-title {
  font-family: var(--font-display);
  font-size: 18px;
  font-weight: 700;
  line-height: 1.35;
}

.template-scenario {
  color: var(--color-text-muted);
  font-size: 13px;
  line-height: 1.6;
}

.template-seats {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.seat-chip {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 0 8px;
  border: 1px solid rgba(15, 143, 127, 0.2);
  border-radius: 999px;
  background: rgba(15, 143, 127, 0.08);
  color: var(--color-accent-text);
  font-size: 12px;
  font-weight: 700;
}

.expand-icon {
  position: absolute;
  right: 16px;
  top: 16px;
  color: var(--color-text-muted);
  transition: transform var(--transition-fast);
}

.expand-icon.open {
  transform: rotate(180deg);
}

.template-body {
  display: grid;
  gap: 12px;
  padding: 0 18px 18px;
}

.template-body pre {
  max-height: 360px;
  overflow: auto;
  margin: 0;
  padding: 14px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  background: #f7f8f4;
  color: var(--color-text);
  font-family: var(--font-body);
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
}

.copy-template {
  justify-self: start;
}

.empty-library {
  text-align: center;
}

@media (max-width: 860px) {
  .template-grid {
    grid-template-columns: 1fr;
  }

  .library-summary {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
