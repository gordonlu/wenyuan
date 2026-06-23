<template>
  <div
    :class="['seat-border', profile.className, statusClass, { 'report-mode': reportMode, 'is-running': isRunning, 'is-active': isActive, 'is-inactive': inactive }]"
    :style="cardStyle"
    @pointermove="handlePointerMove"
    @pointerleave="resetPointer"
  >
    <article
      :class="['seat-role-card', profile.className, statusClass, { 'report-mode': reportMode, 'is-running': isRunning, 'is-active': isActive, 'is-inactive': inactive }]"
      :aria-busy="isRunning"
      role="region"
      :aria-label="`${seatLabels[seat]} — ${status}`"
    >
      <div
        class="ink-name"
        :style="{ backgroundImage: 'url(' + profile.inkUrl + ')' }"
        aria-hidden="true"
      />
      <div class="holo-layer holo-full" aria-hidden="true" />
      <div class="holo-layer holo-gallery" aria-hidden="true" />
      <div class="holo-layer holo-reverse" aria-hidden="true" />
      <div class="role-head">
        <div>
          <span class="role-kicker">{{ profile.kicker }}</span>
          <h3>{{ seatLabels[seat] }}</h3>
        </div>
        <span v-if="!reportMode" :class="['badge', 'role-status', badgeClass]">{{ status }}</span>
      </div>

      <p class="role-summary">{{ profile.summary }}</p>

      <div v-if="!reportMode && currentActivity" :class="['role-live', currentActivity.tone]" aria-live="polite">
        <span class="role-live-dot" aria-hidden="true" />
        <strong>{{ currentActivity.label }}</strong>
        <span v-if="currentActivity.detail" class="role-live-detail">{{ currentActivity.detail }}</span>
      </div>

      <dl v-if="!reportMode" class="role-metrics">
        <div>
          <dt>模型</dt>
          <dd>{{ modelName(providerRef) }}</dd>
        </div>
        <div>
          <dt>调用</dt>
          <dd>{{ stats.calls }} 次</dd>
        </div>
        <div>
          <dt>耗时</dt>
          <dd>{{ (stats.durationMs / 1000).toFixed(1) }} 秒</dd>
        </div>
        <div>
          <dt>工具</dt>
          <dd>{{ toolStats.calls }} 次</dd>
        </div>
      </dl>

      <dl v-else class="role-metrics role-metrics-report">
        <div>
          <dt>调用</dt>
          <dd>{{ stats.calls }} 次</dd>
        </div>
        <div>
          <dt>耗时</dt>
          <dd>{{ (stats.durationMs / 1000).toFixed(1) }} 秒</dd>
        </div>
      </dl>

      <div v-if="!reportMode" class="role-foot">
        <span class="prompt-version">{{ stats.promptVersions || '暂无 Prompt 版本' }}</span>
        <span v-if="stats.failed || toolStats.failed" class="runtime-failures">失败 {{ stats.failed + toolStats.failed }} 次</span>
      </div>
    </article>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  seatLabels,
  seatRunStats,
  seatStatus,
  seatStatusClass,
  toolNameLabel,
  type SeatKind,
  type SeatRunTrace,
  type SessionEvent,
  type SessionPhase,
  type ToolRun,
} from '../domain/session'

const props = withDefaults(defineProps<{
  seat: SeatKind
  phase: SessionPhase
  events?: SessionEvent[]
  running?: boolean
  runs?: SeatRunTrace[]
  toolRuns?: ToolRun[]
  providerRef?: string
  reportMode?: boolean
  inactive?: boolean
}>(), {
  events: () => [],
  running: false,
  runs: () => [],
  toolRuns: () => [],
  providerRef: '',
  reportMode: false,
  inactive: false,
})

function modelName(ref: string) {
  if (!ref) return '默认配置'
  const stripped = ref.replace(/^(openai-compatible[=:]|openai[:=])/i, '')
  return stripped || ref
}

const cardStyle = ref<Record<string, string>>({
  '--pointer-x': '76%',
  '--pointer-y': '24%',
  '--tilt-x': '0deg',
  '--tilt-y': '0deg',
})

const profiles: Record<SeatKind, { kicker: string; summary: string; className: string; inkUrl: string }> = {
  mouyuan: {
    kicker: '找新路径',
    summary: '负责提出备选方向、机会窗口和不寻常的解法。',
    className: 'mouyuan',
    inkUrl: '/role-ink/mouyuan-ink.png',
  },
  jingshi: {
    kicker: '看落地',
    summary: '负责把方案落到资源、成本、节奏和执行条件上。',
    className: 'jingshi',
    inkUrl: '/role-ink/jingshi-ink.png',
  },
  chizheng: {
    kicker: '守边界',
    summary: '负责检查风险、证据、反例和不可接受的代价。',
    className: 'chizheng',
    inkUrl: '/role-ink/chizheng-ink.png',
  },
}

const profile = computed(() => profiles[props.seat])
const status = computed(() => seatStatus(props.phase, props.seat, props.events, props.running))
const statusClass = computed(() => seatStatusClass(props.phase, props.seat, props.events, props.running))
const isRunning = computed(() => props.running && ['pending', 'active'].includes(statusClass.value))
const isActive = computed(() => !props.running && statusClass.value === 'active')
const stats = computed(() => seatRunStats(props.runs).find((item) => item.seat === props.seat) ?? {
  seat: props.seat,
  calls: 0,
  failed: 0,
  repaired: 0,
  durationMs: 0,
  tokens: 0,
  hasUsage: false,
  promptVersions: '',
})
const seatToolRuns = computed(() => props.toolRuns.filter((run) => run.seat === props.seat))
const toolStats = computed(() => ({
  calls: seatToolRuns.value.length,
  failed: seatToolRuns.value.filter((run) => run.status !== 'completed').length,
}))
const currentActivity = computed(() => {
  if (props.reportMode || !props.running) return null
  const latest = [...props.events]
    .reverse()
    .find((event) => runtimeEventForSeat(event, props.seat))
  if (!latest) return { label: '等待调度', detail: '', tone: 'idle' }
  const payload = eventPayload(latest)
  const query = typeof payload.query === 'string' ? payload.query : ''
  const count = typeof payload.count === 'number' ? payload.count : undefined
  const toolName = typeof payload.tool_name === 'string' ? payload.tool_name : undefined
  const toolLabel = toolActionLabel(toolName)

  if (latest.event_type === 'tool_started') return { label: `${toolLabel}中`, detail: query, tone: 'tool' }
  if (latest.event_type === 'tool_completed') return { label: `${toolLabel}完成`, detail: query ? `${query}${typeof count === 'number' ? ` · ${count} 条` : ''}` : '', tone: 'done' }
  if (latest.event_type === 'tool_failed') return { label: `${toolLabel}失败`, detail: query, tone: 'warn' }
  if (latest.event_type === 'seat_started') return { label: '模型调用中', detail: '', tone: 'model' }
  if (latest.event_type === 'seat_completed') return { label: '模型已返回', detail: '', tone: 'done' }
  if (latest.event_type === 'seat_failed') return { label: '模型调用失败', detail: '', tone: 'warn' }
  return null
})

const badgeClass = computed(() => {
  if (statusClass.value === 'ok') return 'ok'
  if (statusClass.value === 'danger') return 'danger'
  if (statusClass.value === 'pending' || statusClass.value === 'active') return 'warn'
  return ''
})

function handlePointerMove(event: PointerEvent) {
  const target = event.currentTarget as HTMLElement
  const rect = target.getBoundingClientRect()
  const x = (event.clientX - rect.left) / rect.width
  const y = (event.clientY - rect.top) / rect.height
  const tiltY = (x - 0.5) * 10
  const tiltX = (0.5 - y) * 8

  cardStyle.value = {
    '--pointer-x': `${Math.round(x * 100)}%`,
    '--pointer-y': `${Math.round(y * 100)}%`,
    '--tilt-x': `${tiltX.toFixed(2)}deg`,
    '--tilt-y': `${tiltY.toFixed(2)}deg`,
  }
}

function resetPointer() {
  cardStyle.value = {
    '--pointer-x': '76%',
    '--pointer-y': '24%',
    '--tilt-x': '0deg',
    '--tilt-y': '0deg',
  }
}

function runtimeEventForSeat(event: SessionEvent, seat: SeatKind) {
  if (!['tool_started', 'tool_completed', 'tool_failed', 'seat_started', 'seat_completed', 'seat_failed'].includes(event.event_type)) return false
  return eventPayload(event).seat === seat
}

function eventPayload(event: SessionEvent) {
  return (typeof event.payload === 'object' && event.payload !== null) ? event.payload as Record<string, unknown> : {}
}

function toolActionLabel(name?: string) {
  if (name === 'web_search') return '搜索'
  return name ? toolNameLabel(name) : '工具'
}
</script>

<style scoped>
.seat-border {
  --pointer-x: 76%;
  --pointer-y: 24%;
  --tilt-x: 0deg;
  --tilt-y: 0deg;
  position: relative;
  border-radius: calc(var(--radius-md) + 6px);
  transform: perspective(900px) rotateX(var(--tilt-x)) rotateY(var(--tilt-y));
  transform-style: preserve-3d;
  will-change: transform;
  transition: transform 180ms ease-out, box-shadow 180ms ease-out;
}

.seat-role-card {
  --role-main: #0f8aa1;
  --role-deep: #063f55;
  --role-soft: #c9fbff;
  --role-glow: rgba(15, 138, 161, 0.34);
  position: relative;
  overflow: hidden;
  min-height: 256px;
  padding: 18px;
  border: 0;
  border-radius: var(--radius-md);
  background:
    radial-gradient(circle at 18% 10%, rgba(255, 255, 255, 0.42), transparent 28%),
    linear-gradient(145deg, color-mix(in srgb, var(--role-main) 82%, #ffffff), var(--role-main) 46%, var(--role-deep));
  box-shadow:
    0 0 0 5px rgba(255, 255, 255, 0.3),
    0 20px 42px rgba(16, 24, 40, 0.18),
    inset 0 0 0 1px rgba(0, 0, 0, 0.18),
    inset 0 1px 0 rgba(255, 255, 255, 0.42);
  color: #ffffff;
}

.seat-role-card:hover {
  box-shadow:
    0 0 0 5px rgba(255, 255, 255, 0.3),
    0 26px 54px rgba(16, 24, 40, 0.24),
    inset 0 0 0 1px rgba(0, 0, 0, 0.18),
    inset 0 1px 0 rgba(255, 255, 255, 0.52);
}

.seat-role-card.report-mode {
  min-height: 220px;
}

.seat-role-card.mouyuan {
  --role-main: #0f8aa1;
  --role-deep: #063f55;
  --role-soft: #d7f6fb;
  --role-glow: rgba(15, 138, 161, 0.36);
}

.seat-role-card.jingshi {
  --role-main: #c77a00;
  --role-deep: #6f3f00;
  --role-soft: #ffe8a3;
  --role-glow: rgba(199, 122, 0, 0.32);
}

.seat-role-card.chizheng {
  --role-main: #b62662;
  --role-deep: #641536;
  --role-soft: #ffd7e6;
  --role-glow: rgba(182, 38, 98, 0.3);
}

.holo-layer {
  position: absolute;
  inset: -10px;
  z-index: 1;
  pointer-events: none;
  background-repeat: no-repeat;
  transition: opacity 160ms ease-out, filter 160ms ease-out, mask-image 160ms ease-out;
}

.holo-full {
  opacity: 0.06;
  background:
    url('/role-holo-full-art.png'),
    linear-gradient(118deg, rgba(255, 70, 218, 0.26) 0%, rgba(83, 255, 233, 0.42) 24%, rgba(255, 234, 74, 0.32) 45%, rgba(97, 126, 255, 0.36) 68%, rgba(255, 84, 210, 0.26) 100%),
    radial-gradient(circle at var(--pointer-x) var(--pointer-y), rgba(255, 255, 255, 0.16), transparent 34%);
  background-size: cover, 100% 100%, 100% 100%;
  background-position:
    center,
    center,
    0 0;
  mix-blend-mode: screen;
  filter: saturate(1.4) contrast(1.02);
}

.holo-gallery {
  opacity: 0.03;
  background:
    url('/role-holo-gallery.png'),
    repeating-linear-gradient(118deg, rgba(255, 255, 255, 0) 0 13px, rgba(255, 255, 255, 0.1) 14px 15px),
    radial-gradient(circle at var(--pointer-x) var(--pointer-y), rgba(255, 255, 255, 0.14), transparent 26%),
    conic-gradient(from 140deg at var(--pointer-x) var(--pointer-y), rgba(91, 255, 237, 0.3), rgba(255, 88, 220, 0.2), rgba(255, 238, 82, 0.28), rgba(91, 124, 255, 0.2), rgba(91, 255, 237, 0.3));
  background-size: cover, 240px 240px, 100% 100%, 100% 100%;
  background-position:
    center,
    center,
    0 0,
    0 0;
  mix-blend-mode: screen;
  filter: saturate(1.55) contrast(1.03);
}

.holo-reverse {
  opacity: 0.015;
  background:
    url('/role-holo-reverse.png'),
    linear-gradient(105deg, transparent 20%, rgba(255, 255, 255, 0.18) 48%, transparent 72%);
  background-size: cover, 100% 100%;
  background-position:
    center,
    0 0;
  mix-blend-mode: screen;
  filter: saturate(1.2) brightness(1.08);
}

.seat-role-card:hover .holo-full {
  opacity: 0.5;
  filter: saturate(2.65) contrast(1.12);
  mask-image: radial-gradient(circle 190px at var(--pointer-x) var(--pointer-y), rgba(0, 0, 0, 0.9) 0 18%, rgba(0, 0, 0, 0.56) 50%, rgba(0, 0, 0, 0.16) 82%, transparent 100%);
}

.seat-role-card:hover .holo-gallery {
  opacity: 0.36;
  filter: saturate(2.9) contrast(1.12);
  mask-image: radial-gradient(circle 212px at var(--pointer-x) var(--pointer-y), rgba(0, 0, 0, 0.88) 0 16%, rgba(0, 0, 0, 0.5) 50%, rgba(0, 0, 0, 0.14) 82%, transparent 100%);
}

.seat-role-card:hover .holo-reverse {
  opacity: 0.1;
  mask-image: radial-gradient(circle 166px at var(--pointer-x) var(--pointer-y), rgba(0, 0, 0, 0.72) 0 16%, rgba(0, 0, 0, 0.38) 50%, rgba(0, 0, 0, 0.1) 82%, transparent 100%);
}

/* ── Running state ── */
.seat-role-card.is-running {
  box-shadow:
    0 0 0 5px rgba(255, 255, 255, 0.3),
    0 20px 42px rgba(16, 24, 40, 0.18),
    0 0 22px var(--role-glow),
    inset 0 0 0 1px rgba(0, 0, 0, 0.18),
    inset 0 1px 0 rgba(255, 255, 255, 0.42);
}

.seat-border.is-running::before {
  content: '';
  position: absolute;
  inset: -5px;
  z-index: 4;
  padding: 6px;
  border-radius: calc(var(--radius-md) + 6px);
  will-change: transform;
  background: conic-gradient(from var(--flow-angle, 0deg),
      /* 全圈幻彩：彩虹色环，透明度波动产生流动感 */
      hsl(calc(0 + var(--hue, 0)) 90% 64% / 0.50) 0deg,
      hsl(calc(45 + var(--hue, 0)) 88% 66% / 0.62) 45deg,
      hsl(calc(90 + var(--hue, 0)) 86% 68% / 0.76) 90deg,
      hsl(calc(135 + var(--hue, 0)) 86% 68% / 0.86) 135deg,
      hsl(calc(180 + var(--hue, 0)) 88% 66% / 0.92) 180deg,
      hsl(calc(225 + var(--hue, 0)) 90% 66% / 0.86) 225deg,
      hsl(calc(270 + var(--hue, 0)) 90% 66% / 0.74) 270deg,
      hsl(calc(315 + var(--hue, 0)) 90% 64% / 0.60) 315deg,
      hsl(calc(360 + var(--hue, 0)) 90% 64% / 0.50) 360deg);
  animation: seat-colorflow 6s linear infinite, seat-hue-cycle 10s linear infinite;
  pointer-events: none;
  mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
  mask-composite: exclude;
  -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
  -webkit-mask-composite: xor;
}

.seat-role-card.is-running::after {
  content: '';
  position: absolute;
  inset: 0;
  z-index: 2;
  border-radius: calc(var(--radius-md) - 4px);
  box-shadow: inset 0 0 22px rgba(255, 255, 255, 0.18);
  pointer-events: none;
}

.seat-role-card.is-running .role-status {
  position: relative;
  padding-left: 22px;
}

.seat-role-card.is-running .role-status::before {
  content: '';
  position: absolute;
  left: 8px;
  top: 50%;
  width: 7px;
  height: 7px;
  margin-top: -3.5px;
  border-radius: 50%;
  background: var(--role-soft);
  animation: seat-pulse 1.4s ease-in-out infinite;
}

@property --flow-angle {
  syntax: '<angle>';
  initial-value: 0deg;
  inherits: false;
}

@property --hue {
  syntax: '<number>';
  initial-value: 0;
  inherits: false;
}

@keyframes seat-colorflow {
  0% { --flow-angle: 0deg; }
  100% { --flow-angle: 360deg; }
}

@keyframes seat-hue-cycle {
  0% { --hue: 0; }
  100% { --hue: 360; }
}

@keyframes seat-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.35; transform: scale(0.7); }
}

@media (prefers-reduced-motion: reduce) {
  .seat-border.is-running {
    outline: 1px solid rgba(255, 255, 255, 0.3);
    outline-offset: 2px;
  }
  .seat-border.is-running::before,
  .seat-role-card.is-running::after,
  .seat-role-card.is-running .role-status::before {
    animation: none;
    display: none;
  }
  .role-live.tool .role-live-dot,
  .role-live.model .role-live-dot {
    animation: none;
  }
}

.role-head,
.role-summary,
.role-live,
.role-metrics,
.role-foot,
.ink-name {
  position: relative;
  z-index: 3;
}

.ink-name {
  position: absolute;
  inset: -10px -6px;
  z-index: 0;
  background-position: 98% center;
  background-repeat: no-repeat;
  background-size: 90% auto;
  opacity: 0.15;
  pointer-events: none;
  filter: contrast(1.04) saturate(0.92);
  mix-blend-mode: multiply;
}

.seat-role-card.jingshi .ink-name {
  background-size: 80% auto;
}

.role-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.role-kicker {
  display: block;
  margin-bottom: 2px;
  color: rgba(255, 255, 255, 0.76);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0;
}

.role-head h3 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 25px;
  line-height: 1.15;
  color: #ffffff;
  text-shadow: 0 1px 18px rgba(0, 0, 0, 0.22);
}

.role-summary {
  min-height: 52px;
  margin: 78px 0 14px;
  color: rgba(255, 255, 255, 0.86);
  font-size: 13px;
  line-height: 1.55;
  text-shadow: 0 1px 10px rgba(0, 0, 0, 0.2);
}

.role-live {
  display: flex;
  align-items: center;
  gap: 7px;
  min-height: 32px;
  margin: -2px 0 12px;
  padding: 6px 9px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: var(--radius-sm);
  background: rgba(7, 12, 18, 0.28);
  color: rgba(255, 255, 255, 0.88);
  font-size: 12px;
  backdrop-filter: blur(10px);
}

.role-live strong {
  flex: 0 0 auto;
  color: #ffffff;
  font-size: 12px;
}

.role-live-detail {
  min-width: 0;
  overflow: hidden;
  color: rgba(255, 255, 255, 0.68);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.role-live-dot {
  flex: 0 0 auto;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--role-soft);
  box-shadow: 0 0 12px var(--role-soft);
}

.role-live.tool .role-live-dot,
.role-live.model .role-live-dot {
  animation: seat-pulse 1.2s ease-in-out infinite;
}

.role-live.warn {
  border-color: rgba(255, 255, 255, 0.34);
  background: rgba(82, 24, 31, 0.34);
}

.role-metrics {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0;
  margin: 0;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: var(--radius-sm);
  background: rgba(9, 14, 20, 0.24);
  backdrop-filter: blur(10px);
}

.role-metrics div {
  min-width: 0;
  padding: 9px 10px;
  border-right: 1px solid rgba(255, 255, 255, 0.14);
  border-bottom: 1px solid rgba(255, 255, 255, 0.14);
}

.role-metrics div:nth-child(2n) {
  border-right: 0;
}

.role-metrics div:nth-last-child(-n + 2) {
  border-bottom: 0;
}

.role-metrics dt {
  color: rgba(255, 255, 255, 0.58);
  font-size: 11px;
}

.role-metrics dd {
  overflow: hidden;
  margin: 2px 0 0;
  color: #ffffff;
  font-size: 13px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.role-metrics-report {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.role-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 12px;
}

.role-status {
  min-width: 66px;
  height: 26px;
  justify-content: center;
  border-color: rgba(255, 255, 255, 0.3);
  background: rgba(255, 255, 255, 0.18);
  color: #ffffff;
  white-space: nowrap;
}

.prompt-version {
  overflow: hidden;
  color: rgba(255, 255, 255, 0.62);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.runtime-failures {
  flex: 0 0 auto;
  color: rgba(255, 235, 235, 0.86);
  font-size: 11px;
  font-weight: 700;
  white-space: nowrap;
}

@media (prefers-reduced-motion: reduce) {
  .seat-role-card {
    transform: none;
  }
}

.seat-role-card.is-inactive {
  opacity: 0.45;
  filter: grayscale(0.86);
  pointer-events: none;
}
.seat-role-card.is-inactive .holo-layer {
  opacity: 0 !important;
}
.seat-role-card.is-inactive .ink-name {
  opacity: 0.15;
}
</style>
