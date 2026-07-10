<template>
  <div class="tab-content">
    <nav class="tab-nav">
      <a v-if="suggestions.length" href="#followup-suggestions">建议</a>
      <a v-if="turns.length" href="#followup-timeline">历史</a>
      <a v-if="objects.length" href="#followup-redelib">复议</a>
    </nav>

    <div id="followup-suggestions">
      <FollowUpCards
        v-if="suggestions.length"
        :suggestions="suggestions"
        :loading="loading"
        @regenerate="$emit('regenerate')"
        @start="$emit('startFollowup', $event)"
      />
      <section v-else class="panel">
        <h2>续议建议</h2>
        <p class="muted">暂无续议建议。合议完成后将自动生成。</p>
      </section>
    </div>

    <div id="followup-timeline">
      <FollowUpTimeline v-if="turns.length" :turns="turns" />
    </div>

    <div id="followup-redelib">
      <ReDeliberationBox
        v-if="objects.length"
        :objects="objects"
        :running="reDelibRunning"
        :result="reDelibResult"
        :error-message="reDelibError"
        @submit="$emit('reDeliberate', $event)"
        @clear="$emit('clearReDelibError')"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import FollowUpCards from '../FollowUpCards.vue'
import FollowUpTimeline from '../FollowUpTimeline.vue'
import ReDeliberationBox from '../ReDeliberationBox.vue'

defineProps<{
  suggestions: any[]
  loading: boolean
  turns: any[]
  objects: any[]
  reDelibRunning: boolean
  reDelibResult: any
  reDelibError: string
}>()

defineEmits<{
  regenerate: []
  startFollowup: [payload: any]
  reDeliberate: [payload: any]
  clearReDelibError: []
}>()
</script>
