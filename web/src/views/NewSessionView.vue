<template>
  <section class="page">
    <header class="page-head">
      <p>新建议题</p>
      <h1>三席独议，交相批议</h1>
    </header>
    <form class="form-panel" @submit.prevent="submit">
      <label>
        标题
        <input v-model="title" required placeholder="是否为桌面助手加入多 Agent 讨论能力" />
      </label>
      <label>
        议题正文
        <textarea v-model="topic" required rows="7" placeholder="写下完整议题、目标和需要权衡的问题" />
      </label>
      <label>
        补充背景
        <textarea v-model="context" rows="5" placeholder="可为空" />
      </label>
      <ApiErrorState :message="error" />
      <button class="primary" :disabled="loading">
        <Send :size="18" />
        创建并开议
      </button>
    </form>
  </section>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { Send } from '@lucide/vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'

const router = useRouter()
const title = ref('')
const topic = ref('')
const context = ref('')
const loading = ref(false)
const error = ref('')

async function submit() {
  loading.value = true
  error.value = ''
  try {
    const session = await api.createSession({ title: title.value, topic: topic.value, context: context.value })
    await api.startSession(session.id)
    router.push(`/sessions/${session.id}`)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '创建失败'
  } finally {
    loading.value = false
  }
}
</script>
