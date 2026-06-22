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
      <dd>
        {{ config.search_provider || '未配置' }}
        <p v-if="config.search_provider?.includes('bing') || config.search_provider?.includes('duckduckgo')" class="search-warning">
          ⚠ bing/duckduckgo 仅供本地实验。生产环境请配置 authorized search provider（custom / tavily / doubao / google / searxng）。
        </p>
      </dd>
    </dl>

    <form v-if="preferences" class="form-panel preferences-panel" @submit.prevent="savePreferences">
      <div class="row-head">
        <div>
          <h2>用户偏好</h2>
          <p class="muted">只保存本地默认配置，不抽取历史决策，也不保存隐藏推理。</p>
        </div>
        <button class="primary" :disabled="saving">
          {{ saving ? '保存中' : '保存偏好' }}
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

      <div class="seat-model-preferences">
        <label>
          谋远席默认模型
          <input v-model="preferences.models.mouyuan" placeholder="留空使用服务默认模型" />
        </label>
        <label>
          经世席默认模型
          <input v-model="preferences.models.jingshi" placeholder="留空使用服务默认模型" />
        </label>
        <label>
          持正席默认模型
          <input v-model="preferences.models.chizheng" placeholder="留空使用服务默认模型" />
        </label>
      </div>
    </form>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api'
import ApiErrorState from '../components/ApiErrorState.vue'
import type { ConfigStatus, UserPreferences } from '../domain/session'

const config = ref<ConfigStatus | null>(null)
const preferences = ref<UserPreferences | null>(null)
const error = ref('')
const saving = ref(false)

onMounted(async () => {
  try {
    const [configPayload, preferencesPayload] = await Promise.all([
      api.configStatus(),
      api.preferences(),
    ])
    config.value = configPayload
    preferences.value = preferencesPayload
  } catch (err) {
    error.value = err instanceof Error ? err.message : '加载失败'
  }
})

async function savePreferences() {
  if (!preferences.value) return
  saving.value = true
  error.value = ''
  try {
    preferences.value = await api.updatePreferences(preferences.value)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '保存失败'
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.preferences-panel {
  display: grid;
  gap: 18px;
  margin-top: 18px;
}

.preferences-panel h2 {
  margin: 0 0 4px;
}

.preferences-grid,
.seat-model-preferences {
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
.search-warning {
  font-size: 12px;
  color: var(--color-warning, #b8860b);
  margin: 4px 0 0;
  line-height: 1.4;
}
</style>
