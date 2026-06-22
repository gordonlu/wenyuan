<template>
  <div class="search-test">
    <header class="page-head">
      <h1>搜索测试</h1>
      <button class="icon" title="返回" @click="$router.push('/')"><ArrowLeft :size="18" /></button>
    </header>

    <section class="panel input-panel">
      <textarea v-model="topic" class="input" rows="3" placeholder="输入议题文字…" />
      <button class="primary" :disabled="loading || !topic.trim()" @click="run">
        {{ loading ? '搜索中…' : 'LLM 拆词 → 搜索' }}
      </button>
    </section>

    <section v-if="error" class="panel">
      <p class="error">{{ error }}</p>
    </section>

    <template v-if="result">
      <section v-if="result.keyword_step" class="panel step-panel">
        <div class="step-head">
          <span class="step-num">1</span>
          <h2>LLM 提取关键词</h2>
        </div>
        <div class="step-body">
          <label>Prompt</label>
          <pre class="code-block">{{ result.keyword_step.prompt ?? '' }}</pre>

          <label>Raw Response</label>
          <pre class="code-block">{{ result.keyword_step.raw_response ?? '' }}</pre>

          <label>提取结果</label>
          <div class="extracted-row">
            <code>{{ result.keyword_step.extracted ?? '' }}</code>
            <span class="step-note">{{ result.keyword_step.note ?? '' }}</span>
          </div>
        </div>
      </section>

      <section class="panel step-panel">
        <div class="step-head">
          <span class="step-num">2</span>
          <h2>搜索</h2>
        </div>
        <div class="step-body">
          <label>搜索查询</label>
          <pre class="code-block">{{ result.search_query ?? '' }}</pre>

          <div v-for="b in (result.backends ?? [])" :key="b.name" class="backend-block">
            <div class="backend-head">
              <span class="backend-name">{{ b.name }}</span>
              <span v-if="b.error" class="badge warn">{{ b.error }}</span>
              <span v-else class="badge ok">{{ b.results.length }} 条</span>
            </div>
            <div v-if="b.results.length" class="result-list">
              <article v-for="(item, i) in b.results" :key="i" class="result-item">
                <a :href="item.url" target="_blank" rel="noopener" class="result-title">{{ item.title ?? '' }}</a>
                <p class="result-snippet">{{ item.snippet ?? '' }}</p>
                <p class="result-meta">{{ item.url ?? '' }} · {{ item.source ?? '' }}</p>
              </article>
            </div>
            <p v-else-if="!b.error" class="muted">无结果</p>
          </div>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api'
import { ArrowLeft } from '@lucide/vue'
import type { SearchTestResponse } from '../domain/session'

const topic = ref('')
const loading = ref(false)
const error = ref('')
const result = ref<SearchTestResponse | null>(null)

async function run() {
  loading.value = true
  error.value = ''
  result.value = null
  try {
    result.value = await api.searchTest({ topic: topic.value })
  } catch (err) {
    error.value = String(err)
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.search-test {
  max-width: 760px;
  margin: 0 auto;
  padding: 24px;
}
.page-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}
.page-head h1 {
  margin: 0;
}
.input-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
textarea.input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 14px;
  line-height: 1.5;
  resize: vertical;
  font-family: inherit;
}
.step-panel {
  margin-top: 16px;
}
.step-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}
.step-head h2 {
  margin: 0;
  font-size: 16px;
}
.step-num {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: var(--color-accent);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  font-weight: 700;
  flex-shrink: 0;
}
.step-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.step-body label {
  font-size: 12px;
  font-weight: 700;
  color: var(--color-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.code-block {
  background: var(--color-surface-dim);
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  line-height: 1.5;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-word;
  margin: 0;
}
.extracted-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  background: #f0f7e8;
  border: 1px solid #cde0b8;
  border-radius: var(--radius-sm);
}
.extracted-row code {
  font-size: 14px;
  font-weight: 600;
}
.step-note {
  font-size: 12px;
  color: var(--color-text-muted);
}
.result-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.result-item {
  padding: 12px;
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-sm);
}
.result-title {
  font-weight: 600;
  font-size: 14px;
  color: var(--color-accent);
  text-decoration: none;
}
.result-title:hover {
  text-decoration: underline;
}
.result-snippet {
  font-size: 13px;
  color: var(--color-text);
  margin: 4px 0;
  line-height: 1.5;
}
.result-meta {
  font-size: 11px;
  color: var(--color-text-dim);
}
.error {
  color: var(--color-danger);
}
.backend-block {
  margin-top: 8px;
}
.backend-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.backend-name {
  font-weight: 700;
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
</style>
