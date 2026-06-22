<template>
  <div class="phase-progress-wrap">
    <nav class="phase-progress" aria-label="合议阶段进度">
      <template v-for="(step, index) in steps" :key="step.phase">
        <div
          :class="['phase-step', { active: step.phase === phase, done: isDone(step.phase) }]"
          :title="step.label"
        >
          <span class="phase-step-circle">{{ step.icon }}</span>
          <span class="phase-step-label">{{ step.label }}</span>
        </div>
        <span v-if="index < steps.length - 1" class="phase-connector" />
      </template>
    </nav>
    <p v-if="statusText" class="phase-status-text" aria-live="polite">{{ statusText }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { phaseLabels, seatLabels, seatStatus, type SeatKind, type SessionEvent, type SessionPhase } from '../domain/session'

const props = defineProps<{
  phase: SessionPhase
  running?: boolean
  events?: SessionEvent[]
}>()

const steps = [
  { phase: 'draft' as SessionPhase, label: '待陈策', icon: '1' },
  { phase: 'independent_deliberation' as SessionPhase, label: '独议', icon: '2' },
  { phase: 'cross_critique' as SessionPhase, label: '批议', icon: '3' },
  { phase: 'revision' as SessionPhase, label: '复议', icon: '4' },
  { phase: 'voting' as SessionPhase, label: '阁议', icon: '5' },
  { phase: 'convergence' as SessionPhase, label: '合案', icon: '6' },
  { phase: 'completed' as SessionPhase, label: '完成', icon: '✓' },
]

const phaseOrder: SessionPhase[] = [
  'draft',
  'independent_deliberation',
  'cross_critique',
  'revision',
  'voting',
  'convergence',
  'completed',
]

const runningSeatEvents = computed(() =>
  (props.events ?? []).filter((e) =>
    ['seat_started', 'seat_completed', 'seat_failed'].includes(e.event_type)
  )
)

const statusText = computed(() => {
  if (!props.running && props.phase !== 'convergence') return ''
  if (props.phase === 'convergence') return '三席意见分散，进入合案复议'
  if (props.phase === 'completed') return ''

  const latest = runningSeatEvents.value[runningSeatEvents.value.length - 1]
  if (latest?.event_type === 'seat_started') {
    const seat = (latest.payload as { seat?: SeatKind })?.seat
    if (seat) {
      const labels: Record<string, string> = {
        independent_deliberation: `${seatLabels[seat]}正在独议`,
        cross_critique: `${seatLabels[seat]}正在批议`,
        revision: `${seatLabels[seat]}正在复议`,
        voting: `${seatLabels[seat]}正在投票`,
      }
      return labels[props.phase] ?? `${seatLabels[seat]}正在${phaseLabels[props.phase]}`
    }
  }

  if (props.phase === 'independent_deliberation') return '三席正在独立陈策'
  if (props.phase === 'cross_critique') return '三席正在交叉批议'
  if (props.phase === 'revision') return '三席正在修订策案'
  if (props.phase === 'voting') return '三席正在阁议投票'
  return ''
})

function isDone(phase: SessionPhase) {
  const currentIdx = phaseOrder.indexOf(props.phase)
  const stepIdx = phaseOrder.indexOf(phase)
  if (props.phase === 'failed' || props.phase === 'cancelled') return stepIdx <= currentIdx
  return stepIdx < currentIdx
}
</script>

<style scoped>
.phase-progress-wrap {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.phase-progress {
  display: flex;
  align-items: center;
  gap: 0;
}

.phase-step {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
}

.phase-step-circle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
  background: var(--color-surface-alt);
  color: var(--color-text-muted);
  transition: background 200ms, color 200ms, box-shadow 200ms;
}

.phase-step.active .phase-step-circle {
  background: var(--color-accent);
  color: #ffffff;
  box-shadow: 0 0 0 3px var(--color-accent-light);
  animation: step-pulse 1.6s ease-in-out infinite;
}

.phase-step.done .phase-step-circle {
  background: var(--color-success);
  color: #ffffff;
}

.phase-step-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  white-space: nowrap;
}

.phase-step.done .phase-step-label {
  color: var(--color-text-muted);
}

.phase-connector {
  display: block;
  width: 20px;
  height: 2px;
  margin: 0 5px;
  background: var(--color-border);
  border-radius: 1px;
  flex-shrink: 0;
}

.phase-step.done + .phase-connector {
  background: var(--color-success);
}

.phase-step.active + .phase-connector {
  background: var(--color-accent);
}

.phase-status-text {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-accent);
  animation: status-fade 200ms ease-out;
}

@keyframes step-pulse {
  0%, 100% { box-shadow: 0 0 0 3px var(--color-accent-light); }
  50% { box-shadow: 0 0 0 5px var(--color-accent-light), 0 0 12px var(--color-accent-light); }
}

@keyframes status-fade {
  from { opacity: 0; transform: translateY(-3px); }
  to { opacity: 1; transform: translateY(0); }
}

@media (prefers-reduced-motion: reduce) {
  .phase-step.active .phase-step-circle {
    animation: none;
    box-shadow: 0 0 0 3px var(--color-accent-light);
  }
  .phase-status-text {
    animation: none;
  }
}
</style>
