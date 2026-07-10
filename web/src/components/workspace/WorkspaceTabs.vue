<template>
  <nav class="workspace-tabs" role="tablist" aria-label="工作台页签">
    <button
      v-for="t in tabs"
      :key="t.key"
      type="button"
      role="tab"
      :aria-selected="tab === t.key"
      :class="{ active: tab === t.key }"
      @click="$emit('update:tab', t.key)"
    >
      {{ t.label }}
    </button>
  </nav>
</template>

<script setup lang="ts">
defineProps<{ tab: string }>()
defineEmits<{ 'update:tab': [tab: string] }>()

const tabs = [
  { key: 'overview', label: '概览' },
  { key: 'process', label: '合议过程' },
  { key: 'evidence', label: '证据' },
  { key: 'decision', label: '决策' },
  { key: 'followup', label: '续议' },
  { key: 'audit', label: '审计' },
]
</script>

<style scoped>
.workspace-tabs {
  display: flex;
  gap: 2px;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
  padding: 0 var(--space-md);
  background: var(--color-bg);
  border-bottom: 1px solid var(--color-border);
}
.workspace-tabs button {
  padding: 10px 20px;
  border: none;
  background: transparent;
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-muted);
  cursor: pointer;
  white-space: nowrap;
  border-radius: 6px 6px 0 0;
  transition: color 0.15s ease, background 0.15s ease;
}
.workspace-tabs button:hover {
  color: var(--color-text);
  background: var(--color-bg-subtle);
}
.workspace-tabs button.active {
  color: var(--color-accent);
  background: var(--color-surface);
  box-shadow: 0 -1px 0 var(--color-accent) inset;
}
.workspace-tabs button.active:hover {
  background: var(--color-surface);
}
@media (max-width: 640px) {
  .workspace-tabs {
    padding: 0 var(--space-sm);
  }
  .workspace-tabs button {
    padding: 10px 14px;
    font-size: 13px;
  }
}
</style>
