<template>
  <section class="page">
    <header class="page-head">
      <p>历史议题</p>
      <h1>已创建的合议</h1>
    </header>
    <ApiErrorState :message="store.error" />
    <div class="list">
      <RouterLink v-for="session in store.sessions" :key="session.id" class="list-row" :to="`/sessions/${session.id}`">
        <strong>{{ session.title }}</strong>
        <span>{{ phaseLabels[session.phase] }}</span>
        <span>{{ session.has_majority ? '形成多数' : '未形成多数' }}</span>
        <time>{{ new Date(session.created_at).toLocaleString() }}</time>
      </RouterLink>
    </div>
  </section>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import ApiErrorState from '../components/ApiErrorState.vue'
import { phaseLabels } from '../domain/session'
import { useSessionStore } from '../stores/sessionStore'

const store = useSessionStore()
onMounted(() => store.loadHistory())
</script>
