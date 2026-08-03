import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import FormField from '../app/components/entity/FormField.vue';
import type { FieldDefinition } from '../web/src/types/view.js';

function mountField(field: Partial<FieldDefinition>, modelValue: unknown = '') {
  const fullField: FieldDefinition = {
    predicate_id: 'pred-1',
    alias: 'name',
    label_key: 'fields.name',
    kind: 'text',
    required: false,
    read_only: false,
    identity: false,
    hidden: false,
    min_cardinality: 0,
    order: 0,
    ...field,
  };
  return mount(FormField, {
    props: { field: fullField, modelValue },
    global: {
      stubs: {
        UFormField: { template: '<div><slot /><slot name="hint" /></div>' },
        UInput: { template: '<input :type="type" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />', props: ['modelValue', 'type', 'disabled', 'placeholder'] },
        UTextarea: { template: '<textarea :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />', props: ['modelValue', 'rows', 'disabled'] },
        USwitch: { template: '<button @click="$emit(\'update:modelValue\', !modelValue)" />', props: ['modelValue', 'disabled'] },
        USelect: { template: '<select @change="$emit(\'update:modelValue\', $event.target.value)"><slot /></select>', props: ['modelValue', 'items', 'disabled'] },
        UIcon: { template: '<span />' },
        EntityAutocomplete: { template: '<div />' },
        MultiValueField: { template: '<div />' },
      },
    },
  });
}

describe('FormField', () => {
  it('renders text input for text kind', () => {
    const wrapper = mountField({ kind: 'text' }, 'hello');
    expect(wrapper.find('input').exists()).toBe(true);
    expect(wrapper.find('input').element.value).toBe('hello');
  });

  it('renders textarea for textarea kind', () => {
    const wrapper = mountField({ kind: 'textarea' }, 'long text');
    expect(wrapper.find('textarea').exists()).toBe(true);
  });

  it('renders switch for boolean kind', () => {
    const wrapper = mountField({ kind: 'boolean' }, true);
    expect(wrapper.find('button').exists()).toBe(true);
  });

  it('renders number input for integer kind', () => {
    const wrapper = mountField({ kind: 'integer' }, '42');
    const input = wrapper.find('input');
    expect(input.exists()).toBe(true);
    expect(input.attributes('type')).toBe('number');
  });

  it('renders date input for date kind', () => {
    const wrapper = mountField({ kind: 'date' }, '2026-01-01');
    expect(wrapper.find('input').attributes('type')).toBe('date');
  });

  it('renders datetime-local input for datetime kind', () => {
    const wrapper = mountField({ kind: 'datetime' }, '2026-01-01T00:00');
    expect(wrapper.find('input').attributes('type')).toBe('datetime-local');
  });

  it('renders file input for file kind', () => {
    const wrapper = mountField({ kind: 'file' });
    expect(wrapper.find('input').attributes('type')).toBe('file');
  });

  it('renders read-only div for derived kind', () => {
    const wrapper = mountField({ kind: 'derived', read_only: true }, 'computed');
    expect(wrapper.text()).toContain('computed');
  });

  it('renders unsupported message for unknown kind', () => {
    const wrapper = mountField({ kind: 'relation_editor' as any });
    // Mock useI18n returns key as-is
    expect(wrapper.text()).toContain('entity.unsupported_field');
  });

  it('emits update:value when input changes', async () => {
    const wrapper = mountField({ kind: 'text' }, '');
    await wrapper.find('input').setValue('new value');
    expect(wrapper.emitted('update:value')).toBeTruthy();
    expect(wrapper.emitted('update:value')![0]).toEqual(['new value']);
  });

  it('shows canonical preview for identity fields', () => {
    const wrapper = mountField({ kind: 'text', identity: true }, 'Hello');
    // The hint slot should be rendered (UFormField stub passes it through)
    expect(wrapper.text()).toContain('entity.canonical_preview');
  });
});
