<template>
  <section class="page">
    <header class="page-head">
      <p>配置状态</p>
      <h1>本地服务</h1>
    </header>
    <ApiErrorState :message="error" />
    <dl v-if="config" class="config-grid">
      <dt>Provider</dt>
      <dd>{{ config.provider_kind }}</dd>
      <dt>是否已配置</dt>
      <dd>{{ config.provider_configured ? '是' : '否' }}</dd>
      <dt>当前模型</dt>
      <dd>{{ config.model }}</dd>
      <dt>数据库</dt>
      <dd>{{ config.database_url }}</dd>
      <dt>服务版本</dt>
      <dd>{{ config.version }}</dd>
    </dl>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'
import type { ConfigStatus } from '../domain/session'

const config = ref<ConfigStatus | null>(null)
const error = ref('')

onMounted(async () => {
  try {
    config.value = await api.configStatus()
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载失败'
  }
})
</script>
