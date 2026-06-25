<template>
  <div class="barchart-wrap" :style="{ height: height + 'px' }">
    <svg :viewBox="`0 0 ${width} ${height}`" class="barchart-svg">
      <line
        v-for="tick in ticks"
        :key="tick"
        :x1="padLeft" :x2="width - padRight"
        :y1="scaleY(tick)" :y2="scaleY(tick)"
        stroke="var(--color-border-light)"
        stroke-width="1"
        stroke-dasharray="3,3"
      />
      <g v-for="(group, gi) in groups" :key="gi">
        <rect
          v-for="(bar, bi) in group.bars"
          :key="bi"
          :x="barX(gi, bi)"
          :y="scaleY(bar.value)"
          :width="barW"
          :height="Math.max(1, height - padBottom - padTop - scaleY(bar.value))"
          :fill="bar.color"
          :rx="4"
          :ry="4"
          class="bar"
          style="transition: height 0.6s ease, y 0.6s ease"
        />
        <text
          v-if="showValues"
          v-for="(bar, bi) in group.bars"
          :key="'v' + bi"
          :x="barX(gi, bi) + barW / 2"
          :y="scaleY(bar.value) - 6"
          text-anchor="middle"
          class="bar-value"
        >{{ bar.value }}</text>
      </g>
      <text
        v-for="(group, gi) in groups"
        :key="'l' + gi"
        :x="groupCenterX(gi)"
        :y="height - 4"
        text-anchor="middle"
        class="bar-label"
      >{{ group.label }}</text>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

export interface BarItem {
  value: number
  color: string
}

export interface BarGroup {
  label: string
  bars: BarItem[]
}

const props = withDefaults(defineProps<{
  groups: BarGroup[]
  width?: number
  height?: number
  showValues?: boolean
}>(), { width: 400, height: 200, showValues: true })

const padLeft = 8
const padRight = 8
const padTop = 24
const padBottom = 28

const maxVal = computed(() => {
  let m = 0
  for (const g of props.groups) {
    for (const b of g.bars) {
      if (b.value > m) m = b.value
    }
  }
  return m || 1
})

const scaleY = computed(() => {
  const range = props.height - padTop - padBottom
  return (v: number) => padTop + range - (v / maxVal.value) * range
})

const groupCount = computed(() => props.groups.length)
const maxBars = computed(() => Math.max(...props.groups.map((g) => g.bars.length), 1))

const groupW = computed(() => (props.width - padLeft - padRight) / groupCount.value)
const barW = computed(() => Math.max(8, groupW.value / (maxBars.value + 1) - 2))
const groupGap = computed(() => groupW.value - maxBars.value * barW.value - (maxBars.value - 1) * 2)

function groupCenterX(gi: number) {
  return padLeft + groupW.value * gi + groupW.value / 2
}

function barX(gi: number, bi: number) {
  const startX = padLeft + groupW.value * gi + groupGap.value / 2
  return startX + bi * (barW.value + 2)
}

const ticks = computed(() => {
  const m = maxVal.value
  const step = Math.max(1, Math.ceil(m / 4))
  const t: number[] = []
  for (let i = 0; i <= m; i += step) t.push(i)
  if (t[t.length - 1] !== m) t.push(m)
  return t
})
</script>

<style scoped>
.barchart-wrap {
  width: 100%;
  max-width: 100%;
  overflow: hidden;
}
.barchart-svg {
  width: 100%;
  height: 100%;
}
.bar {
  cursor: pointer;
}
.bar:hover {
  opacity: 0.8;
}
.bar-value {
  font-size: 11px;
  font-weight: 700;
  fill: var(--color-text-muted);
}
.bar-label {
  font-size: 12px;
  fill: var(--color-text);
  font-weight: 600;
}
</style>
