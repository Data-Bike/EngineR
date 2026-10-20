import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import AuditPage from '../app/pages/audit/index.vue';

const UCard = { template: '<div class="u-card"><slot name="header" /><slot /></div>' };
const UFormField = { props: ['label'], template: '<div class="u-form-field"><slot /></div>' };
const UButton = {
  emits: ['click'],
  template: '<button class="u-button" @click="$emit(\'click\')"><slot /></button>',
};
const UAlert = { template: '<div class="u-alert" />' };
const UBadge = { template: '<span class="u-badge"><slot /></span>' };
const UIcon = { props: ['name'], template: '<span class="u-icon" />' };
const UInput = {
  props: ['modelValue'],
  emits: ['update:modelValue'],
  template: '<input :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
};
const USelect = {
  props: ['modelValue', 'items', 'valueKey'],
  emits: ['update:modelValue'],
  // setValue() on a <select> only sticks when a matching <option>
  // exists — render them so tests can pick a catalog.
  template:
    '<select class="u-select" :value="modelValue" @change="$emit(\'update:modelValue\', $event.target.value)"><option v-for="i in items" :key="i.value" :value="i.value">{{ i.label }}</option></select>',
};

const CATALOGS = [
  { public_id: 'c1', alias: 'wh', name: 'Склад' },
  { public_id: 'c2', alias: 'qa_t', name: 'QA' },
];

// The page imports useApi explicitly, so the auto-import stub does not
// apply — mock the module instead.
const query = vi.hoisted(() => vi.fn());
vi.mock('~/composables/useApi', () => ({
  useApi: () => ({
    audit: {
      query,
      latestCheckpoint: vi.fn().mockResolvedValue({ data: null }),
      getRetentionPolicy: vi.fn().mockResolvedValue({
        data: { retention_days: 365, legal_hold: false, archive_before_delete: true, archive_url: null },
      }),
      verifyChain: vi.fn(),
      createCheckpoint: vi.fn(),
      setRetentionPolicy: vi.fn(),
    },
  }),
}));

function mountPage() {
  query.mockReset().mockResolvedValue({ data: [] });
  (globalThis as any).useCatalog = () => ({
    catalogParams: () => ({}),
    selected: ref(''),
    available: { value: CATALOGS },
    hasMultiple: { value: true },
    select: vi.fn(),
  });
  const wrapper = mount(AuditPage, {
    global: { stubs: { UCard, UFormField, UButton, UAlert, UBadge, UIcon, UInput, USelect } },
  });
  return { wrapper, query };
}

describe('AuditPage catalog filter', () => {
  beforeEach(() => vi.clearAllMocks());

  it('sends no catalog filter by default', async () => {
    const { query } = mountPage();
    await flushPromises();
    expect(query).toHaveBeenCalled();
    const arg = query.mock.calls.at(-1)?.[0] as Record<string, unknown>;
    expect(arg.catalog).toBeUndefined();
  });

  it('offers every readable catalog plus an all-catalogs sentinel', async () => {
    const { wrapper } = mountPage();
    await flushPromises();
    const selects = wrapper.findAllComponents(USelect);
    // kind select + catalog select
    const catalogSelect = selects[selects.length - 1];
    const items = catalogSelect.props('items') as { label: string; value: string }[];
    expect(items[0].value).toBe('__all__');
    expect(items.map((i) => i.value)).toEqual(['__all__', 'wh', 'qa_t']);
    // Reka UI rejects empty-string item values — the sentinel must be nonempty.
    expect(items.every((i) => i.value !== '')).toBe(true);
  });

  it('scopes the query to the chosen catalog and carries it into pagination', async () => {
    const { wrapper, query } = mountPage();
    await flushPromises();
    const selects = wrapper.findAllComponents(USelect);
    const catalogSelect = selects[selects.length - 1];
    await catalogSelect.find('select').setValue('qa_t');
    const buttons = wrapper.findAllComponents(UButton);
    // Search button triggers fetchAudit
    await buttons.find((b) => b.text().includes('common.search'))!.trigger('click');
    await flushPromises();
    const arg = query.mock.calls.at(-1)?.[0] as Record<string, unknown>;
    expect(arg.catalog).toBe('qa_t');
  });
});
