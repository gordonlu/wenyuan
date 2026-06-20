<template>
  <div v-if="message" class="error-state" role="alert">
    <p>{{ message }}</p>
    <p v-if="hint" class="error-hint">{{ hint }}</p>
    <button v-if="actionLabel" class="error-action" @click="$emit('action')">{{ actionLabel }}</button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  message?: string
  hint?: string
  actionLabel?: string
}>()

defineEmits<{
  action: []
}>()
</script>

<style scoped>
.error-state {
  padding: 10px 14px;
  border-radius: var(--radius-sm);
  background: var(--color-danger-light);
  border: 1px solid rgba(154, 63, 52, 0.2);
  color: var(--color-danger);
  font-size: 13px;
  line-height: 1.5;
}

.error-state p {
  margin: 0;
}

.error-hint {
  margin-top: 6px !important;
  font-size: 12px;
  opacity: 0.8;
}

.error-action {
  margin-top: 8px;
  padding: 4px 12px;
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-danger);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.error-action:hover {
  background: var(--color-danger);
  color: #ffffff;
}
</style>
