<template>
  <section class="panel decision">
    <header class="section-head">
      <h2>表决结果</h2>
      <span :class="['badge', decision?.status === 'majority_reached' ? 'ok' : 'warn']">
        {{ decision?.status === 'majority_reached' ? '形成多数' : '未形成多数' }}
      </span>
    </header>
    <div v-if="decision?.selected_proposal" class="result-block">
      <h3>{{ decision.selected_proposal.title }}</h3>
      <p>{{ decision.selected_proposal.summary }}</p>
      <dl>
        <dt>投票数</dt>
        <dd>{{ decision.vote_count }}</dd>
        <dt>自投记录</dt>
        <dd>{{ decision.self_vote_count }}</dd>
      </dl>
    </div>
    <div class="columns">
      <div>
        <h3>多数理由</h3>
        <p v-if="!decision?.majority_reasons?.length">暂无</p>
        <ul>
          <li v-for="reason in decision?.majority_reasons" :key="reason">{{ reason }}</li>
        </ul>
      </div>
      <div>
        <h3>少数留议</h3>
        <p v-if="!decision?.minority_opinion?.length">暂无</p>
        <ul>
          <li v-for="opinion in decision?.minority_opinion" :key="opinion">{{ opinion }}</li>
        </ul>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import type { Decision } from '../domain/session'

defineProps<{
  decision?: Decision | null
}>()
</script>
