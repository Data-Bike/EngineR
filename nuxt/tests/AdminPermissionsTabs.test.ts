/**
 * /admin/permissions class tabs: roles are grouped into 'data access'
 * and 'system access' tabs by group class; labels show per-tab counts;
 * a new role inherits the class of the tab it is created from.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import PermissionsPage from '../app/pages/admin/permissions.vue';

const UTabs = {
  props: ['modelValue', 'items'],
  emits: ['update:model-value'],
  template: `
    <div class="u-tabs">
      <button v-for="it in items" :key="it.value" :data-tab="it.value"
        :class="{ active: it.value === modelValue }"
        @click="$emit('update:model-value', it.value)">{{ it.label }}</button>
    </div>`,
};

const passthrough = (tag = 'div') => ({ template: `<${tag}><slot /></${tag}>` });
const UCard = passthrough();
const UModal = passthrough();
const UFormField = passthrough();
const UIcon = { template: '<i />' };
const UBadge = { template: '<span><slot /></span>' };
const UButton = {
  props: ['icon', 'loading'],
  template: '<button @click="$emit(\'click\')"><slot /></button>',
  emits: ['click'],
};
const UInput = { props: ['modelValue'], template: '<input />' };
const USelect = { props: ['modelValue', 'items'], template: '<select />' };
const UInputMenu = { props: ['modelValue', 'items'], template: '<input />' };

const GROUPS = [
  { id: 'g1', name: 'Readers', description: '', class: 'data', parent_id: null, special: false, principals: ['u1'], rules: [] },
  { id: 'g2', name: 'Editors', description: '', class: 'data', parent_id: null, special: false, principals: [], rules: [] },
  { id: 'g3', name: 'Sysadmins', description: '', class: 'capability', parent_id: null, special: false, principals: [], rules: [] },
];

function mountPage() {
  (globalThis as any).useApi = () => ({
    security: { listGroups: vi.fn().mockResolvedValue({ data: GROUPS }) },
  });
  return mount(PermissionsPage, {
    global: {
      components: { UTabs, UCard, UModal, UFormField, UIcon, UBadge, UButton, UInput, USelect, UInputMenu },
    },
  });
}

beforeEach(() => {
  (globalThis as any).useAuthStore = () => ({ hasCapability: () => true });
});

describe('permissions class tabs', () => {
  it('renders data and system tabs with counts', async () => {
    const wrapper = mountPage();
    await flushPromises();
    const tabs = wrapper.findAll('.u-tabs button');
    expect(tabs).toHaveLength(2);
    expect(tabs[0].text()).toContain('admin.tab_data');
    expect(tabs[0].text()).toContain('(2)');
    expect(tabs[1].text()).toContain('admin.tab_system');
    expect(tabs[1].text()).toContain('(1)');
  });

  it('data tab shows only data-class roles', async () => {
    const wrapper = mountPage();
    await flushPromises();
    const text = wrapper.text();
    expect(text).toContain('Readers');
    expect(text).toContain('Editors');
    expect(text).not.toContain('Sysadmins');
  });

  it('switching to system tab shows capability-class roles', async () => {
    const wrapper = mountPage();
    await flushPromises();
    await wrapper.find('.u-tabs button[data-tab="capability"]').trigger('click');
    const text = wrapper.text();
    expect(text).toContain('Sysadmins');
    expect(text).not.toContain('Readers');
  });

  it('shows empty state when a tab has no roles', async () => {
    (globalThis as any).useApi = () => ({
      security: { listGroups: vi.fn().mockResolvedValue({ data: [GROUPS[2]] }) },
    });
    const wrapper = mount(PermissionsPage, {
      global: {
        components: { UTabs, UCard, UModal, UFormField, UIcon, UBadge, UButton, UInput, USelect, UInputMenu },
      },
    });
    await flushPromises();
    // only the capability group exists — data tab (default) is empty
    expect(wrapper.text()).toContain('entity.empty');
    await wrapper.find('.u-tabs button[data-tab="capability"]').trigger('click');
    expect(wrapper.text()).toContain('Sysadmins');
  });
});
