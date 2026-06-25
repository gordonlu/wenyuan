<template>
  <section class="panel">
    <div class="row-head">
      <h2>新事实复议</h2>
      <span v-if="running" class="badge warn">进行中</span>
    </div>

    <p class="muted" style="margin-bottom: 12px">
      发现新事实时，可在此提交重新评估对已有结论的影响。
    </p>

    <div v-if="!running">
      <textarea
        v-model="newFact"
        rows="3"
        placeholder="输入新事实或新信息…"
        style="width: 100%"
        :disabled="submitting"
      />
      <div v-if="objects.length > 0" style="margin: 8px 0">
        <p class="muted" style="margin-bottom: 4px; font-size: 13px">影响以下决策对象（可选）：</p>
        <div class="object-select-grid">
          <label
            v-for="obj in objects"
            :key="obj.id"
            :class="['object-check', { selected: selectedIds.has(obj.id) }]"
          >
            <input type="checkbox" :value="obj.id" v-model="selectedArray" />
            <span :class="['badge', kindBadge(obj.kind)]">{{ decisionObjectKindLabels[obj.kind] }}</span>
            <span>{{ obj.title }}</span>
          </label>
        </div>
      </div>
      <div class="actions">
        <button :disabled="!newFact.trim() || submitting" @click="submit">
          {{ submitting ? '提交中…' : '提交复议' }}
        </button>
      </div>
    </div>

    <div v-else class="re-delib-loading">
      <p>正在执行新事实复议，请稍候…</p>
    </div>

    <div v-if="result" class="re-delib-result">
      <h3>复议结果</h3>
      <pre>{{ resultText }}</pre>
    </div>

    <div v-if="errorMessage" class="error-state">{{ errorMessage }}</div>
  </section>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { decisionObjectKindLabels, type DecisionObject } from '../domain/session'

const props = defineProps<{
  objects: DecisionObject[]
  running: boolean
  result: unknown
  errorMessage: string
}>()

const emit = defineEmits<{
  submit: [payload: { new_fact: string; affected_object_ids: string[] }]
  clear: []
}>()

const newFact = ref('')
const selectedArray = ref<string[]>([])
const selectedIds = computed(() => new Set(selectedArray.value))
const submitting = ref(false)

const resultText = computed(() => {
  if (!props.result) return ''
  try { return JSON.stringify(props.result, null, 2) } catch { return String(props.result) }
})

function submit() {
  if (!newFact.value.trim()) return
  submitting.value = true
  emit('submit', {
    new_fact: newFact.value.trim(),
    affected_object_ids: selectedArray.value,
  })
}

function kindBadge(kind: string) {
  if (kind === 'risk' || kind === 'minority_concern') return 'warn'
  if (kind === 'opportunity') return 'ok'
  return ''
}

defineExpose({ submitting })
</script>

<style scoped>
textarea { font-size: 13px; }
.object-select-grid { display: flex; flex-direction: column; gap: 4px; max-height: 200px; overflow-y: auto; }
.object-check { display: flex; align-items: center; gap: 6px; padding: 4px 8px; border: 1px solid transparent; border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
.object-check:hover { background: var(--color-bg-subtle); }
.object-check.selected { border-color: var(--color-accent); background: var(--color-accent-light); }
.object-check input { margin: 0; }
.re-delib-loading { padding: 20px 0; text-align: center; color: var(--color-text-dim); }
.re-delib-result { margin-top: 12px; }
.re-delib-result pre { margin-top: 6px; padding: 10px; background: var(--color-bg-subtle); border-radius: var(--radius-sm); font-size: 12px; overflow-x: auto; white-space: pre-wrap; max-height: 300px; overflow-y: auto; font-family: var(--font-mono); }
.error-state { margin-top: 8px; color: var(--color-danger); font-size: 13px; }
</style>
