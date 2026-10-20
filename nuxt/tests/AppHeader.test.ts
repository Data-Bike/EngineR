import { beforeEach, describe, expect, it, vi } from 'vitest';
import { computed, ref } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import AppHeader from '../app/components/AppHeader.vue';

const UButton = {
  props: ['icon', 'label', 'ariaLabel', 'to', 'color', 'variant', 'size'],
  emits: ['click'],
  template:
    '<button :aria-label="ariaLabel" :data-to="to" @click="$emit(\'click\')">{{ label }}</button>',
};

const USelect = {
  props: ['modelValue', 'items', 'icon', 'ariaLabel', 'size'],
  emits: ['update:model-value'],
  template: `
    <select class="u-select" :data-icon="icon" :aria-label="ariaLabel" @change="$emit('update:model-value', $event.target.value)">
      <option v-for="item in items" :key="item.value" :value="item.value" :selected="item.value === modelValue">{{ item.label }}</option>
    </select>
  `,
};

const UDropdownMenu = {
  props: ['items'],
  template: '<div class="u-dropdown"><slot /></div>',
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

function mountHeader(opts: {
  catalogs?: { alias: string; name: string }[];
  catalogParamsCatalog?: string;
  authenticated?: boolean;
  displayName?: string;
} = {}) {
  const catalogs = opts.catalogs ?? [];
  const select = vi.fn();
  const auth = {
    user: ref(opts.authenticated === false ? null : { display_name: opts.displayName ?? 'Admin' }),
    isAuthenticated: ref(opts.authenticated !== false),
    logout: vi.fn().mockResolvedValue(undefined),
  };
  (globalThis as any).useAuth = () => auth;
  (globalThis as any).useCatalog = () => ({
    catalogParams: () => (opts.catalogParamsCatalog ? { catalog: opts.catalogParamsCatalog } : {}),
    selected: ref(''),
    available: ref(catalogs),
    hasMultiple: computed(() => catalogs.length > 1),
    select,
  });
  (globalThis as any).navigateTo = vi.fn();
  (globalThis as any).useRoute = () => ({ params: {}, query: {} });
  const wrapper = mount(AppHeader, {
    global: { stubs: { UButton, USelect, UDropdownMenu, UInput, NuxtLink } },
  });
  return { wrapper, select, auth };
}

describe('AppHeader', () => {
  beforeEach(() => vi.clearAllMocks());

  it('passes a non-empty items array to the profile dropdown', () => {
    const { wrapper } = mountHeader();
    const dropdown = wrapper.findComponent(UDropdownMenu);
    expect(dropdown.props('items').length).toBeGreaterThanOrEqual(2);
    expect(dropdown.props('items').some((i: { label: string }) => i.label === 'auth.logout')).toBe(true);
  });

  it('renders the catalog selector only when several catalogs are readable', () => {
    const { wrapper: multi } = mountHeader({
      catalogs: [{ alias: 'wh', name: 'Warehouse' }, { alias: 'hr', name: 'HR' }],
    });
    const selects = multi.findAllComponents(USelect);
    const catalogSelect = selects.find((s) => s.props('icon') === 'i-lucide-layers');
    expect(catalogSelect).toBeTruthy();
    const values = catalogSelect!.props('items').map((i: { value: string }) => i.value);
    expect(values).toEqual(['__all__', 'wh', 'hr']);

    const { wrapper: single } = mountHeader({ catalogs: [{ alias: 'wh', name: 'Warehouse' }] });
    expect(
      single.findAllComponents(USelect).some((s) => s.props('icon') === 'i-lucide-layers'),
    ).toBe(false);
  });

  it('calls select() when the user picks a catalog', async () => {
    const { wrapper, select } = mountHeader({
      catalogs: [{ alias: 'wh', name: 'Warehouse' }, { alias: 'hr', name: 'HR' }],
    });
    const catalogSelect = wrapper
      .findAllComponents(USelect)
      .find((s) => s.props('icon') === 'i-lucide-layers')!;
    await catalogSelect.find('select').setValue('hr');
    expect(select).toHaveBeenCalledWith('hr');
  });

  it('maps the all-catalogs sentinel back to an empty selection', async () => {
    const { wrapper, select } = mountHeader({
      catalogs: [{ alias: 'wh', name: 'Warehouse' }, { alias: 'hr', name: 'HR' }],
    });
    const catalogSelect = wrapper
      .findAllComponents(USelect)
      .find((s) => s.props('icon') === 'i-lucide-layers')!;
    await catalogSelect.find('select').setValue('__all__');
    expect(select).toHaveBeenCalledWith('');
  });

  it('navigates to /search on submit of the global search input', async () => {
    const { wrapper } = mountHeader();
    const input = wrapper.findComponent(UInput);
    await input.find('input').setValue('ромашка');
    await input.find('input').trigger('keyup.enter');
    expect((globalThis as any).navigateTo).toHaveBeenCalledWith(
      expect.objectContaining({ path: '/search', query: { q: 'ромашка' } }),
    );
  });

  it('shows a mobile search entry hidden on desktop widths', () => {
    const { wrapper } = mountHeader();
    const searchBtn = wrapper
      .findAllComponents(UButton)
      .find((b) => b.props('icon') === 'i-lucide-search' && b.props('to') === '/search');
    expect(searchBtn).toBeTruthy();
    expect(searchBtn!.attributes('aria-label')).toBe('entity.search');
  });

  it('logs out through the profile dropdown item', async () => {
    const { wrapper, auth } = mountHeader({ displayName: 'Root' });
    const logout = auth.logout;
    const items = wrapper.findComponent(UDropdownMenu).props('items') as {
      label: string;
      onSelect?: () => void;
    }[];
    items.find((i) => i.label === 'auth.logout')!.onSelect!();
    await flushPromises();
    expect(logout).toHaveBeenCalled();
    expect((globalThis as any).navigateTo).toHaveBeenCalledWith('/login');
  });
});
