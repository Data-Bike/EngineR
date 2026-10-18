import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import EntityFormRenderer from '../app/components/entity/EntityFormRenderer.vue';
import type { ViewDefinition } from '../web/src/types/view.js';

const view: ViewDefinition = {
  view_id: 'legal_entity_form',
  view_type: 'form',
  type_id: 'legal_entity',
  schema_revision: 7,
  name_key: 'types.legal_entity',
  sections: [{
    label_key: 'sections.main',
    order: 0,
    collapsible: false,
    default_collapsed: false,
    fields: [
      {
        predicate_id: 'full_name',
        alias: 'full_name',
        label_key: 'fields.legal_entity.full_name',
        kind: 'text',
        required: true,
        read_only: false,
        identity: true,
        hidden: false,
        min_cardinality: 1,
        max_cardinality: 1,
        order: 0,
      },
      {
        predicate_id: 'has_tax_id',
        alias: 'has_tax_id',
        label_key: 'fields.legal_entity.inn',
        kind: 'text',
        required: false,
        read_only: false,
        identity: true,
        hidden: false,
        min_cardinality: 0,
        max_cardinality: 1,
        order: 1,
      },
    ],
  }],
  columns: [],
  default_sort: [],
  filters: [],
  facets: [],
  row_actions: [],
  bulk_actions: [],
  export_allowed: true,
};

function mountForm() {
  return mount(EntityFormRenderer, {
    props: {
      view,
      initialValues: {
        full_name: 'ООО Исходное имя',
        has_tax_id: '7701234567',
      },
      submitLabel: 'Сохранить',
    },
    global: {
      stubs: {
        FormField: {
          props: ['field', 'modelValue'],
          emits: ['update:value'],
          template: '<input :data-field="field.predicate_id" :value="modelValue" @input="$emit(\'update:value\', $event.target.value)" />',
        },
        UButton: {
          props: ['type'],
          template: '<button :type="type || \'button\'"><slot /></button>',
        },
      },
    },
  });
}

describe('EntityFormRenderer edit scenario', () => {
  it('shows existing object values and submits edited fields without losing untouched identity data', async () => {
    const wrapper = mountForm();
    const name = wrapper.get('[data-field="full_name"]');
    const inn = wrapper.get('[data-field="has_tax_id"]');
    expect((name.element as HTMLInputElement).value).toBe('ООО Исходное имя');
    expect((inn.element as HTMLInputElement).value).toBe('7701234567');

    await name.setValue('ООО Обновлённое имя');
    await wrapper.get('form').trigger('submit');

    const submission = wrapper.emitted('submit')?.[0]?.[0] as {
      type_id: string;
      schema_revision: number;
      fields: Array<{ predicate_id: string; value: unknown }>;
    };
    expect(submission.type_id).toBe('legal_entity');
    expect(submission.schema_revision).toBe(7);
    expect(submission.fields).toEqual(expect.arrayContaining([
      { predicate_id: 'full_name', value: 'ООО Обновлённое имя' },
      { predicate_id: 'has_tax_id', value: '7701234567' },
    ]));
  });
});
