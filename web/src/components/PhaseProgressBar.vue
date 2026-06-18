<template>
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
</template>

<script setup lang="ts">
import type { SessionPhase } from '../domain/session'

const props = defineProps<{
  phase: SessionPhase
}>()

const steps = [
  { phase: 'draft' as SessionPhase, label: '待陈策', icon: '①' },
  { phase: 'independent_deliberation' as SessionPhase, label: '独议', icon: '②' },
  { phase: 'cross_critique' as SessionPhase, label: '批议', icon: '③' },
  { phase: 'revision' as SessionPhase, label: '复议', icon: '④' },
  { phase: 'voting' as SessionPhase, label: '阁议', icon: '⑤' },
  { phase: 'convergence' as SessionPhase, label: '合案', icon: '⑥' },
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

function isDone(phase: SessionPhase) {
  const currentIdx = phaseOrder.indexOf(props.phase)
  const stepIdx = phaseOrder.indexOf(phase)
  if (props.phase === 'failed' || props.phase === 'cancelled') return stepIdx <= currentIdx
  return stepIdx < currentIdx
}
</script>
