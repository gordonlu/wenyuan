<template>
  <div class="donut-wrap">
    <svg :viewBox="`0 0 ${size} ${size}`" class="donut-svg" :style="{ width: size + 'px', height: size + 'px' }">
      <circle
        :cx="cx" :cy="cy" :r="r"
        fill="none"
        stroke="var(--color-border-light)"
        :stroke-width="strokeWidth"
      />
      <circle
        v-for="(s, i) in normalized"
        :key="i"
        :cx="cx" :cy="cy" :r="r"
        fill="none"
        :stroke="s.color"
        :stroke-width="strokeWidth"
        :stroke-dasharray="`${s.arcLength} ${circumference}`"
        :stroke-dashoffset="-s.offset"
        transform="rotate(-90, cx, cy)"
        class="donut-segment"
        style="transition: stroke-dasharray 0.8s ease"
      />
      <text :x="cx" :y="cy - 6" text-anchor="middle" class="donut-total">{{ total }}</text>
      <text :x="cx" :y="cy + 14" text-anchor="middle" class="donut-label">来源</text>
    </svg>
    <div class="donut-legend">
      <div v-for="(s, i) in normalized" :key="i" class="legend-row">
        <span class="legend-dot" :style="{ background: s.color }" />
        <span class="legend-label">{{ s.label }}</span>
        <span class="legend-value">{{ s.value }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

export interface DonutSegment {
  label: string
  value: number
  color: string
}

const props = withDefaults(defineProps<{
  segments: DonutSegment[]
  size?: number
}>(), { size: 160 })

const cx = computed(() => props.size / 2)
const cy = computed(() => props.size / 2)
const r = computed(() => props.size * 0.35)
const strokeWidth = computed(() => props.size * 0.1)
const circumference = computed(() => 2 * Math.PI * r.value)

const total = computed(() => props.segments.reduce((sum, s) => sum + s.value, 0))

const normalized = computed(() => {
  const t = total.value
  if (!t) return []
  let cumulative = 0
  return props.segments.map((s) => {
    const pct = s.value / t
    const arcLength = pct * circumference.value
    const offset = cumulative * circumference.value
    cumulative += pct
    return { ...s, pct, arcLength, offset: offset }
  })
})
</script>

<style scoped>
.donut-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
.donut-svg {
  flex-shrink: 0;
}
.donut-segment {
  cursor: pointer;
}
.donut-segment:hover {
  opacity: 0.8;
}
.donut-total {
  font-size: 22px;
  font-weight: 800;
  fill: var(--color-text);
}
.donut-label {
  font-size: 11px;
  fill: var(--color-text-muted);
}
.donut-legend {
  display: flex;
  gap: 8px 14px;
  flex-wrap: wrap;
  justify-content: center;
}
.legend-row {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
}
.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.legend-label {
  color: var(--color-text-muted);
}
.legend-value {
  font-weight: 600;
  color: var(--color-text);
}
</style>
