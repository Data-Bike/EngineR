/**
 * AppBreadcrumbs: path-derived crumbs — known segments get i18n labels,
 * dynamic ids render raw, last crumb has no link, home is always first.
 * The bar is hidden on the landing page itself.
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { reactive } from 'vue';
import { mount } from '@vue/test-utils';
import AppBreadcrumbs from '../app/components/AppBreadcrumbs.vue';

const UBreadcrumb = {
  props: ['items'],
  template:
    '<ol class="u-breadcrumb"><li v-for="(item, i) in items" :key="i" :data-to="item.to">{{ item.label }}</li></ol>',
};

const route = reactive({ path: '/' });

function mountCrumbs() {
  (globalThis as any).useRoute = () => route;
  return mount(AppBreadcrumbs, {
    global: { components: { UBreadcrumb } },
  });
}

beforeEach(() => {
  route.path = '/';
});

describe('AppBreadcrumbs', () => {
  it('renders nothing on the home page', () => {
    route.path = '/';
    const wrapper = mountCrumbs();
    expect(wrapper.find('nav').exists()).toBe(false);
  });

  it('home crumb is always first and links to /', () => {
    route.path = '/objects';
    const wrapper = mountCrumbs();
    const lis = wrapper.findAll('li');
    expect(lis[0].text()).toBe('nav.home');
    expect(lis[0].attributes('data-to')).toBe('/');
  });

  it('maps a known top-level segment to its nav label', () => {
    route.path = '/audit';
    const wrapper = mountCrumbs();
    const lis = wrapper.findAll('li');
    expect(lis.map((l) => l.text())).toEqual(['nav.home', 'nav.audit']);
  });

  it('intermediate crumbs link, the last crumb does not', () => {
    route.path = '/admin/permissions';
    const wrapper = mountCrumbs();
    const lis = wrapper.findAll('li');
    expect(lis[1].text()).toBe('nav.admin');
    expect(lis[1].attributes('data-to')).toBe('/admin');
    expect(lis[2].text()).toBe('admin.permissions_title');
    expect(lis[2].attributes('data-to')).toBeUndefined();
  });

  it('dynamic segments render the raw path segment', () => {
    route.path = '/objects/person/create';
    const wrapper = mountCrumbs();
    const lis = wrapper.findAll('li');
    expect(lis.map((l) => l.text())).toEqual([
      'nav.home',
      'nav.objects',
      'person',
      'entity.create',
    ]);
    expect(lis[2].attributes('data-to')).toBe('/objects/person');
  });

  it('entity crumb links to /objects — /entities has no index route', () => {
    route.path = '/entities/018f-abc';
    const wrapper = mountCrumbs();
    const lis = wrapper.findAll('li');
    expect(lis.map((l) => l.text())).toEqual([
      'nav.home',
      'nav.objects',
      '018f-abc',
    ]);
    // The crumb must not link to /entities (404) — it links to /objects.
    expect(lis[1].attributes('data-to')).toBe('/objects');
  });

  it('does not link an intermediate crumb whose path resolves to no route', () => {
    // '/nope' and '/nope/x' resolve to nothing (router stub deny list) —
    // crumbs render as text instead of linking into a 404.
    (globalThis as any).__routerDeny = ['/nope', '/nope/x'];
    route.path = '/nope/x/y';
    const wrapper = mountCrumbs();
    const lis = wrapper.findAll('li');
    expect(lis[1].text()).toBe('nope');
    expect(lis[1].attributes('data-to')).toBeUndefined();
    expect(lis[2].text()).toBe('x');
    expect(lis[2].attributes('data-to')).toBeUndefined();
    (globalThis as any).__routerDeny = ['/entities'];
  });

  it('updates when the route changes', async () => {
    route.path = '/objects';
    const wrapper = mountCrumbs();
    expect(wrapper.findAll('li')).toHaveLength(2);
    route.path = '/admin/schema';
    await wrapper.vm.$nextTick();
    const lis = wrapper.findAll('li');
    expect(lis.map((l) => l.text())).toEqual([
      'nav.home',
      'nav.admin',
      'schema.title',
    ]);
  });
});
