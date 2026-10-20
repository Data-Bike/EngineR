import { beforeEach, describe, expect, it, vi } from 'vitest';
import { reactive, ref } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import SearchPage from '../app/pages/search.vue';

const UCard = {
  template: '<div class="u-card"><slot /><slot name="header" /></div>',
};
const UBadge = {
  props: ['variant', 'size'],
  template: '<span class="u-badge"><slot /></span>',
};
const UButton = {
  props: ['variant', 'icon'],
  emits: ['click'],
  template: '<button @click="$emit(\'click\')"><slot /></button>',
};
const UInput = {
  props: ['modelValue', 'placeholder', 'icon'],
  emits: ['update:model-value', 'keyup.enter'],
  template: `
    <input class="u-input" :value="modelValue"
      @input="$emit('update:model-value', $event.target.value)"
      @keyup.enter="$emit('keyup.enter')" />
  `,
};
const NuxtLink = {
  props: ['to'],
  template: '<a :href="to"><slot /></a>',
};

function mountPage(opts: {
  query?: Record<string, string>;
  catalog?: string;
  list?: ReturnType<typeof vi.fn>;
} = {}) {
  const list =
    opts.list ?? vi.fn().mockResolvedValue({ data: [] });
  (globalThis as any).useApi = () => ({ entities: { list } });
  (globalThis as any).useRoute = () => ({
    params: {},
    query: opts.query ?? {},
  });
  (globalThis as any).useCatalog = () => ({
    catalogParams: () => (opts.catalog ? { catalog: opts.catalog } : {}),
    selected: ref(opts.catalog ?? ''),
    available: { value: [] },
    hasMultiple: { value: false },
    select: vi.fn(),
  });
  (globalThis as any).navigateTo = vi.fn();
  const wrapper = mount(SearchPage, {
    global: { stubs: { UCard, UBadge, UButton, UInput, NuxtLink } },
  });
  return { wrapper, list };
}

describe('SearchPage', () => {
  beforeEach(() => vi.clearAllMocks());

  it('queries entities with the q param and the selected catalog', async () => {
    const { list } = mountPage({ query: { q: 'ромашка' }, catalog: 'wh' });
    await flushPromises();
    expect(list).toHaveBeenCalledWith(
      expect.objectContaining({ search: 'ромашка', limit: 50, catalog: 'wh' }),
    );
  });

  it('omits catalog when no catalog is selected', async () => {
    const { list } = mountPage({ query: { q: 'x' } });
    await flushPromises();
    expect(list).toHaveBeenCalledWith(expect.objectContaining({ catalog: undefined }));
  });

  it('renders result cards with type and catalog', async () => {
    const list = vi.fn().mockResolvedValue({
      data: [
        { public_id: 'p1', display_name: 'Склад №1', type_id: 'wh_warehouse', catalog: 'wh', status: 'active' },
      ],
    });
    const { wrapper } = mountPage({ query: { q: 'склад' }, list });
    await flushPromises();

    expect(wrapper.text()).toContain('Склад №1');
    expect(wrapper.text()).toContain('wh_warehouse · wh');
    const link = wrapper.findComponent(NuxtLink);
    expect(link.props('to')).toBe('/entities/p1');
  });

  it('shows the empty state when nothing matches', async () => {
    const { wrapper } = mountPage({ query: { q: 'нет' } });
    await flushPromises();
    expect(wrapper.text()).toContain('entity.empty');
  });

  it('re-runs the search when the q param changes', async () => {
    const route = reactive({ params: {}, query: { q: 'one' } });
    const list = vi.fn().mockResolvedValue({ data: [] });
    (globalThis as any).useApi = () => ({ entities: { list } });
    (globalThis as any).useRoute = () => route;
    (globalThis as any).navigateTo = vi.fn();
    mount(SearchPage, {
      global: { stubs: { UCard, UBadge, UButton, UInput, NuxtLink } },
    });
    await flushPromises();
    route.query = { q: 'two' };
    await flushPromises();
    expect(list).toHaveBeenLastCalledWith(expect.objectContaining({ search: 'two' }));
  });
});
