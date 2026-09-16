<template>
  <section class="page meeting-page">
    <header class="page-head meeting-head">
      <div>
        <p>External Agent Meeting</p>
        <h1>外部 Agent 会议室</h1>
      </div>
      <button class="secondary-btn" type="button" @click="goCreate">
        <Plus :size="16" />
        新建会议
      </button>
    </header>

    <div class="meeting-shell">
      <aside class="meeting-list-panel">
        <div class="list-head">
          <div>
            <strong>会议</strong>
            <span>{{ meetings.length }} 场</span>
          </div>
          <button class="icon-btn" type="button" title="刷新" @click="refreshAll(false)">
            <RotateCw :size="16" :class="{ spinning: refreshing }" />
          </button>
        </div>

        <button
          v-for="meeting in meetings"
          :key="meeting.meeting_id"
          type="button"
          :class="['meeting-row', { active: meeting.meeting_id === currentId }]"
          @click="selectMeeting(meeting.meeting_id)"
        >
          <span class="meeting-row-title">{{ meeting.title }}</span>
          <span class="meeting-row-meta">
            <span :class="['state-dot', stageTone(meeting.stage)]" />
            {{ stageLabels[meeting.stage] }}
            · {{ meeting.participant_count }}/{{ meeting.participant_limit }} Agent
          </span>
        </button>

        <div v-if="!meetings.length && !loadingMeetings" class="empty-list">
          暂无外部 Agent 会议
        </div>
      </aside>

      <main class="meeting-workspace">
        <div v-if="error" class="error-banner">
          <AlertTriangle :size="17" />
          <span>{{ error }}</span>
        </div>

        <section v-if="!currentId" class="panel create-panel">
          <div class="create-copy">
            <h2>让 2～3 个 Agent 围绕同一个目标开会</h2>
            <p>
              所有 Agent 都独立寻找自己认为最好的实现，再只讨论真正的分歧、遗漏和更优方案；不会被强制分配不同立场。
            </p>
          </div>

          <form class="meeting-form" @submit.prevent="createMeeting">
            <label>
              会议标题
              <input v-model.trim="form.title" maxlength="200" placeholder="例如：Reef Expert v0.2 架构方案" required />
            </label>
            <label>
              用户目标
              <textarea
                v-model.trim="form.goal"
                rows="5"
                maxlength="12000"
                placeholder="描述你真正想解决的问题、成功标准和约束。所有 Agent 会收到同一个目标。"
                required
              />
            </label>
            <label>
              补充上下文 <span class="optional">可选</span>
              <textarea v-model.trim="form.context" rows="4" maxlength="30000" placeholder="已有方案、代码背景、资源限制等" />
            </label>

            <div class="participant-choice">
              <span>参与 Agent 数量</span>
              <div class="segmented">
                <button type="button" :class="{ selected: form.participant_limit === 2 }" @click="form.participant_limit = 2">
                  2 个
                </button>
                <button type="button" :class="{ selected: form.participant_limit === 3 }" @click="form.participant_limit = 3">
                  3 个
                </button>
              </div>
              <small>3 个 Agent 掉线 1 个后可降级为 2 个；2 个 Agent 不会静默降级成单 Agent。</small>
            </div>

            <button class="primary-btn create-action" type="submit" :disabled="creating">
              <UsersRound :size="17" />
              {{ creating ? '正在创建…' : '创建会议室' }}
            </button>
          </form>
        </section>

        <template v-else-if="detail">
          <section class="room-header">
            <div>
              <div class="room-title-line">
                <h2>{{ detail.title }}</h2>
                <span :class="['stage-badge', stageTone(detail.stage)]">{{ stageLabels[detail.stage] }}</span>
              </div>
              <p>{{ detail.goal }}</p>
            </div>
            <span class="updated-at">更新 {{ formatTime(detail.updated_at) }}</span>
          </section>

          <section v-if="detail.paused_reason" class="notice warning-notice">
            <AlertTriangle :size="18" />
            <div>
              <strong>会议已暂停</strong>
              <p>{{ detail.paused_reason }}</p>
            </div>
          </section>

          <section v-if="detail.owner_access" class="invite-panel">
            <div class="invite-copy">
              <span class="section-label">Agent 邀请</span>
              <strong>{{ detail.participants.length }}/{{ detail.participant_limit }} 已加入</strong>
              <p>Agent 只需通过 MCP 加入此会议。会议开始后系统会自动分配独立回答、交叉审阅和综合任务。</p>
            </div>
            <div class="invite-data">
              <div>
                <span>MCP</span>
                <code>{{ detail.mcp_endpoint || 'MCP 未启动' }}</code>
              </div>
              <div>
                <span>Meeting ID</span>
                <code>{{ detail.meeting_id }}</code>
              </div>
              <div>
                <span>Join Code</span>
                <code class="join-code">{{ detail.join_code }}</code>
              </div>
            </div>
            <div class="invite-actions">
              <button class="secondary-btn" type="button" :disabled="!detail.join_code" @click="copyInvite">
                <Copy :size="15" />
                {{ copied ? '已复制' : '复制邀请信息' }}
              </button>
              <button v-if="canStartEarly" class="primary-btn" type="button" :disabled="starting" @click="startEarly">
                <Play :size="15" />
                {{ starting ? '启动中…' : '以 2 个 Agent 提前开始' }}
              </button>
            </div>
          </section>

          <section v-else class="notice readonly-notice">
            <Info :size="18" />
            <div>
              <strong>只读会议状态</strong>
              <p>此浏览器没有该会议的主办方凭证，因此不会显示 Join Code、过程全文或用户回答历史。</p>
            </div>
          </section>

          <section class="status-grid">
            <div class="status-section participants-section">
              <div class="section-title-row">
                <h3>参与 Agent</h3>
                <span>{{ detail.progress.participants_online }} 在线</span>
              </div>
              <div class="participants-grid">
                <article v-for="participant in detail.participants" :key="participant.id" class="participant-card">
                  <span :class="['presence', participant.state]" />
                  <div>
                    <strong>{{ participant.display_name }}</strong>
                    <small>{{ participant.client_name || 'MCP client' }}</small>
                  </div>
                  <span class="participant-state">{{ participantStateLabels[participant.state] }}</span>
                </article>
                <article v-for="slot in emptySlots" :key="`slot-${slot}`" class="participant-card empty-slot">
                  <span class="presence empty" />
                  <div>
                    <strong>等待 Agent</strong>
                    <small>使用 Join Code 加入</small>
                  </div>
                </article>
              </div>
            </div>

            <div class="status-section progress-section">
              <div class="section-title-row">
                <h3>会议进度</h3>
                <span>{{ detail.stage === 'completed' ? '已完成' : '自动推进' }}</span>
              </div>
              <div class="stage-track">
                <div v-for="stage in stageOrder" :key="stage.key" :class="['stage-step', { current: detail.stage === stage.key, done: isStageDone(stage.key) }]">
                  <span class="stage-index">
                    <CheckCircle2 v-if="isStageDone(stage.key)" :size="17" />
                    <Clock3 v-else :size="16" />
                  </span>
                  <div>
                    <strong>{{ stage.label }}</strong>
                    <small>{{ stageProgressText(stage.key) }}</small>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section v-if="detail.active_question" class="question-panel">
            <div class="question-head">
              <div>
                <span class="section-label">Agent 需要一个关键信息</span>
                <h3>{{ detail.active_question.question }}</h3>
              </div>
              <span>由 {{ detail.active_question.asked_by_count }} 个 Agent 请求</span>
            </div>
            <p>{{ detail.active_question.reason }}</p>
            <form v-if="detail.owner_access" class="answer-row" @submit.prevent="answerQuestion">
              <textarea v-model.trim="answerText" rows="3" placeholder="回答一次，系统会共享给所有 Agent，并避免重复发问。" required />
              <button class="primary-btn" type="submit" :disabled="answering || !answerText.trim()">
                {{ answering ? '发送中…' : '回答并继续会议' }}
              </button>
            </form>
          </section>

          <section v-if="detail.final" class="final-panel">
            <div class="section-title-row">
              <div>
                <span class="section-label">Final Synthesis</span>
                <h3>最终综合方案</h3>
              </div>
              <span v-if="detail.final.degraded" class="degraded-note">本轮曾发生 Agent 掉线降级</span>
            </div>
            <pre>{{ detail.final.response }}</pre>
          </section>

          <section v-if="detail.owner_access" class="transcript-panel">
            <div class="section-title-row transcript-title">
              <div>
                <span class="section-label">Meeting Transcript</span>
                <h3>过程记录</h3>
              </div>
              <span>只展示可审阅结论，不展示隐藏推理</span>
            </div>

            <details v-for="stage in stageOrder" :key="stage.key" class="stage-transcript" :open="stage.key !== 'synthesis' || detail.stage === 'completed'">
              <summary>
                <span>{{ stage.label }}</span>
                <span>{{ turnsFor(stage.key).length }} 条任务</span>
              </summary>
              <div v-if="turnsFor(stage.key).length" class="turn-list">
                <article v-for="turn in turnsFor(stage.key)" :key="turn.id" class="turn-card">
                  <div class="turn-head">
                    <strong>{{ turn.assignee_name }}</strong>
                    <span :class="['turn-status', turn.status]">{{ turnStatusLabels[turn.status] }}</span>
                  </div>
                  <pre v-if="turn.response">{{ turn.response }}</pre>
                  <p v-else-if="turn.status === 'skipped'" class="muted">已跳过：{{ turn.skip_reason || 'Agent 掉线' }}</p>
                  <p v-else class="muted">等待 Agent 提交…</p>
                </article>
              </div>
              <p v-else class="muted stage-empty">尚未进入此阶段</p>
            </details>
          </section>
        </template>

        <section v-else class="panel loading-panel">
          <p class="muted">正在读取会议状态…</p>
        </section>
      </main>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { AlertTriangle, CheckCircle2, Clock3, Copy, Info, Play, Plus, RotateCw, UsersRound } from '@lucide/vue'
import { api } from '../api'
import type { CreateMeetingInput, MeetingDetail, MeetingStage, MeetingSummary, TurnStage, TurnStatus } from '../domain/meeting'

interface OwnerSecret {
  owner_token: string
  join_code: string
}

const OWNER_STORAGE_KEY = 'wenyuan.meetingOwners.v1'
const route = useRoute()
const router = useRouter()

const meetings = ref<MeetingSummary[]>([])
const detail = ref<MeetingDetail | null>(null)
const loadingMeetings = ref(false)
const refreshing = ref(false)
const creating = ref(false)
const starting = ref(false)
const answering = ref(false)
const copied = ref(false)
const error = ref('')
const answerText = ref('')
let pollTimer: number | undefined

const form = reactive<CreateMeetingInput>({
  title: '',
  goal: '',
  context: '',
  participant_limit: 3,
})

const ownerSecrets = ref<Record<string, OwnerSecret>>(readOwnerSecrets())

const stageLabels: Record<MeetingStage, string> = {
  lobby: '等待加入',
  initial: '独立回答',
  cross_review: '交叉审阅',
  synthesis: '综合方案',
  completed: '已完成',
  paused: '已暂停',
}

const participantStateLabels = {
  online: '在线',
  stale: '连接异常',
  left: '已离开',
} as const

const turnStatusLabels: Record<TurnStatus, string> = {
  pending: '等待',
  claimed: '处理中',
  submitted: '已提交',
  skipped: '已跳过',
}

const stageOrder: Array<{ key: TurnStage; label: string }> = [
  { key: 'initial', label: '独立回答' },
  { key: 'cross_review', label: '交叉审阅' },
  { key: 'synthesis', label: '综合方案' },
]

const currentId = computed(() => {
  const value = route.params.id
  return typeof value === 'string' ? value : ''
})

const ownerToken = computed(() => currentId.value ? ownerSecrets.value[currentId.value]?.owner_token ?? '' : '')
const emptySlots = computed(() => detail.value ? Math.max(0, detail.value.participant_limit - detail.value.participants.length) : 0)
const canStartEarly = computed(() => {
  const room = detail.value
  return !!room
    && room.owner_access
    && room.stage === 'lobby'
    && room.participant_limit === 3
    && room.participants.length >= 2
})

function readOwnerSecrets(): Record<string, OwnerSecret> {
  if (typeof window === 'undefined') return {}
  try {
    const raw = window.localStorage.getItem(OWNER_STORAGE_KEY)
    return raw ? JSON.parse(raw) as Record<string, OwnerSecret> : {}
  } catch {
    return {}
  }
}

function saveOwnerSecret(meetingId: string, secret: OwnerSecret) {
  ownerSecrets.value = { ...ownerSecrets.value, [meetingId]: secret }
  window.localStorage.setItem(OWNER_STORAGE_KEY, JSON.stringify(ownerSecrets.value))
}

function stageTone(stage: MeetingStage) {
  if (stage === 'completed') return 'ok'
  if (stage === 'paused') return 'warn'
  if (stage === 'lobby') return 'idle'
  return 'active'
}

function formatTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function turnsFor(stage: TurnStage) {
  return detail.value?.transcript.filter((turn) => turn.stage === stage) ?? []
}

function isStageDone(stage: TurnStage) {
  const progress = detail.value?.progress.turns[stage]
  return !!progress && progress.total > 0 && progress.submitted + progress.skipped >= progress.total
}

function stageProgressText(stage: TurnStage) {
  const progress = detail.value?.progress.turns[stage]
  if (!progress || progress.total === 0) return '尚未开始'
  const skipped = progress.skipped ? ` · ${progress.skipped} 跳过` : ''
  return `${progress.submitted}/${progress.total} 已提交${skipped}`
}

function selectMeeting(id: string) {
  router.push(`/meetings/${id}`)
}

function goCreate() {
  detail.value = null
  error.value = ''
  router.push('/meetings')
}

async function loadMeetings(silent = false) {
  if (!silent) loadingMeetings.value = true
  try {
    meetings.value = await api.listMeetings()
  } catch (err) {
    if (!silent) error.value = err instanceof Error ? err.message : String(err)
  } finally {
    if (!silent) loadingMeetings.value = false
  }
}

async function loadDetail(silent = false) {
  if (!currentId.value) {
    detail.value = null
    return
  }
  try {
    detail.value = await api.getMeeting(currentId.value, ownerToken.value || undefined)
    if (!silent) error.value = ''
  } catch (err) {
    if (!silent) error.value = err instanceof Error ? err.message : String(err)
  }
}

async function refreshAll(showSpinner = true) {
  if (showSpinner) refreshing.value = true
  await Promise.all([loadMeetings(true), loadDetail(true)])
  if (showSpinner) refreshing.value = false
}

async function createMeeting() {
  creating.value = true
  error.value = ''
  try {
    const created = await api.createMeeting({ ...form })
    saveOwnerSecret(created.meeting_id, {
      owner_token: created.owner_token,
      join_code: created.join_code,
    })
    form.title = ''
    form.goal = ''
    form.context = ''
    await loadMeetings(true)
    await router.push(`/meetings/${created.meeting_id}`)
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    creating.value = false
  }
}

async function startEarly() {
  if (!currentId.value || !ownerToken.value) return
  starting.value = true
  error.value = ''
  try {
    await api.startMeeting(currentId.value, ownerToken.value)
    await refreshAll(false)
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    starting.value = false
  }
}

async function answerQuestion() {
  const question = detail.value?.active_question
  if (!currentId.value || !ownerToken.value || !question || !answerText.value.trim()) return
  answering.value = true
  error.value = ''
  try {
    await api.answerMeetingQuestion(currentId.value, question.id, ownerToken.value, answerText.value.trim())
    answerText.value = ''
    await refreshAll(false)
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    answering.value = false
  }
}

async function copyText(text: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text)
    return
  }
  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.style.position = 'fixed'
  textarea.style.opacity = '0'
  document.body.appendChild(textarea)
  textarea.select()
  document.execCommand('copy')
  textarea.remove()
}

async function copyInvite() {
  const room = detail.value
  if (!room?.join_code) return
  const endpoint = room.mcp_endpoint || 'http://127.0.0.1:3847/mcp'
  const text = [
    '请加入文渊阁外部 Agent 会议。',
    `MCP: ${endpoint}`,
    `meeting_id: ${room.meeting_id}`,
    `join_code: ${room.join_code}`,
    '',
    '调用 wenyuan_join_meeting 加入；之后持续使用 wenyuan_next_task 获取任务、wenyuan_submit_turn 提交。等待期间发送 wenyuan_heartbeat。只有缺失信息会实质改变结论时才调用 wenyuan_ask_user。',
  ].join('\n')
  try {
    await copyText(text)
    copied.value = true
    window.setTimeout(() => { copied.value = false }, 1600)
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  }
}

watch(currentId, async () => {
  answerText.value = ''
  await loadDetail(false)
})

onMounted(async () => {
  await loadMeetings(false)
  await loadDetail(false)
  pollTimer = window.setInterval(() => {
    if (currentId.value && detail.value?.stage !== 'completed') {
      refreshAll(false)
    } else {
      loadMeetings(true)
    }
  }, 3000)
})

onBeforeUnmount(() => {
  if (pollTimer !== undefined) window.clearInterval(pollTimer)
})
</script>

<style scoped>
.meeting-page {
  max-width: 1440px;
}

.meeting-head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: var(--space-lg);
}

.meeting-head .secondary-btn {
  margin-bottom: 2px;
}

.meeting-shell {
  display: grid;
  grid-template-columns: 270px minmax(0, 1fr);
  gap: var(--space-lg);
  align-items: start;
}

.meeting-list-panel {
  position: sticky;
  top: 30px;
  min-height: 420px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  overflow: hidden;
}

.list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border-bottom: 1px solid var(--color-border-light);
}

.list-head > div {
  display: grid;
  gap: 2px;
}

.list-head strong {
  font-size: 14px;
}

.list-head span,
.updated-at,
.section-title-row > span,
.question-head > span {
  color: var(--color-text-muted);
  font-size: 12px;
}

.icon-btn,
.secondary-btn,
.primary-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.icon-btn {
  width: 34px;
  height: 34px;
  border: 1px solid var(--color-border-light);
  background: #fff;
  color: var(--color-text-muted);
}

.secondary-btn,
.primary-btn {
  min-height: 38px;
  padding: 8px 13px;
  font-size: 13px;
  font-weight: 600;
}

.secondary-btn {
  border: 1px solid var(--color-border);
  background: #fff;
  color: var(--color-text);
}

.secondary-btn:hover,
.icon-btn:hover {
  border-color: var(--color-border-hover);
  background: var(--color-surface-hover);
}

.primary-btn {
  border: 1px solid var(--color-accent);
  background: var(--color-accent);
  color: white;
}

.primary-btn:hover:not(:disabled) {
  background: var(--color-accent-hover);
}

.primary-btn:disabled,
.secondary-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.meeting-row {
  width: 100%;
  border: 0;
  border-bottom: 1px solid var(--color-border-light);
  background: transparent;
  text-align: left;
  padding: 14px 16px;
  cursor: pointer;
  display: grid;
  gap: 5px;
  transition: background var(--transition-fast), box-shadow var(--transition-fast);
}

.meeting-row:hover {
  background: var(--color-surface-hover);
}

.meeting-row.active {
  background: var(--color-accent-light);
  box-shadow: inset 3px 0 0 var(--color-accent);
}

.meeting-row-title {
  color: var(--color-text);
  font-weight: 650;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.meeting-row-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--color-text-muted);
  font-size: 11px;
}

.state-dot,
.presence {
  display: inline-block;
  flex: 0 0 auto;
  border-radius: 999px;
}

.state-dot {
  width: 7px;
  height: 7px;
  background: var(--color-text-dim);
}

.state-dot.active,
.state-dot.ok,
.presence.online {
  background: var(--color-success);
}

.state-dot.warn,
.presence.stale {
  background: #b27a23;
}

.state-dot.idle,
.presence.empty,
.presence.left {
  background: var(--color-text-dim);
}

.empty-list {
  padding: 28px 16px;
  color: var(--color-text-muted);
  font-size: 13px;
  text-align: center;
}

.meeting-workspace {
  min-width: 0;
}

.error-banner,
.notice {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  border-radius: var(--radius-md);
  padding: 13px 15px;
  margin-bottom: var(--space-md);
}

.error-banner {
  color: var(--color-danger);
  background: var(--color-danger-light);
  border: 1px solid rgba(154, 63, 52, 0.28);
}

.notice p {
  margin: 3px 0 0;
  font-size: 13px;
}

.warning-notice {
  border: 1px solid var(--color-warning-border);
  color: var(--color-warning-text);
  background: var(--color-warning-bg);
}

.readonly-notice {
  border: 1px solid var(--color-border);
  color: var(--color-text-muted);
  background: var(--color-surface-alt);
}

.create-panel {
  margin-top: 0;
  display: grid;
  grid-template-columns: minmax(220px, 0.7fr) minmax(360px, 1.3fr);
  gap: var(--space-xl);
}

.create-copy h2 {
  font-size: 22px;
  line-height: 1.4;
}

.create-copy p {
  color: var(--color-text-muted);
  line-height: 1.85;
}

.optional {
  color: var(--color-text-dim);
  font-weight: 400;
  font-size: 12px;
}

.participant-choice {
  display: grid;
  gap: 9px;
  margin-bottom: var(--space-lg);
  font-size: 14px;
  font-weight: 600;
}

.participant-choice small {
  color: var(--color-text-muted);
  font-weight: 400;
  line-height: 1.6;
}

.segmented {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
  padding: 4px;
  background: var(--color-surface-alt);
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
}

.segmented button {
  border: 1px solid transparent;
  border-radius: 3px;
  padding: 8px 12px;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  font-size: 13px;
}

.segmented button.selected {
  color: var(--color-accent-text);
  background: #fff;
  border-color: var(--color-border);
  box-shadow: var(--shadow-sm);
  font-weight: 650;
}

.create-action {
  width: 100%;
}

.room-header {
  display: flex;
  justify-content: space-between;
  gap: var(--space-lg);
  align-items: flex-start;
  padding: 3px 0 var(--space-lg);
}

.room-title-line {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.room-title-line h2 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 24px;
}

.room-header p {
  margin: 7px 0 0;
  color: var(--color-text-muted);
  max-width: 900px;
  line-height: 1.7;
}

.stage-badge,
.turn-status {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  border: 1px solid var(--color-border);
  padding: 3px 9px;
  font-size: 11px;
  font-weight: 650;
  background: var(--color-surface-alt);
  color: var(--color-text-muted);
}

.stage-badge.active,
.turn-status.claimed {
  border-color: rgba(15, 138, 161, 0.3);
  background: var(--color-accent-light);
  color: var(--color-accent-text);
}

.stage-badge.ok,
.turn-status.submitted {
  border-color: rgba(47, 93, 80, 0.25);
  background: rgba(47, 93, 80, 0.09);
  color: var(--color-success);
}

.stage-badge.warn,
.turn-status.skipped {
  border-color: var(--color-warning-border);
  background: var(--color-warning-bg);
  color: var(--color-warning-text);
}

.invite-panel,
.status-section,
.question-panel,
.final-panel,
.transcript-panel {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
}

.invite-panel {
  display: grid;
  grid-template-columns: minmax(210px, 0.7fr) minmax(320px, 1.3fr) auto;
  gap: var(--space-lg);
  align-items: center;
  padding: 18px 20px;
  margin-bottom: var(--space-lg);
}

.section-label {
  display: block;
  color: var(--color-accent-text);
  font-size: 10px;
  font-family: var(--font-mono);
  letter-spacing: 1.2px;
  text-transform: uppercase;
  margin-bottom: 4px;
}

.invite-copy {
  display: grid;
  gap: 3px;
}

.invite-copy p {
  margin: 4px 0 0;
  color: var(--color-text-muted);
  font-size: 12px;
  line-height: 1.55;
}

.invite-data {
  display: grid;
  gap: 6px;
}

.invite-data > div {
  display: grid;
  grid-template-columns: 82px minmax(0, 1fr);
  align-items: center;
  gap: 8px;
}

.invite-data span {
  color: var(--color-text-muted);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.7px;
}

.invite-data code {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text);
  font-family: var(--font-mono);
  font-size: 11px;
}

.invite-data .join-code {
  font-size: 14px;
  color: var(--color-accent-text);
  font-weight: 700;
  letter-spacing: 0.8px;
}

.invite-actions {
  display: grid;
  gap: 8px;
}

.status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-lg);
  margin-bottom: var(--space-lg);
}

.status-section,
.question-panel,
.final-panel,
.transcript-panel {
  padding: 18px 20px;
}

.section-title-row,
.question-head,
.turn-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-md);
}

.section-title-row h3,
.question-head h3,
.final-panel h3,
.transcript-panel h3 {
  margin: 0;
  font-size: 16px;
}

.participants-grid {
  display: grid;
  gap: 8px;
  margin-top: 14px;
}

.participant-card {
  display: grid;
  grid-template-columns: 10px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  min-height: 54px;
  padding: 9px 11px;
  border: 1px solid var(--color-border-light);
  background: #fff;
  border-radius: var(--radius-sm);
}

.presence {
  width: 8px;
  height: 8px;
}

.participant-card > div {
  display: grid;
}

.participant-card strong {
  font-size: 13px;
}

.participant-card small,
.participant-state {
  color: var(--color-text-muted);
  font-size: 11px;
}

.empty-slot {
  border-style: dashed;
  background: transparent;
}

.stage-track {
  display: grid;
  gap: 8px;
  margin-top: 14px;
}

.stage-step {
  display: grid;
  grid-template-columns: 32px 1fr;
  gap: 9px;
  align-items: center;
  min-height: 54px;
  padding: 8px 10px;
  border-left: 2px solid var(--color-border);
  color: var(--color-text-muted);
}

.stage-step.current {
  border-left-color: var(--color-accent);
  color: var(--color-text);
  background: var(--color-accent-light);
}

.stage-step.done {
  border-left-color: var(--color-success);
}

.stage-index {
  display: grid;
  place-items: center;
  color: currentColor;
}

.stage-step > div {
  display: grid;
}

.stage-step strong {
  font-size: 13px;
}

.stage-step small {
  font-size: 11px;
  color: var(--color-text-muted);
}

.question-panel,
.final-panel,
.transcript-panel {
  margin-bottom: var(--space-lg);
}

.question-panel {
  border-color: var(--color-warning-border);
  background: var(--color-warning-bg);
}

.question-head h3 {
  margin-top: 4px;
  font-size: 18px;
}

.question-panel > p {
  color: var(--color-warning-text);
  margin: 8px 0 14px;
  font-size: 13px;
}

.answer-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 10px;
  align-items: stretch;
}

.answer-row textarea {
  margin: 0;
  resize: vertical;
}

.final-panel {
  border-top: 3px solid var(--color-success);
}

.final-panel pre,
.turn-card pre {
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-body);
  margin: 14px 0 0;
  color: var(--color-text);
  line-height: 1.78;
}

.final-panel pre {
  font-size: 14px;
}

.degraded-note {
  color: var(--color-warning-text) !important;
}

.transcript-title {
  margin-bottom: 12px;
}

.stage-transcript {
  border-top: 1px solid var(--color-border-light);
}

.stage-transcript:last-child {
  border-bottom: 1px solid var(--color-border-light);
}

.stage-transcript summary {
  list-style: none;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  gap: var(--space-md);
  padding: 12px 2px;
  font-size: 13px;
  font-weight: 650;
}

.stage-transcript summary::-webkit-details-marker {
  display: none;
}

.stage-transcript summary span:last-child {
  color: var(--color-text-muted);
  font-size: 11px;
  font-weight: 400;
}

.turn-list {
  display: grid;
  gap: 9px;
  padding: 0 0 14px;
}

.turn-card {
  padding: 12px 14px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
  background: #fff;
}

.turn-card pre {
  font-size: 13px;
}

.stage-empty {
  padding: 0 0 12px;
  font-size: 12px;
}

.loading-panel {
  margin-top: 0;
}

.spinning {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 1100px) {
  .meeting-shell {
    grid-template-columns: 220px minmax(0, 1fr);
  }

  .invite-panel,
  .create-panel {
    grid-template-columns: 1fr;
  }

  .invite-actions {
    display: flex;
    flex-wrap: wrap;
  }
}

@media (max-width: 820px) {
  .meeting-head {
    align-items: flex-start;
  }

  .meeting-shell,
  .status-grid {
    grid-template-columns: 1fr;
  }

  .meeting-list-panel {
    position: static;
    min-height: 0;
    max-height: 240px;
    overflow-y: auto;
  }

  .answer-row {
    grid-template-columns: 1fr;
  }

  .room-header {
    display: grid;
  }
}
</style>
