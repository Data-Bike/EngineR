import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import MultiValueField from '../app/components/entity/MultiValueField.vue';
import type { FieldDefinition } from '../web/src/types/view.js';

function mountMultiValue(
  field: Partial<FieldDefinition>,
  modelValue: unknown,
) {
  const fullField: FieldDefinition = {
    predicate_id: 'pred-1',
    alias: 'phones',
    label_key: 'fields.phones',
    kind: 'multi_value',
    required: false,
    read_only: false,
    identity: false,
    hidden: false,
    min_cardinality: 0,
    order: 0,
    ...field,
  };
  return mount(MultiValueField, {
    props: { field: fullField, modelValue },
    global: {
      stubs: {
        UButton: {
          template: '<button @click="$emit(\'click\')"><slot /></button>',
          props: ['icon', 'size', 'variant', 'color', 'disabled'],
        },
        UInput: {
          template: '<input :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
          props: ['modelValue', 'disabled'],
        },
        UIcon: { template: '<span />' },
      },
    },
  });
}

describe('MultiValueField', () => {
  it('renders existing items from array', () => {
    const wrapper = mountMultiValue({}, ['a', 'b', 'c']);
    const inputs = wrapper.findAll('input');
    expect(inputs).toHaveLength(3);
    expect(inputs[0].element.value).toBe('a');
    expect(inputs[1].element.value).toBe('b');
    expect(inputs[2].element.value).toBe('c');
  });

  it('wraps single value in array', () => {
    const wrapper = mountMultiValue({}, 'single');
    const inputs = wrapper.findAll('input');
    expect(inputs).toHaveLength(1);
    expect(inputs[0].element.value).toBe('single');
  });

  it('handles null/undefined as empty array', () => {
    const wrapper = mountMultiValue({}, null);
    expect(wrapper.findAll('input')).toHaveLength(0);
  });

  it('emits array with new item on add', async () => {
    const wrapper = mountMultiValue({ min_cardinality: 0 }, []);
    const buttons = wrapper.findAll('button');
    const addButton = buttons.find((b) => b.text().includes('entity.add_value'));
    expect(addButton).toBeTruthy();
    await addButton!.trigger('click');
    expect(wrapper.emitted('update:model-value')).toBeTruthy();
    expect(wrapper.emitted('update:model-value')![0]).toEqual([['']]);
  });

  it('emits array without removed item', async () => {
    const wrapper = mountMultiValue({ min_cardinality: 0 }, ['a', 'b']);
    const removeButtons = wrapper.findAll('button').filter((b) => !b.text().includes('entity.add_value'));
    expect(removeButtons).toHaveLength(2);
    await removeButtons[0].trigger('click');
    expect(wrapper.emitted('update:model-value')![0]).toEqual([['b']]);
  });

  it('respects max_cardinality', () => {
    const wrapper = mountMultiValue({ max_cardinality: 2 }, ['a', 'b']);
    const addButton = wrapper.findAll('button').find((b) => b.text().includes('entity.add_value'));
    expect(addButton).toBeFalsy();
  });

  it('respects min_cardinality for removal', () => {
    const wrapper = mountMultiValue({ min_cardinality: 1 }, ['a']);
    const removeButtons = wrapper.findAll('button').filter((b) => !b.text().includes('entity.add_value'));
    expect(removeButtons).toHaveLength(0);
  });
});
