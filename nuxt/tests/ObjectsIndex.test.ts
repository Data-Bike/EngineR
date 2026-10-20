import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import ObjectsIndex from '../app/pages/objects/index.vue';

const UCard = {
  emits: ['click'],
  template: '<div class="u-card" @click="$emit(\'click\')"><slot name="header" /><slot /></div>',
};
const UIcon = { props: ['name'], template: '<span class="u-icon" />' };
const UBadge = {
  props: ['variant', 'size'],
  template: '<span class="u-badge"><slot /></span>',
};

const TYPES = [
  { id: 'person', name: 'person', display_name: 'Person', status: 'active', catalog: 'general', entity_count: 3 },
  { id: 'person', name: 'person', display_name: 'Персона QA', status: 'active', catalog: 'qa_t', entity_count: 1 },
];

function mountPage(opts: { catalog?: string; types?: unknown[] } = {}) {
  const listTypes = vi.fn().mockResolvedValue({ data: opts.types ?? TYPES });
  // A shared ref mirrors the real composable — catalogParams() derives
  // from the live selection so switching catalogs changes the scope.
  const selected = ref(opts.catalog ?? '');
  (globalThis as any).useApi = () => ({ schemas: { listTypes } });
  (globalThis as any).useCatalog = () => ({
    catalogParams: () => (selected.value ? { catalog: selected.value } : {}),
    selected,
    available: { value: [] },
    hasMultiple: { value: false },
    select: vi.fn((a: string) => { selected.value = a; }),
  });
  (globalThis as any).useRoute = () => ({ params: {}, query: {} });
  (globalThis as any).navigateTo = vi.fn();
  const wrapper = mount(ObjectsIndex, {
    global: { stubs: { UCard, UIcon, UBadge } },
  });
  return { wrapper, listTypes, selected };
}

describe('ObjectsIndex', () => {
  beforeEach(() => vi.clearAllMocks());

  it('scopes the type listing to the selected catalog', async () => {
    const { listTypes } = mountPage({ catalog: 'wh' });
    await flushPromises();
    expect(listTypes).toHaveBeenCalledWith({ catalog: 'wh' });
  });

  it('shows the catalog next to the alias so same-alias types stay distinct', async () => {
    const { wrapper } = mountPage();
    await flushPromises();
    const text = wrapper.text();
    expect(text).toContain('person · general');
    expect(text).toContain('person · qa_t');
  });

  it('navigates with the row catalog pinned in the route query', async () => {
    const { wrapper } = mountPage();
    await flushPromises();
    const cards = wrapper.findAllComponents(UCard);
    expect(cards).toHaveLength(2);
    await cards[1].trigger('click');
    expect((globalThis as any).navigateTo).toHaveBeenCalledWith({
      path: '/objects/person',
      query: { catalog: 'qa_t' },
    });
  });

  it('refetches types when the header catalog selection changes', async () => {
    const { listTypes, selected } = mountPage();
    await flushPromises();
    expect(listTypes).toHaveBeenCalledTimes(1);
    selected.value = 'qa_t';
    await flushPromises();
    expect(listTypes).toHaveBeenCalledTimes(2);
    expect(listTypes).toHaveBeenLastCalledWith({ catalog: 'qa_t' });
    selected.value = '';
    await flushPromises();
    expect(listTypes).toHaveBeenLastCalledWith({});
  });
});
