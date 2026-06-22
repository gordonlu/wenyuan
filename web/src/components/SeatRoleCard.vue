<template>
  <article
    :class="['seat-role-card', profile.className, statusClass, { 'report-mode': reportMode, 'is-running': isRunning, 'is-active': isActive, 'is-inactive': inactive }]"
    :style="cardStyle"
    :aria-busy="isRunning"
    role="region"
    :aria-label="`${seatLabels[seat]} — ${status}`"
    @pointermove="handlePointerMove"
    @pointerleave="resetPointer"
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
        <dt>失败</dt>
        <dd>{{ stats.failed }} 次</dd>
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
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { RotateCw } from '@lucide/vue'
import {
  seatLabels,
  seatRunStats,
  seatStatus,
  seatStatusClass,
  type SeatKind,
  type SeatRunTrace,
  type SessionEvent,
  type SessionPhase,
} from '../domain/session'

const props = withDefaults(defineProps<{
  seat: SeatKind
  phase: SessionPhase
  events?: SessionEvent[]
  running?: boolean
  runs?: SeatRunTrace[]
  providerRef?: string
  reportMode?: boolean
  inactive?: boolean
}>(), {
  events: () => [],
  running: false,
  runs: () => [],
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
</script>

<style scoped>
.seat-role-card {
  --role-main: #0f8aa1;
  --role-deep: #063f55;
  --role-soft: #c9fbff;
  --role-glow: rgba(15, 138, 161, 0.34);
  --pointer-x: 76%;
  --pointer-y: 24%;
  --tilt-x: 0deg;
  --tilt-y: 0deg;
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
  isolation: isolate;
  color: #ffffff;
  transform: perspective(900px) rotateX(var(--tilt-x)) rotateY(var(--tilt-y));
  transform-style: preserve-3d;
  transition: transform 180ms ease-out, box-shadow 180ms ease-out;
  will-change: transform;
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
  animation: seat-breath 2s ease-in-out infinite;
}

.seat-role-card.is-running::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: 2;
  border-radius: var(--radius-md);
  background:
    conic-gradient(from 0deg at 50% 50%,
      transparent 0deg,
      rgba(255, 255, 255, 0.15) 45deg,
      transparent 90deg,
      transparent 180deg,
      rgba(255, 255, 255, 0.08) 225deg,
      transparent 270deg,
      transparent 360deg);
  animation: seat-shimmer 3s linear infinite;
  pointer-events: none;
  mask-image: radial-gradient(circle 140% at 50% 50%, black 82%, transparent 100%);
  -webkit-mask-image: radial-gradient(circle 140% at 50% 50%, black 82%, transparent 100%);
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

@keyframes seat-breath {
  0%, 100% { box-shadow:
      0 0 0 5px rgba(255, 255, 255, 0.3),
      0 20px 42px rgba(16, 24, 40, 0.18),
      inset 0 0 0 1px rgba(0, 0, 0, 0.18),
      inset 0 1px 0 rgba(255, 255, 255, 0.42); }
  50% { box-shadow:
      0 0 0 5px rgba(255, 255, 255, 0.34),
      0 20px 52px rgba(16, 24, 40, 0.22),
      0 0 24px var(--role-glow),
      inset 0 0 0 1px rgba(0, 0, 0, 0.18),
      inset 0 1px 0 rgba(255, 255, 255, 0.48); }
}

@keyframes seat-shimmer {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

@keyframes seat-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.35; transform: scale(0.7); }
}

@media (prefers-reduced-motion: reduce) {
  .seat-role-card.is-running {
    animation: none;
    outline: 1px solid rgba(255, 255, 255, 0.3);
    outline-offset: 2px;
  }
  .seat-role-card.is-running::before,
  .seat-role-card.is-running .role-status::before {
    animation: none;
    display: none;
  }
}

.role-head,
.role-summary,
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
