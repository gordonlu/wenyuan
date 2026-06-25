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

          <div class="api-key-row">
            <label v-if="!changingKey" class="key-status">
              API Key：<strong>{{ apiKeyConfigured ? '已配置' : '未配置' }}</strong>
              <span v-if="apiKeySource" class="key-source">{{ apiKeySource === 'env' ? '（环境变量）' : '（页面配置）' }}</span>
              <button class="subtle" @click="changingKey = true">{{ apiKeyConfigured ? '更换' : '配置' }}</button>
            </label>
            <label v-else class="key-input-wrap">
              API Key
              <input v-model="apiKeyInput" type="password" autocomplete="new-password" spellcheck="false" placeholder="输入新的 API Key" />
            </label>
          </div>

          <div class="model-list-area">
            <div class="model-list-header">
              <span>可用模型</span>
              <button
                v-if="baseUrl"
                :disabled="fetchingModels"
                class="subtle"
                @click="fetchModels"
              >{{ fetchingModels ? '获取中…' : '获取列表' }}</button>
            </div>
            <div v-if="availableModels.length" class="model-chips">
              <span v-for="m in availableModels" :key="m.value" class="model-chip">{{ m.label }}</span>
            </div>
            <p v-else class="model-list-empty">未获取模型列表，可手动填写</p>
          </div>

          <div class="seat-provider-grid">
            <div class="seat-provider-header">各议席配置</div>
            <div v-for="s in seats" :key="s.key" class="seat-provider-card">
              <div class="seat-provider-title">{{ s.label }}</div>
              <label>模型
                <select v-model="seatModelMap[s.key]">
                  <option value="">使用全局默认</option>
                  <option v-for="m in availableModels" :key="m.value" :value="m.value">{{ m.label || m.value }}</option>
                </select>
              </label>
              <label>Base URL
                <input v-model="seatBaseUrlMap[s.key]" placeholder="使用全局默认" />
              </label>
              <label>
                API Key
                <input v-model="seatApiKeyMap[s.key]" type="password" autocomplete="new-password" placeholder="使用全局默认" />
                <span v-if="seatApiKeyConfiguredMap[s.key]" class="key-source">（已配置）</span>
              </label>
            </div>
          </div>
        </template>

        <div class="search-section">
          <div class="search-header">联网搜索</div>
          <label>
            搜索 Provider
            <select v-model="searchProvider">
              <option value="">禁用</option>
              <option value="doubao">Doubao</option>
              <option value="tavily">Tavily</option>
              <option value="google">Google</option>
              <option value="searxng">SearXNG</option>
              <option value="custom">Custom</option>
            </select>
            <span v-if="searchProvider && searchProvider === envSearchProvider" class="search-source-badge">环境变量</span>
            <span v-else-if="searchProvider && searchProvider !== envSearchProvider" class="search-source-badge page-badge">页面配置</span>
          </label>
          <label>
            API Key / URL
            <input v-model="searchApiUrl" :placeholder="searchProvider === 'searxng' ? 'https://searxng.example.com' : 'API Key'" />
          </label>
        </div>

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
import { useConfirm } from '../composables/useConfirm'
import type { ConfigStatus, UserPreferences, TestProviderResponse } from '../domain/session'

const { confirm } = useConfirm()

const config = ref<ConfigStatus | null>(null)
const preferences = ref<UserPreferences | null>(null)
const error = ref('')
const saving = ref(false)
const testing = ref(false)
const fetchingModels = ref(false)
const savingPrefs = ref(false)
const testResult = ref<TestProviderResponse | null>(null)

const providerType = ref('mock')
const baseUrl = ref('')
const modelName = ref('')
const apiKeyConfigured = ref(false)
const apiKeySource = ref('')
const apiKeyInput = ref('')
const changingKey = ref(false)
const searchProvider = ref('')
const searchApiUrl = ref('')
const envSearchProvider = ref('')

const seatModelMap = ref<Record<string, string>>({})
const seatBaseUrlMap = ref<Record<string, string>>({})
const seatApiKeyMap = ref<Record<string, string>>({})
const seatApiKeyConfiguredMap = ref<Record<string, boolean>>({})

const seats = [
  { key: 'MOUYUAN', label: '谋远席' },
  { key: 'JINGSHI', label: '经世席' },
  { key: 'CHIZHENG', label: '持正席' },
]

const availableModels = computed(() => {
  if (config.value?.available_models?.length) return config.value.available_models
  // Fallback: build from configured models
  const seen = new Set<string>()
  const models: Array<{ value: string; label: string }> = []
  const add = (name: string) => {
    if (name && !seen.has(name)) { seen.add(name); models.push({ value: name, label: name }) }
  }
  add(config.value?.model ?? '')
  for (const m of Object.values(config.value?.seat_models ?? {})) add(m)
  return models
})

async function loadSettings() {
  try {
    const settings = await api.getProviderSettings()
    providerType.value = settings.provider || (config.value?.provider_configured ? 'openai_compatible' : 'mock')
    baseUrl.value = settings.base_url || config.value?.base_url || ''
    modelName.value = settings.model || config.value?.model || ''
    apiKeyConfigured.value = settings.api_key_configured
    apiKeySource.value = settings.api_key_source || ''
    envSearchProvider.value = config.value?.search_provider || ''
    searchProvider.value = settings.search_provider || envSearchProvider.value || ''
    searchApiUrl.value = settings.search_api_url || ''
    for (const s of seats) {
      const sp = settings.seat_providers?.[s.key]
      seatBaseUrlMap.value[s.key] = sp?.base_url || ''
      seatApiKeyMap.value[s.key] = ''
      seatApiKeyConfiguredMap.value[s.key] = sp?.api_key_configured || false
    }
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

async function fetchModels() {
  if (!baseUrl.value) return
  fetchingModels.value = true
  const key = apiKeyInput.value || undefined
  try {
    const resp = await fetch(`${baseUrl.value.replace(/\/+$/, '')}/models`, {
      headers: { Authorization: key ? `Bearer ${key}` : '' },
    })
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
    const data = await resp.json()
    const models: Array<{ id: string }> = data.data ?? data
    // Update the config.available_models reactively
    if (config.value) {
      config.value.available_models = models.map((m: { id: string }) => ({ value: m.id, label: m.id }))
    }
  } catch (err) {
    error.value = `获取模型列表失败：${err instanceof Error ? err.message : '未知错误'}`
  } finally {
    fetchingModels.value = false
  }
}

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
      search_provider: searchProvider.value || undefined,
      search_api_url: searchApiUrl.value || undefined,
      seat_providers: Object.fromEntries(seats.map(s => [s.key, {
        base_url: seatBaseUrlMap.value[s.key] || '',
        api_key: seatApiKeyMap.value[s.key] || '',
      }])),
    })
    apiKeyConfigured.value = result.api_key_configured
    apiKeyInput.value = ''
    changingKey.value = false
    if (result.restart_required) {
      error.value = '配置已保存，重启后生效。当前进行中的合议不会受影响。'
    }
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
  if (!(await confirm('确认删除 API Key？'))) return
  try {
    const result = await api.updateProviderSettings({
      provider: providerType.value,
      base_url: baseUrl.value,
      model: modelName.value,
      clear_api_key: true,
    })
    apiKeyConfigured.value = false
    apiKeyInput.value = ''
    if (result.restart_required) {
      error.value = '配置已保存，重启后生效。当前进行中的合议不会受影响。'
    }
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
  color: #475569;
}

.model-list-area {
  display: grid;
  gap: 6px;
}

.model-list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  color: #334155;
}

.model-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.model-chip {
  padding: 3px 8px;
  background: rgba(15, 138, 161, 0.12);
  border: 1px solid rgba(15, 138, 161, 0.2);
  border-radius: 4px;
  color: #0f766e;
  font-size: 12px;
  font-family: monospace;
}

.seat-provider-grid {
  display: grid;
  gap: 10px;
  padding: 4px 0;
}

.seat-provider-header {
  font-size: 13px;
  font-weight: 600;
  color: #334155;
}

.seat-provider-card {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 6px;
  border: 1px solid rgba(0, 0, 0, 0.06);
}

.seat-provider-title {
  font-size: 13px;
  font-weight: 600;
  color: #1e293b;
}

.seat-provider-card label {
  display: grid;
  gap: 2px;
  font-size: 12px;
  color: #475569;
}

.seat-model-grid {
  display: grid;
  gap: 8px;
  padding: 4px 0;
}

.model-list-empty {
  font-size: 13px;
  color: #64748b;
}

.seat-model-row {
  display: grid;
  grid-template-columns: 60px 1fr;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: #c8d0d8;
}

.api-key-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.key-status strong {
  color: #1e293b;
  font-family: monospace;
  margin: 0 6px;
}

.key-source {
  font-size: 11px;
  color: #64748b;
  margin-right: 6px;
}

.search-section {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  background: rgba(15, 138, 161, 0.06);
  border-radius: 6px;
}

.search-header {
  font-size: 13px;
  font-weight: 600;
  color: #334155;
}

.search-source-badge {
  display: inline-block;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  margin-left: 6px;
  background: rgba(15, 138, 161, 0.15);
  color: #0f766e;
}
.search-source-badge.page-badge {
  background: rgba(182, 38, 98, 0.12);
  color: #be185d;
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
  background: rgba(190, 24, 93, 0.15);
  border: 1px solid rgba(190, 24, 93, 0.4);
  color: #be185d;
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.danger:hover {
  background: rgba(190, 24, 93, 0.25);
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
