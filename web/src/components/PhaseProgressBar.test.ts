import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PhaseProgressBar from './PhaseProgressBar.vue'

function createEvent(type: string, seat: string) {
  return { id: 0, event_type: type, payload: { seat }, created_at: new Date().toISOString() }
}

describe('PhaseProgressBar', () => {
  it('renders all phase steps', () => {
    const wrapper = mount(PhaseProgressBar, {
      props: { phase: 'draft' },
    })
    expect(wrapper.text()).toContain('待陈策')
    expect(wrapper.text()).toContain('独议')
    expect(wrapper.text()).toContain('批议')
    expect(wrapper.text()).toContain('完成')
  })

  it('marks current phase as active', () => {
    const wrapper = mount(PhaseProgressBar, {
      props: { phase: 'cross_critique' },
    })
    const activeSteps = wrapper.findAll('.phase-step.active')
    expect(activeSteps.length).toBe(1)
    expect(activeSteps[0].text()).toContain('批议')
  })

  it('marks previous phases as done', () => {
    const wrapper = mount(PhaseProgressBar, {
      props: { phase: 'revision' },
    })
    const doneSteps = wrapper.findAll('.phase-step.done')
    const doneLabels = doneSteps.map((s) => s.text())
    expect(doneLabels.some((l) => l.includes('待陈策'))).toBe(true)
    expect(doneLabels.some((l) => l.includes('独议'))).toBe(true)
    expect(doneLabels.some((l) => l.includes('批议'))).toBe(true)
  })

  it('shows status text when running in independent deliberation', () => {
    const wrapper = mount(PhaseProgressBar, {
      props: {
        phase: 'independent_deliberation',
        running: true,
      },
    })
    expect(wrapper.text()).toContain('三席正在独立陈策')
  })

  it('shows seat-specific status text when latest event is seat_started', () => {
    const wrapper = mount(PhaseProgressBar, {
      props: {
        phase: 'cross_critique',
        running: true,
        events: [createEvent('seat_started', 'jingshi')],
      },
    })
    expect(wrapper.text()).toContain('经世席正在批议')
  })

  it('shows convergence text for convergence phase', () => {
    const wrapper = mount(PhaseProgressBar, {
      props: { phase: 'convergence' },
    })
    expect(wrapper.text()).toContain('合案复议')
  })

  it('hides status text when phase is completed', () => {
    const wrapper = mount(PhaseProgressBar, {
      props: { phase: 'completed' },
    })
    expect(wrapper.find('.phase-status-text').exists()).toBe(false)
  })
})
