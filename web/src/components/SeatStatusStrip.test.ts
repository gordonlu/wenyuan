import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SeatStatusStrip from './SeatStatusStrip.vue'

describe('SeatStatusStrip', () => {
  it('renders three seat status updates', () => {
    const wrapper = mount(SeatStatusStrip, {
      props: { phase: 'cross_critique', events: [] },
    })
    expect(wrapper.text()).toContain('谋远席')
    expect(wrapper.text()).toContain('经世席')
    expect(wrapper.text()).toContain('持正席')
    expect(wrapper.text()).toContain('批议中')
  })
})
