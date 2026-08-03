import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import StatCard from '../app/components/common/StatCard.vue';

function mountStatCard(props: Record<string, unknown> = {}) {
  return mount(StatCard, {
    props,
    global: {
      stubs: {
        NuxtLink: {
          template: '<a :href="to"><slot /></a>',
          props: ['to'],
        },
        UIcon: { template: '<span />' },
      },
    },
  });
}

describe('StatCard', () => {
  it('renders label and value', () => {
    const wrapper = mountStatCard({
      label: 'Test Label',
      value: 42,
      icon: 'i-lucide-home',
    });
    expect(wrapper.text()).toContain('42');
    expect(wrapper.text()).toContain('Test Label');
  });

  it('renders as link when `to` prop is provided', () => {
    const wrapper = mountStatCard({
      label: 'Linked',
      value: '5',
      icon: 'i-lucide-home',
      to: '/test',
    });
    const link = wrapper.find('a');
    expect(link.exists()).toBe(true);
    expect(link.attributes('href')).toContain('/test');
  });

  it('renders as div when `to` prop is not provided', () => {
    const wrapper = mountStatCard({
      label: 'No Link',
      value: 0,
      icon: 'i-lucide-home',
    });
    expect(wrapper.find('a').exists()).toBe(false);
    expect(wrapper.find('div').exists()).toBe(true);
  });
});
