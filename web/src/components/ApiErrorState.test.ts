import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ApiErrorState from './ApiErrorState.vue'

describe('ApiErrorState', () => {
  it('renders api error state', () => {
    const wrapper = mount(ApiErrorState, { props: { message: '服务不可用' } })
    expect(wrapper.text()).toContain('服务不可用')
    expect(wrapper.attributes('role')).toBe('alert')
  })

  it('shows hint text when provided', () => {
    const wrapper = mount(ApiErrorState, { props: { message: '搜索失败', hint: '请检查搜索引擎配置' } })
    expect(wrapper.text()).toContain('请检查搜索引擎配置')
  })

  it('shows action button when actionLabel provided', () => {
    const wrapper = mount(ApiErrorState, { props: { message: '配置缺失', actionLabel: '前往设置' } })
    expect(wrapper.find('button').exists()).toBe(true)
    expect(wrapper.text()).toContain('前往设置')
  })

  it('emits action on button click', () => {
    const wrapper = mount(ApiErrorState, { props: { message: '配置缺失', actionLabel: '前往设置' } })
    wrapper.find('button').trigger('click')
    expect(wrapper.emitted('action')).toBeTruthy()
  })
})
