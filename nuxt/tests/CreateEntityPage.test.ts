import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import CreatePage from '../app/pages/objects/[typeAlias]/create.vue';

const UButton = {
  props: ['variant', 'icon'],
  emits: ['click'],
  template: '<button @click="$emit(\'click\')"><slot /></button>',
};

const markSaved = vi.fn();
const EntityFormRenderer = {
  props: ['view', 'submitting'],
  emits: ['submit', 'cancel', 'save-draft'],
  methods: { markSaved },
  template: `
    <button class="submit" @click="$emit('submit', {
      type_id: 'wh_warehouse',
      fields: [
        { predicate_id: 'volume', value: { amount: '42', unit: 'm3' } },
        { predicate_id: 'open', value: false },
      ],
    })">save</button>
  `,
};

const DuplicateSuggestionDialog = {
  props: ['open', 'matches', 'identityMode'],
  template: '<div class="dup-dialog" />',
};

function mountPage(opts: {
  routeQuery?: Record<string, string>;
  catalog?: string;
  create?: ReturnType<typeof vi.fn>;
} = {}) {
  const getTypeView = vi.fn().mockResolvedValue({ data: { sections: [] } });
  const create =
    opts.create ??
    vi.fn().mockResolvedValue({ data: { outcome: 'created', public_id: 'p9' } });
  const query = vi.fn().mockResolvedValue({ data: { rows: [] } });
  (globalThis as any).useApi = () => ({
    entities: { create, query },
    schemas: { getTypeView },
  });
  (globalThis as any).useRoute = () => ({
    params: { typeAlias: 'wh_warehouse' },
    query: opts.routeQuery ?? {},
  });
  (globalThis as any).useCatalog = () => ({
    catalogParams: () => (opts.catalog ? { catalog: opts.catalog } : {}),
    selected: ref(opts.catalog ?? ''),
    available: { value: [] },
    hasMultiple: { value: false },
    select: vi.fn(),
  });
  (globalThis as any).navigateTo = vi.fn();
  const wrapper = mount(CreatePage, {
    global: { stubs: { UButton, EntityFormRenderer, DuplicateSuggestionDialog } },
  });
  return { wrapper, create, getTypeView, query };
}

describe('EntityCreatePage', () => {
  beforeEach(() => vi.clearAllMocks());

  it('scopes schema and create calls to the catalog pinned in the route query', async () => {
    const { getTypeView, create, wrapper } = mountPage({
      routeQuery: { catalog: 'qa_t' },
      catalog: 'wh',
    });
    await flushPromises();
    expect(getTypeView).toHaveBeenCalledWith('wh_warehouse', 'form', { catalog: 'qa_t' });

    await wrapper.find('.submit').trigger('click');
    await flushPromises();
    expect(create).toHaveBeenCalledWith(expect.anything(), { catalog: 'qa_t' });
  });

  it('falls back to the header catalog selection without a route query', async () => {
    const { getTypeView, create, wrapper } = mountPage({ catalog: 'wh' });
    await flushPromises();
    expect(getTypeView).toHaveBeenCalledWith('wh_warehouse', 'form', { catalog: 'wh' });
    await wrapper.find('.submit').trigger('click');
    await flushPromises();
    expect(create).toHaveBeenCalledWith(expect.anything(), { catalog: 'wh' });
  });

  it('sends raw field values instead of forcing kind:text', async () => {
    const { create, wrapper } = mountPage();
    await flushPromises();
    await wrapper.find('.submit').trigger('click');
    await flushPromises();

    expect(create).toHaveBeenCalledWith(
      expect.objectContaining({
        statements: [
          { predicate_id: 'volume', value: { amount: '42', unit: 'm3' } },
          { predicate_id: 'open', value: false },
        ],
      }),
      expect.anything(),
    );
    expect(markSaved).toHaveBeenCalled();
    expect((globalThis as any).navigateTo).toHaveBeenCalledWith('/entities/p9');
  });
});
