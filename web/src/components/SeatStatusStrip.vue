<template>
  <section class="seat-grid" aria-label="三席状态">
    <article v-for="seat in seats" :key="seat" class="seat-tile">
      <span>{{ seatLabels[seat] }}</span>
      <strong>{{ seatStatus(phase, seat, events) }}</strong>
    </article>
  </section>
</template>

<script setup lang="ts">
import { seatLabels, seatStatus, type SeatKind, type SessionEvent, type SessionPhase } from '../domain/session'

defineProps<{
  phase: SessionPhase
  events?: SessionEvent[]
}>()

const seats: SeatKind[] = ['mouyuan', 'jingshi', 'chizheng']
</script>
