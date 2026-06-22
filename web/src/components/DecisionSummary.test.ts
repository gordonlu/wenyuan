import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import DecisionSummary from './DecisionSummary.vue'
import type { Decision } from '../domain/session'

const decision: Decision = {
  status: 'majority_reached',
  selected_proposal: {
    id: 'p1',
    proposed_by: 'mouyuan',
    title: '最小闭环策案',
    summary: '先跑通 Mock 再接入真实模型',
    user_value: '降低讨论成本',
    implementation_path: '分阶段交付',
    risks: [],
    success_metrics: [],
  },
  vote_count: 3,
  majority_reasons: ['更容易落地'],
  minority_opinion: ['需要关注风险'],
  adoption_conditions: ['先完成单元测试'],
  unresolved_questions: ['如何评估效果？'],
  next_steps: ['部署到 staging'],
  self_vote_count: 1,
}

describe('DecisionSummary', () => {
  it('shows majority and minority opinions', () => {
    const wrapper = mount(DecisionSummary, { props: { decision } })
    expect(wrapper.text()).toContain('形成多数')
    expect(wrapper.text()).toContain('更容易落地')
    expect(wrapper.text()).toContain('行动清单')
    expect(wrapper.text()).toContain('需要注意')
  })
})
