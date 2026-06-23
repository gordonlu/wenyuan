import { reactive, readonly } from 'vue'

interface ConfirmState {
  visible: boolean
  message: string
  resolve: ((value: boolean) => void) | null
}

const state = reactive<ConfirmState>({
  visible: false,
  message: '',
  resolve: null,
})

export function useConfirm() {
  function confirm(message: string): Promise<boolean> {
    state.message = message
    state.visible = true
    return new Promise<boolean>((resolve) => {
      state.resolve = resolve
    })
  }

  function confirmResolve(value: boolean) {
    state.visible = false
    state.resolve?.(value)
    state.resolve = null
  }

  return {
    confirmState: readonly(state),
    confirm,
    confirmResolve,
  }
}
