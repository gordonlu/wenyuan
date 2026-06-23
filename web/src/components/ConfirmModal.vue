<template>
  <Teleport to="body">
    <div v-if="confirmState.visible" class="confirm-overlay" @click.self="cancel">
      <div class="confirm-modal" role="alertdialog" aria-modal="true">
        <p class="confirm-message">{{ confirmState.message }}</p>
        <div class="confirm-actions">
          <button class="btn-cancel" @click="cancel">取消</button>
          <button class="btn-confirm" @click="confirm">确认</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { useConfirm } from '../composables/useConfirm'

const { confirmState, confirmResolve } = useConfirm()

function confirm() {
  confirmResolve(true)
}

function cancel() {
  confirmResolve(false)
}
</script>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(7, 12, 18, 0.54);
  backdrop-filter: blur(3px);
  animation: overlay-in 160ms ease-out;
}

.confirm-modal {
  width: 380px;
  max-width: 90vw;
  padding: var(--space-lg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  box-shadow: var(--shadow-lg);
  animation: modal-in 200ms ease-out;
}

.confirm-message {
  margin: 0 0 var(--space-lg);
  font-size: 15px;
  line-height: 1.6;
  color: var(--color-text);
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-sm);
}

.btn-cancel,
.btn-confirm {
  min-width: 72px;
  height: 34px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background var(--transition-fast), box-shadow var(--transition-fast);
}

.btn-cancel {
  border: 1px solid var(--color-border);
  background: transparent;
  color: var(--color-text-muted);
}
.btn-cancel:hover {
  background: var(--color-surface-hover);
  color: var(--color-text);
}

.btn-confirm {
  border: 0;
  background: var(--color-accent);
  color: #ffffff;
  box-shadow: 0 1px 3px rgba(15, 138, 161, 0.28);
}
.btn-confirm:hover {
  background: var(--color-accent-hover);
  box-shadow: 0 2px 6px rgba(15, 138, 161, 0.34);
}

@keyframes overlay-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes modal-in {
  from { opacity: 0; transform: scale(0.96) translateY(8px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
</style>
