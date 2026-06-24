<template>
  <section class="page">
    <header class="page-head">
      <p>配置状态</p>
      <h1>本地服务</h1>
    </header>
    <ApiErrorState :message="error" />

    <section class="panel provider-panel">
      <h2>模型配置</h2>

      <div class="provider-form">
        <label>
          Provider 类型
          <select v-model="providerType">
            <option value="openai_compatible">OpenAI-compatible</option>
            <option value="mock">Mock（无需 API Key）</option>
          </select>
        </label>

        <template v-if="providerType === 'openai_compatible'">
          <label>
            Base URL
            <input v-model="baseUrl" placeholder="https://api.deepseek.com" />
          </label>

          <label>
            全局默认模型
            <input v-model="modelName" placeholder="deepseek-chat" />
          </label>

          <div class="seat-model-grid" v-if="availableModels.length">
            <label v-for="s in seats" :key="s.key" class="seat-model-row">
              <span>{{ s.label }}</span>
              <select v-model="seatModelMap[s.key]">
                <option value="">使用全局默认</option>
                <option v-for="m in availableModels" :key="m.value" :value="m.value">{{ m.label }}</option>
              </select>
            </label>
          </div>

          <div class="api-key-row">
            <label v-if="!changingKey" class="key-status">
              API Key：<strong>{{ apiKeyConfigured ? '已配置' : '未配置' }}</strong>
              <button class="subtle" @click="changingKey = true">{{ apiKeyConfigured ? '更换' : '配置' }}</button>
            </label>
            <label v-else class="key-input-wrap">
              API Key
              <input
                v-model="apiKeyInput"
                type="password"
                autocomplete="new-password"
                spellcheck="false"
                placeholder="输入新的 API Key"
              />
            </label>
          </div>
        </template>

        <div class="provider-actions">
          <button class="primary" :disabled="saving" @click="saveSettings">
            {{ saving ? '保存中…' : '保存配置' }}
          </button>
          <button v-if="providerType === 'openai_compatible' && baseUrl" :disabled="testing" @click="doTest">
            {{ testing ? '测试中…' : '测试连接' }}
          </button>
          <button v-if="apiKeyConfigured && providerType === 'openai_compatible'" class="danger" @click="clearKey">
            删除 API Key
          </button>
        </div>

        <p v-if="testResult" :class="['test-result', testResult.ok ? 'ok' : 'fail']">
          {{ testResult.message }}
          <span v-if="testResult.latency_ms">（{{ testResult.latency_ms }}ms）</span>
        </p>
      </div>
    </section>

    <dl v-if="config" class="config-grid">
      <dt>Provider</dt>
      <dd>{{ config.provider_kind }}</dd>
      <dt>是否已配置</dt>
      <dd>{{ config.provider_configured ? '是' : '否' }}</dd>
      <dt>当前模型</dt>
      <dd>{{ config.model }}</dd>
      <dt>谋远席模型</dt>
      <dd>{{ config.seat_models?.MOUYUAN || config.model }}</dd>
      <dt>经世席模型</dt>
      <dd>{{ config.seat_models?.JINGSHI || config.model }}</dd>
      <dt>持正席模型</dt>
      <dd>{{ config.seat_models?.CHIZHENG || config.model }}</dd>
      <dt>数据库</dt>
      <dd>{{ config.database_url }}</dd>
      <dt>服务版本</dt>
      <dd>{{ config.version }}</dd>
      <dt>搜索 Provider</dt>
      <dd>{{ config.search_provider || '未配置' }}</dd>
    </dl>

    <form v-if="preferences" class="form-panel preferences-panel" @submit.prevent="savePreferences">
      <div class="row-head">
        <div>
          <h2>用户偏好</h2>
          <p class="muted">只保存本地默认配置，不抽取历史决策，也不保存隐藏推理。</p>
        </div>
        <button class="primary" :disabled="savingPrefs">
          {{ savingPrefs ? '保存中' : '保存偏好' }}
        </button>
      </div>

      <div class="preferences-grid">
        <label>
          默认模式
          <select v-model="preferences.defaults.mode">
            <option value="three_seat">三席合议</option>
            <option value="single_agent">单 Agent</option>
          </select>
        </label>
        <label>
          默认投票策略
          <select v-model="preferences.defaults.vote_strategy">
            <option value="simple_majority">普通多数（2/3）</option>
            <option value="risk_veto">风险否决</option>
            <option value="unanimous">全票通过（3/3）</option>
            <option value="conditional_pass">有条件通过</option>
            <option value="weighted_score">加权评分</option>
          </select>
        </label>
        <label>
          默认视图
          <select v-model="preferences.defaults.view_mode">
            <option value="workbench">工作台</option>
            <option value="report">报告</option>
          </select>
        </label>
        <label>
          代码搜索根目录
          <input v-model="preferences.tools.code_search_root" placeholder="." />
        </label>
        <label>
          文件大小上限（MB）
          <input v-model.number="preferences.tools.max_file_size_mb" type="number" min="1" max="100" />
        </label>
      </div>

      <div class="preferences-toggles">
        <label class="toggle-row">
          <input type="checkbox" v-model="preferences.defaults.search_enabled" />
          <span>新建议题默认启用联网搜索</span>
        </label>
        <label class="toggle-row">
          <input type="checkbox" v-model="preferences.defaults.allow_self_vote" />
          <span>默认允许自投</span>
        </label>
      </div>
    </form>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'
import type { ConfigStatus, UserPreferences, TestProviderResponse } from '../domain/session'

const config = ref<ConfigStatus | null>(null)
const preferences = ref<UserPreferences | null>(null)
const error = ref('')
const saving = ref(false)
const testing = ref(false)
const savingPrefs = ref(false)
const testResult = ref<TestProviderResponse | null>(null)

const providerType = ref('mock')
const baseUrl = ref('')
const modelName = ref('')
const apiKeyConfigured = ref(false)
const apiKeyInput = ref('')
const changingKey = ref(false)

const seatModelMap = ref<Record<string, string>>({})

const seats = [
  { key: 'MOUYUAN', label: '谋远席' },
  { key: 'JINGSHI', label: '经世席' },
  { key: 'CHIZHENG', label: '持正席' },
]

const availableModels = computed(() => config.value?.available_models ?? [])

async function loadSettings() {
  try {
    const settings = await api.getProviderSettings()
    providerType.value = settings.provider
    baseUrl.value = settings.base_url
    modelName.value = settings.model
    apiKeyConfigured.value = settings.api_key_configured
  } catch {
    // settings endpoint unavailable
  }
}

onMounted(async () => {
  try {
    const [configPayload, preferencesPayload] = await Promise.all([
      api.configStatus(),
      api.preferences(),
    ])
    config.value = configPayload
    preferences.value = preferencesPayload
    for (const s of seats) {
      seatModelMap.value[s.key] = configPayload.seat_models?.[s.key] || ''
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载失败'
  }
  await loadSettings()
})

async function saveSettings() {
  saving.value = true
  error.value = ''
  testResult.value = null
  try {
    const result = await api.updateProviderSettings({
      provider: providerType.value,
      base_url: baseUrl.value,
      model: modelName.value,
      api_key: apiKeyInput.value || undefined,
    })
    apiKeyConfigured.value = result.api_key_configured
    apiKeyInput.value = ''
    changingKey.value = false
  } catch (err) {
    error.value = err instanceof Error ? err.message : '保存失败'
  } finally {
    saving.value = false
  }
}

async function doTest() {
  testing.value = true
  testResult.value = null
  try {
    testResult.value = await api.testProvider({
      provider: providerType.value,
      base_url: baseUrl.value,
      model: modelName.value,
      api_key: apiKeyInput.value || undefined,
      use_saved_key: !apiKeyInput.value && apiKeyConfigured.value,
    })
  } catch (err) {
    testResult.value = { ok: false, message: err instanceof Error ? err.message : '测试失败' }
  } finally {
    testing.value = false
  }
}

async function clearKey() {
  try {
    await api.updateProviderSettings({
      provider: providerType.value,
      base_url: baseUrl.value,
      model: modelName.value,
      clear_api_key: true,
    })
    apiKeyConfigured.value = false
    apiKeyInput.value = ''
  } catch (err) {
    error.value = err instanceof Error ? err.message : '删除失败'
  }
}

async function savePreferences() {
  if (!preferences.value) return
  savingPrefs.value = true
  error.value = ''
  try {
    preferences.value = await api.updatePreferences(preferences.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '保存失败'
  } finally {
    savingPrefs.value = false
  }
}
</script>

<style scoped>
.provider-panel {
  margin-bottom: 18px;
}

.provider-form {
  display: grid;
  gap: 14px;
  margin-top: 12px;
}

.provider-form label {
  display: grid;
  gap: 4px;
  font-size: 13px;
  color: #c8d0d8;
}

.provider-form input,
.provider-form select {
  padding: 8px 10px;
  border: 1px solid rgba(212, 226, 236, 0.25);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.07);
  color: #f1f5f9;
  font-size: 14px;
}

.provider-form input:focus,
.provider-form select:focus {
  outline: none;
  border-color: #0f8aa1;
  background: rgba(255, 255, 255, 0.1);
}

.provider-form input::placeholder {
  color: #6b7a85;
}

.seat-model-grid {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  background: rgba(0, 0, 0, 0.12);
  border-radius: 6px;
}

.seat-model-row {
  display: grid;
  grid-template-columns: 60px 1fr;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: #c8d0d8;
}

.seat-model-row select {
  padding: 6px 8px;
  border: 1px solid rgba(212, 226, 236, 0.2);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  color: #f1f5f9;
  font-size: 13px;
}

.api-key-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.key-status strong {
  color: #e2e8f0;
  font-family: monospace;
  margin: 0 6px;
}

.key-input-wrap {
  flex: 1;
}

.provider-actions {
  display: flex;
  gap: 10px;
  margin-top: 4px;
}

.danger {
  background: rgba(182, 38, 98, 0.3);
  border: 1px solid rgba(182, 38, 98, 0.5);
  color: #ffd7e6;
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.danger:hover {
  background: rgba(182, 38, 98, 0.5);
}

.test-result {
  margin: 0;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
}

.test-result.ok {
  background: rgba(15, 138, 161, 0.15);
  color: #8ddbd1;
}

.test-result.fail {
  background: rgba(182, 38, 98, 0.15);
  color: #ff8ab5;
}

.preferences-panel {
  display: grid;
  gap: 18px;
  margin-top: 18px;
}

.preferences-panel h2 {
  margin: 0 0 4px;
}

.preferences-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 14px;
}

.preferences-toggles {
  display: grid;
  gap: 10px;
}

.toggle-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toggle-row input {
  width: 18px;
  height: 18px;
}
</style>
