import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import EditPage from '../app/pages/entities/[publicId]/edit.vue';

const UButton = {
  props: ['variant', 'icon'],
  emits: ['click'],
  template: '<button @click="$emit(\'click\')"><slot /></button>',
};

const markSaved = vi.fn();
// Emits a submit with structured field values to prove the page forwards
// raw values — the server, not the UI, applies the declared field kind.
const EntityFormRenderer = {
  props: ['view', 'initialValues', 'submitting'],
  emits: ['submit', 'cancel'],
  methods: { markSaved },
  template: `
    <button class="submit" @click="$emit('submit', {
      fields: [
        { predicate_id: 'price', value: { amount: '12.5', currency: 'USD' } },
        { predicate_id: 'active', value: true },
        { predicate_id: 'name', value: 'Склад №1' },
      ],
    })">save</button>
  `,
};

function mountPage(opts: { entity?: Record<string, unknown> } = {}) {
  const entity = opts.entity ?? {
    public_id: 'p1',
    display_name: 'Склад №1',
    type_id: 'wh_warehouse',
    catalog: 'wh',
    revision: 3,
    fields: {},
  };
  const get = vi.fn().mockResolvedValue({ data: entity });
  const update = vi.fn().mockResolvedValue({ data: { ...entity, revision: 4 } });
  const getTypeView = vi.fn().mockResolvedValue({ data: { sections: [] } });
  (globalThis as any).useApi = () => ({
    entities: { get, update },
    schemas: { getTypeView },
  });
  (globalThis as any).useRoute = () => ({ params: { publicId: 'p1' }, query: {} });
  (globalThis as any).navigateTo = vi.fn();
  const wrapper = mount(EditPage, {
    global: { stubs: { UButton, EntityFormRenderer } },
  });
  return { wrapper, get, update, getTypeView };
}

describe('EntityEditPage', () => {
  beforeEach(() => vi.clearAllMocks());

  it('loads the form view scoped to the entity own catalog', async () => {
    const { getTypeView } = mountPage();
    await flushPromises();
    expect(getTypeView).toHaveBeenCalledWith('wh_warehouse', 'form', { catalog: 'wh' });
  });

  it('resolves the form view without a catalog param when the entity has none', async () => {
    const { getTypeView } = mountPage({
      entity: { public_id: 'p1', display_name: 'X', type_id: 'person', revision: 1, fields: {} },
    });
    await flushPromises();
    expect(getTypeView).toHaveBeenCalledWith('person', 'form', undefined);
  });

  it('sends raw field values to update instead of forcing kind:text', async () => {
    const { wrapper, update } = mountPage();
    await flushPromises();
    await wrapper.find('.submit').trigger('click');
    await flushPromises();

    expect(update).toHaveBeenCalledWith('p1', {
      revision: 3,
      fields: [
        { predicate_id: 'price', value: { amount: '12.5', currency: 'USD' } },
        { predicate_id: 'active', value: true },
        { predicate_id: 'name', value: 'Склад №1' },
      ],
    });
    expect(markSaved).toHaveBeenCalled();
    expect((globalThis as any).navigateTo).toHaveBeenCalledWith('/entities/p1');
  });
});
