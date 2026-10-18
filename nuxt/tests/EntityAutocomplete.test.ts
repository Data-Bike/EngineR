import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import EntityAutocomplete from '../app/components/entity/EntityAutocomplete.vue';

const UInputMenu = {
  props: ['modelValue', 'items', 'loading', 'searchTerm', 'placeholder', 'disabled'],
  emits: ['update:model-value', 'update:search-term', 'focus'],
  template: `
    <div>
      <input
        class="search-input"
        :value="searchTerm"
        @input="$emit('update:search-term', $event.target.value)"
        @focus="$emit('focus')"
      />
      <button
        v-for="item in items"
        :key="item.value"
        class="option"
        @click="$emit('update:model-value', item)"
      >{{ item.label }}</button>
    </div>
  `,
};

function mountAutocomplete(list: ReturnType<typeof vi.fn>, props = {}) {
  (globalThis as any).useApi = () => ({
    entities: {
      list,
      get: vi.fn().mockResolvedValue({ data: { display_name: 'Resolved', public_id: 'x' } }),
    },
  });
  return mount(EntityAutocomplete, {
    props: { modelValue: '', ...props },
    global: { stubs: { UInputMenu } },
  });
}

describe('EntityAutocomplete', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  it('loads initial options on mount so the field is not empty', async () => {
    const list = vi.fn().mockResolvedValue({
      data: [{ public_id: 'p1', display_name: 'ООО «Ромашка»', type_id: 'legal_entity' }],
    });
    const wrapper = mountAutocomplete(list);
    await flushPromises();

    expect(list).toHaveBeenCalledWith(expect.objectContaining({ search: undefined }));
    expect(wrapper.text()).toContain('ООО «Ромашка»');
    expect(wrapper.text()).not.toContain('No matching data');
  });

  it('queries the API with the typed search term (debounced)', async () => {
    const list = vi.fn().mockResolvedValue({ data: [] });
    const wrapper = mountAutocomplete(list);
    await flushPromises();
    list.mockClear();

    await wrapper.find('.search-input').setValue('рома');
    vi.advanceTimersByTime(300);
    await flushPromises();

    expect(list).toHaveBeenCalledWith(expect.objectContaining({ search: 'рома', limit: 20 }));
  });

  it('emits the selected entity public id', async () => {
    const list = vi.fn().mockResolvedValue({
      data: [{ public_id: 'p9', display_name: 'Цель', type_id: 'person' }],
    });
    const wrapper = mountAutocomplete(list);
    await flushPromises();

    await wrapper.find('.option').trigger('click');

    expect(wrapper.emitted('update:model-value')?.[0]).toEqual(['p9']);
  });

  it('restricts results to the target type when configured', async () => {
    const list = vi.fn().mockResolvedValue({ data: [] });
    mountAutocomplete(list, { targetTypeId: 'legal_entity' });
    await flushPromises();

    expect(list).toHaveBeenCalledWith(expect.objectContaining({ type: 'legal_entity' }));
  });
});
