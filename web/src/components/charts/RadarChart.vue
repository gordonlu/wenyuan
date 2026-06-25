<template>
  <div class="radar-wrap">
    <svg :viewBox="`0 0 ${size} ${size}`" :style="{ width: size + 'px', height: size + 'px' }" class="radar-svg" role="img" aria-label="讨论质量雷达图">
      <defs>
        <radialGradient id="rf" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stop-color="var(--color-accent)" stop-opacity="0.2" />
          <stop offset="100%" stop-color="var(--color-accent)" stop-opacity="0.04" />
        </radialGradient>
      </defs>

      <!-- Grid rings -->
      <circle
        v-for="ring in 4" :key="ring"
        :cx="cx" :cy="cy"
        :r="(ring / 4) * r"
        fill="none"
        stroke="var(--color-border-light)"
        stroke-width="1"
      />
      <text
        v-for="ring in 3" :key="'l' + ring"
        :x="cx + 4"
        :y="cy - (ring / 4) * r + 4"
        class="radar-ring-label"
      >{{ ring * 25 }}</text>

      <!-- Axes -->
      <line
        v-for="(_, i) in axes"
        :key="'a' + i"
        :x1="cx" :y1="cy"
        :x2="point(i, 1).x" :y2="point(i, 1).y"
        stroke="var(--color-border-light)"
        stroke-width="1"
      />

      <!-- Data polygon fill -->
      <polygon
        v-if="hasData"
        :points="dataPoints"
        fill="url(#rf)"
        stroke="var(--color-accent)"
        stroke-width="2"
        class="radar-poly"
      />

      <!-- Data points -->
      <circle
        v-for="(ax, i) in axes" :key="'d' + i"
        v-if="hasData"
        :cx="point(i, ax.value / (ax.max || 1)).x"
        :cy="point(i, ax.value / (ax.max || 1)).y"
        :r="4"
        fill="var(--color-accent)"
        stroke="#fff"
        stroke-width="2"
        class="radar-dot"
      />

      <!-- Axis labels -->
      <text
        v-for="(ax, i) in axes"
        :key="'t' + i"
        :x="labelPoint(i).x"
        :y="labelPoint(i).y"
        :text-anchor="labelAnchor(i)"
        :dominant-baseline="labelBaseline(i)"
        class="radar-axis-label"
      >{{ ax.label }}</text>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

export interface RadarAxis {
  label: string
  value: number
  max?: number
}

const props = withDefaults(defineProps<{
  axes: RadarAxis[]
  size?: number
}>(), { size: 280 })

const cx = computed(() => props.size / 2)
const cy = computed(() => props.size / 2)
const r = computed(() => props.size * 0.30)
const labelR = 1.25

const hasData = computed(() => props.axes.some((a) => a.value > 0))

const dataPoints = computed(() => {
  return props.axes
    .map((ax, i) => {
      const p = point(i, ax.value / (ax.max || 1))
      return `${p.x},${p.y}`
    })
    .join(' ')
})

function angle(i: number) {
  const slice = (2 * Math.PI) / props.axes.length
  return -Math.PI / 2 + i * slice
}

function point(i: number, ratio: number) {
  const a = angle(i)
  return {
    x: cx.value + r.value * ratio * Math.cos(a),
    y: cy.value + r.value * ratio * Math.sin(a),
  }
}

function labelPoint(i: number) {
  const a = angle(i)
  return {
    x: cx.value + r.value * labelR * Math.cos(a),
    y: cy.value + r.value * labelR * Math.sin(a),
  }
}

function labelAnchor(i: number) {
  const a = angle(i)
  const cos = Math.cos(a)
  if (cos > 0.05) return 'start'
  if (cos < -0.05) return 'end'
  return 'middle'
}

function labelBaseline(i: number) {
  const a = angle(i)
  const sin = Math.sin(a)
  if (sin > 0.05) return 'hanging'
  if (sin < -0.05) return 'baseline'
  return 'middle'
}
</script>

<style scoped>
.radar-wrap {
  display: flex;
  justify-content: center;
  overflow: visible;
}
.radar-svg {
  display: block;
  overflow: visible;
}
.radar-poly {
  transition: opacity 0.5s ease;
}
.radar-dot {
  cursor: pointer;
}
.radar-dot:hover {
  r: 6;
}
.radar-axis-label {
  font-size: 10px;
  font-weight: 600;
  fill: var(--color-text);
}
.radar-ring-label {
  font-size: 9px;
  fill: var(--color-text-dim);
}
</style>
