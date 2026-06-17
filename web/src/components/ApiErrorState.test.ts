import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ApiErrorState from './ApiErrorState.vue'

describe('ApiErrorState', () => {
  it('renders api error state', () => {
    const wrapper = mount(ApiErrorState, { props: { message: '服务不可用' } })
    expect(wrapper.text()).toContain('服务不可用')
    expect(wrapper.attributes('role')).toBe('alert')
  })
})
