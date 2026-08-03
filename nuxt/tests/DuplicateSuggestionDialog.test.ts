import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import DuplicateSuggestionDialog from '../app/components/entity/DuplicateSuggestionDialog.vue';

interface DuplicateMatch {
  public_id: string;
  type_id: string;
  match_type: 'exact' | 'fuzzy';
  score?: number;
  explanation?: string;
}

function mountDialog(props: Partial<{ open: boolean; matches: DuplicateMatch[]; identityMode: string }> = {}) {
  return mount(DuplicateSuggestionDialog, {
    props: {
      open: true,
      matches: [],
      identityMode: 'return_existing',
      ...props,
    },
    global: {
      stubs: {
        UModal: { template: '<div v-if="open"><slot name="content" /></div>', props: ['open'] },
        UButton: {
          template: '<button @click="$emit(\'click\')"><slot /></button>',
          props: ['size', 'variant', 'color', 'icon'],
        },
        UBadge: { template: '<span><slot /></span>', props: ['variant', 'size', 'color'] },
      },
    },
  });
}

describe('DuplicateSuggestionDialog', () => {
  it('shows no matches message when matches is empty', () => {
    const wrapper = mountDialog({ matches: [] });
    expect(wrapper.text()).toContain('entity.duplicate.no_matches');
  });

  it('renders exact matches with open-existing button', () => {
    const matches: DuplicateMatch[] = [
      { public_id: 'ent-001', type_id: 'person', match_type: 'exact' },
    ];
    const wrapper = mountDialog({ matches });
    expect(wrapper.text()).toContain('ent-001');
    expect(wrapper.text()).toContain('entity.duplicate.exact_match');
    expect(wrapper.text()).toContain('entity.duplicate.open_existing');
  });

  it('renders fuzzy matches with score', () => {
    const matches: DuplicateMatch[] = [
      { public_id: 'ent-002', type_id: 'person', match_type: 'fuzzy', score: 0.85, explanation: 'name similarity' },
    ];
    const wrapper = mountDialog({ matches });
    expect(wrapper.text()).toContain('ent-002');
    expect(wrapper.text()).toContain('0.85');
    expect(wrapper.text()).toContain('name similarity');
    expect(wrapper.text()).toContain('entity.duplicate.fuzzy_match');
  });

  it('shows create-distinct button only for non-reject identity mode', () => {
    const matches: DuplicateMatch[] = [
      { public_id: 'ent-003', type_id: 'person', match_type: 'exact' },
    ];
    const wrapperReject = mountDialog({ matches, identityMode: 'reject' });
    expect(wrapperReject.text()).not.toContain('entity.duplicate.create_distinct');

    const wrapperAllow = mountDialog({ matches, identityMode: 'return_existing' });
    expect(wrapperAllow.text()).toContain('entity.duplicate.create_distinct');
  });

  it('emits open-existing when button clicked', async () => {
    const matches: DuplicateMatch[] = [
      { public_id: 'ent-004', type_id: 'person', match_type: 'exact' },
    ];
    const wrapper = mountDialog({ matches });
    const buttons = wrapper.findAll('button');
    const openBtn = buttons.find((b) => b.text().includes('entity.duplicate.open_existing'));
    await openBtn!.trigger('click');
    expect(wrapper.emitted('open-existing')).toBeTruthy();
    expect(wrapper.emitted('open-existing')![0]).toEqual(['ent-004']);
  });

  it('emits create-distinct when button clicked', async () => {
    const matches: DuplicateMatch[] = [
      { public_id: 'ent-005', type_id: 'person', match_type: 'exact' },
    ];
    const wrapper = mountDialog({ matches, identityMode: 'return_existing' });
    const buttons = wrapper.findAll('button');
    const createBtn = buttons.find((b) => b.text().includes('entity.duplicate.create_distinct'));
    await createBtn!.trigger('click');
    expect(wrapper.emitted('create-distinct')).toBeTruthy();
  });

  it('emits cancel when cancel button clicked', async () => {
    const wrapper = mountDialog({ matches: [] });
    const buttons = wrapper.findAll('button');
    const cancelBtn = buttons.find((b) => b.text().includes('common.cancel'));
    await cancelBtn!.trigger('click');
    expect(wrapper.emitted('cancel')).toBeTruthy();
  });
});
