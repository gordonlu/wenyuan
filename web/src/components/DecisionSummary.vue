<template>
  <section class="panel decision">
    <header class="section-head">
      <h2>表决结果</h2>
      <div class="phase-labels">
        <span v-if="decision?.has_risk_blocker" class="badge danger">风险阻塞</span>
        <span :class="['badge', decision?.status === 'majority_reached' ? 'ok' : 'warn']">
          {{ decision?.status === 'majority_reached' ? '形成多数' : '未形成多数' }}
        </span>
      </div>
    </header>
    <div v-if="decision?.selected_proposal" class="result-block">
      <h3>{{ decision.selected_proposal.title }}</h3>
      <p>{{ decision.selected_proposal.summary }}</p>
      <dl>
        <dt>有效票数</dt>
        <dd>{{ decision.vote_count }}</dd>
        <dt>自投记录</dt>
        <dd>{{ decision.self_vote_count }}</dd>
      </dl>
    </div>
    <div v-if="decision?.adoption_conditions?.length" class="result-block">
      <h3>采纳条件</h3>
      <ul>
        <li v-for="item in decision.adoption_conditions" :key="item">{{ item }}</li>
      </ul>
    </div>
    <div class="columns">
      <div>
        <h3>多数理由</h3>
        <p v-if="!decision?.majority_reasons?.length" class="muted">暂无</p>
        <ul>
          <li v-for="reason in decision?.majority_reasons" :key="reason">{{ reason }}</li>
        </ul>
      </div>
      <div>
        <h3>少数留议</h3>
        <p v-if="!decision?.minority_opinion?.length" class="muted">暂无</p>
        <ul>
          <li v-for="opinion in decision?.minority_opinion" :key="opinion">{{ opinion }}</li>
        </ul>
      </div>
    </div>
    <div v-if="decision?.minority_choices?.length" class="minority-detail">
      <h3>少数方详情</h3>
      <div v-for="choice in decision.minority_choices" :key="choice.seat" class="minority-card">
        <p>
          <strong>{{ seatLabels[choice.seat] }}</strong>：{{ choice.reason }}
          <span v-if="choice.has_risk_warning" class="badge danger" style="margin-left: 8px">含风险提醒</span>
        </p>
        <p v-if="choice.reassessment_condition" class="muted">重新评估条件：{{ choice.reassessment_condition }}</p>
      </div>
    </div>
    <div v-if="decision?.reassessment_conditions?.length" class="result-block">
      <h3>重新评估条件</h3>
      <ul>
        <li v-for="condition in decision.reassessment_conditions" :key="condition">{{ condition }}</li>
      </ul>
    </div>
    <div v-if="decision?.unresolved_questions?.length" class="result-block">
      <h3>未决问题</h3>
      <ul>
        <li v-for="q in decision.unresolved_questions" :key="q">{{ q }}</li>
      </ul>
    </div>
    <div v-if="decision?.next_steps?.length" class="result-block">
      <h3>下一步</h3>
      <ul>
        <li v-for="step in decision.next_steps" :key="step">{{ step }}</li>
      </ul>
    </div>
  </section>
</template>

<script setup lang="ts">
import { seatLabels } from '../domain/session'
import type { Decision } from '../domain/session'

defineProps<{
  decision?: Decision | null
}>()
</script>
