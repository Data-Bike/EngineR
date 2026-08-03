import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import RiskLevelBadge from '../app/components/entity/RiskLevelBadge.vue';
import type { RiskLevel } from '../web/src/types/skr.js';

function mountBadge(level: RiskLevel) {
  return mount(RiskLevelBadge, {
    props: { level },
    global: {
      stubs: {
        UBadge: { template: '<span class="badge"><slot /></span>', props: ['color', 'variant', 'size'] },
        UIcon: { template: '<span class="icon" />' },
      },
    },
  });
}

describe('RiskLevelBadge', () => {
  it('renders low level', () => {
    const wrapper = mountBadge('low');
    expect(wrapper.text()).toContain('entity.risks.level.low');
  });

  it('renders medium level', () => {
    const wrapper = mountBadge('medium');
    expect(wrapper.text()).toContain('entity.risks.level.medium');
  });

  it('renders high level', () => {
    const wrapper = mountBadge('high');
    expect(wrapper.text()).toContain('entity.risks.level.high');
  });

  it('renders critical level', () => {
    const wrapper = mountBadge('critical');
    expect(wrapper.text()).toContain('entity.risks.level.critical');
  });
});
