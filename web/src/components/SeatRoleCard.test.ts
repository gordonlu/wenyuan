import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SeatRoleCard from './SeatRoleCard.vue'

function createEvent(type: string, seat: string) {
  return { id: 0, event_type: type, payload: { seat }, created_at: new Date().toISOString() }
}

describe('SeatRoleCard', () => {
  it('renders seat name and role summary', () => {
    const wrapper = mount(SeatRoleCard, {
      props: { seat: 'mouyuan', phase: 'draft' },
    })
    expect(wrapper.text()).toContain('谋远席')
    expect(wrapper.text()).toContain('找新路径')
  })

  it('adds is-running class when seat is actively running', () => {
    const wrapper = mount(SeatRoleCard, {
      props: {
        seat: 'jingshi',
        phase: 'cross_critique',
        running: true,
        events: [createEvent('seat_started', 'jingshi')],
      },
    })
    expect(wrapper.find('.seat-role-card.is-running').exists()).toBe(true)
  })

  it('does not add is-running for completed seat', () => {
    const wrapper = mount(SeatRoleCard, {
      props: {
        seat: 'chizheng',
        phase: 'completed',
        events: [createEvent('seat_completed', 'chizheng')],
      },
    })
    expect(wrapper.find('.seat-role-card.is-running').exists()).toBe(false)
  })

  it('shows failure count when seat has failures', () => {
    const wrapper = mount(SeatRoleCard, {
      props: {
        seat: 'mouyuan',
        phase: 'failed',
        runs: [{ id: 'r1', session_id: '', seat: 'mouyuan', phase: 'independent_deliberation', status: 'failed', prompt_version: 'v1', repair_attempted: false, duration_ms: 100, error: 'timeout', raw_output: null, prompt_tokens: null, completion_tokens: null, total_tokens: null, upstream_status: null }],
      },
    })
    expect(wrapper.find('.runtime-failures').exists()).toBe(true)
    expect(wrapper.text()).toContain('失败')
  })

  it('sets aria-busy when running', () => {
    const wrapper = mount(SeatRoleCard, {
      props: {
        seat: 'mouyuan',
        phase: 'independent_deliberation',
        running: true,
        events: [createEvent('seat_started', 'mouyuan')],
      },
    })
    expect(wrapper.classes()).toContain('is-running')
  })
})
