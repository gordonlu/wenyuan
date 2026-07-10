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
  gap: 0;
  border-bottom: 2px solid var(--color-border-light);
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
  padding: 0 var(--space-md);
}

.workspace-tabs button {
  padding: 10px 18px;
  border: none;
  background: none;
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-muted);
  cursor: pointer;
  white-space: nowrap;
  border-bottom: 2px solid transparent;
  margin-bottom: -2px;
  transition: color 0.15s ease, border-color 0.15s ease;
}

.workspace-tabs button:hover {
  color: var(--color-text);
}

.workspace-tabs button.active {
  color: var(--color-accent);
  border-bottom-color: var(--color-accent);
}

@media (max-width: 640px) {
  .workspace-tabs {
    padding: 0 var(--space-sm);
  }
  .workspace-tabs button {
    padding: 10px 12px;
    font-size: 13px;
  }
}
</style>
