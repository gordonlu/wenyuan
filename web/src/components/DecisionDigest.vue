<template>
  <section class="decision-digest" :class="`digest-${digest.status_class}`">
    <div class="digest-top">
      <div>
        <span v-if="digest.has_decision" :class="['badge', digest.status_class === 'ok' ? 'ok' : 'warn']">
          {{ digest.status_label }}
        </span>
        <span v-else class="badge warn">{{ digest.status_label }}</span>
      </div>
      <span v-if="digest.has_decision && digest.vote_count > 0" class="digest-vote-count">
        {{ digest.vote_count }} 票有效
      </span>
    </div>

    <div v-if="digest.has_decision" class="digest-body">
      <div v-if="digest.selected_proposal_title" class="digest-section">
        <span class="digest-label">多数策案</span>
        <p class="digest-value">{{ digest.selected_proposal_title }}</p>
      </div>

      <div v-if="digest.majority_reason_summary" class="digest-section">
        <span class="digest-label">主要依据</span>
        <p class="digest-value">{{ digest.majority_reason_summary }}</p>
      </div>

      <div v-if="digest.minority_summary" class="digest-section">
        <span class="digest-label">少数意见</span>
        <p class="digest-value">{{ digest.minority_summary }}</p>
      </div>

      <div v-if="digest.next_step_summary" class="digest-section">
        <span class="digest-label">下一步</span>
        <p class="digest-value">{{ digest.next_step_summary }}</p>
      </div>

      <div class="digest-flags">
        <span v-if="digest.has_risk_blocker" class="digest-flag digest-flag-warn">
          存在风险阻塞
        </span>
        <span v-if="digest.has_untrusted_external" class="digest-flag digest-flag-warn">
          含不可信外部来源
        </span>
        <span v-if="digest.has_injection_risk" class="digest-flag digest-flag-danger">
          检测到疑似注入
        </span>
        <span v-if="digest.has_unverified_evidence" class="digest-flag digest-flag-info">
          {{ digest.unverified_claims }} 项主张未验证
        </span>
      </div>
    </div>

    <div v-else class="digest-empty">
      <p>讨论进行中，结论将在合议完成后生成</p>
    </div>
  </section>
</template>

<script setup lang="ts">
import type { DecisionDigest } from '../domain/session'

defineProps<{
  digest: DecisionDigest
}>()
</script>

<style scoped>
.decision-digest {
  padding: 16px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.digest-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.digest-vote-count {
  font-size: 12px;
  color: var(--color-text-muted);
  white-space: nowrap;
}

.digest-body {
  display: grid;
  gap: 10px;
}

.digest-section {
  display: grid;
  gap: 2px;
}

.digest-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--color-text-dim);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.digest-value {
  margin: 0;
  font-size: 14px;
  line-height: 1.5;
  color: var(--color-text);
}

.digest-flags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 4px;
}

.digest-flag {
  padding: 3px 8px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  font-weight: 600;
  line-height: 1.4;
}

.digest-flag-warn {
  background: var(--color-warning-bg);
  color: var(--color-warning-text);
  border: 1px solid var(--color-warning-border);
}

.digest-flag-danger {
  background: var(--color-danger-light);
  color: var(--color-danger);
  border: 1px solid rgba(154, 63, 52, 0.2);
}

.digest-flag-info {
  background: var(--color-accent-light);
  color: var(--color-accent-text);
  border: 1px solid rgba(15, 138, 161, 0.2);
}

.digest-empty p {
  margin: 0;
  font-size: 13px;
  color: var(--color-text-muted);
}

/* status variant overrides */
.digest-ok {
  border-left: 6px solid var(--color-success);
}

.digest-warn {
  border-left: 6px solid var(--color-warning-border);
}

.digest-danger {
  border-left: 6px solid var(--color-danger);
}
</style>
