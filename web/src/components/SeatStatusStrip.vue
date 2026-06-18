<template>
  <section class="seat-grid" aria-label="三席状态">
    <article v-for="seat in seats" :key="seat" class="seat-tile">
      <span>{{ seatLabels[seat] }}</span>
      <strong>
        <i :class="['status-dot', seatStatusClass(phase, seat, events, running)]" />
        {{ seatStatus(phase, seat, events, running) }}
      </strong>
    </article>
  </section>
</template>

<script setup lang="ts">
import { seatLabels, seatStatus, seatStatusClass, type SeatKind, type SessionEvent, type SessionPhase } from '../domain/session'

defineProps<{
  phase: SessionPhase
  events?: SessionEvent[]
  running?: boolean
}>()

const seats: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']
</script>
