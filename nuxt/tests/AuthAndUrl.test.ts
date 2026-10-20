/**
 * Authorization UX tests: auth store URL rules, capability middleware
 * (single + any-of), auth middleware URL deny — all exercising the real
 * Pinia store, no mocked permission logic.
 *
 * `ui_rules` mirrors `dataperm::can_url`: rules are `[{path, allow}]`
 * evaluated longest-prefix-first; `blanket` denies unmatched paths.
 */

import { describe, it, expect, vi, beforeAll, beforeEach } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const userProfile = (over: Record<string, unknown> = {}) => ({
  principal_id: 'p1',
  display_name: 'Test',
  tenant_id: 't1',
  locale: 'en',
  timezone: 'UTC',
  auth_strength: 'password',
  groups_revision: 1,
  capabilities: ['entity.read'],
  workspaces: [],
  policy_revision: 1,
  csrf_token: 'tok',
  ui_rules: { rules: [], blanket: false },
  ...over,
});

let useAuthStore: any;
let capability: any;
let authMw: any;

beforeAll(async () => {
  const g = globalThis as any;
  g.defineNuxtRouteMiddleware = (fn: unknown) => fn;
  g.createError = (o: { statusCode: number; statusMessage?: string }) =>
    Object.assign(new Error(o.statusMessage ?? 'error'), o);
  g.useRuntimeConfig = () => ({ public: { apiBaseUrl: '/api/v1' } });
  g.useRoute = () => ({ params: {}, query: {}, fullPath: '/' });
  g.navigateTo = vi.fn((x: unknown) => x);
  const mod = await import('../app/composables/useAuth');
  useAuthStore = mod.useAuthStore;
  g.useAuthStore = mod.useAuthStore; // auto-import stub for middleware files
  const cap = await import('../app/middleware/capability');
  capability = cap.capability;
  authMw = (await import('../app/middleware/auth')).default;
});

beforeEach(() => {
  setActivePinia(createPinia());
  (globalThis as any).navigateTo.mockClear?.();
});

describe('useAuthStore URL rules', () => {
  it('denies nothing when ui_rules empty', () => {
    const store = useAuthStore();
    store.user = userProfile();
    expect(store.canUrl('/admin')).toBe(true);
    expect(store.canUrl('/entities/x')).toBe(true);
  });

  it('denies prefix matches including nested paths', () => {
    const store = useAuthStore();
    store.user = userProfile({
      ui_rules: { rules: [{ path: '/admin', allow: false }], blanket: false },
    });
    expect(store.canUrl('/admin')).toBe(false);
    expect(store.canUrl('/admin/users')).toBe(false);
    expect(store.canUrl('/administrator')).toBe(true); // prefix boundary
    expect(store.canUrl('/entities')).toBe(true);
  });

  it('blanket deny blocks everything outside allow rules', () => {
    const store = useAuthStore();
    store.user = userProfile({
      ui_rules: { rules: [{ path: '/entities', allow: true }], blanket: true },
    });
    expect(store.canUrl('/')).toBe(false);
    expect(store.canUrl('/entities/x')).toBe(true);
    expect(store.canUrl('/reports')).toBe(false);
  });

  it('longest prefix wins: allow inside deny carves a hole', () => {
    const store = useAuthStore();
    store.user = userProfile({
      ui_rules: {
        rules: [
          { path: '/admin', allow: false },
          { path: '/admin/docs', allow: true },
        ],
        blanket: false,
      },
    });
    expect(store.canUrl('/admin')).toBe(false);
    expect(store.canUrl('/admin/users')).toBe(false);
    expect(store.canUrl('/admin/docs')).toBe(true);
    expect(store.canUrl('/admin/docs/x')).toBe(true);
  });

  it('longest prefix wins: deny inside allowlist entry', () => {
    const store = useAuthStore();
    store.user = userProfile({
      ui_rules: {
        rules: [
          { path: '/entities/secret', allow: false },
          { path: '/', allow: true },
          { path: '/entities', allow: true },
        ],
        blanket: true,
      },
    });
    expect(store.canUrl('/entities/secret')).toBe(false);
    expect(store.canUrl('/entities/secret/x')).toBe(false);
    expect(store.canUrl('/entities/ok')).toBe(true);
    expect(store.canUrl('/')).toBe(true);
  });

  it("allow rule on '/' allows every path", () => {
    const store = useAuthStore();
    store.user = userProfile({
      ui_rules: { rules: [{ path: '/', allow: true }], blanket: true },
    });
    expect(store.canUrl('/reports')).toBe(true);
    expect(store.canUrl('/admin')).toBe(true);
  });
});

describe('menu access decision', () => {
  it('grants when no capability is required and URL is allowed', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: [] });
    expect(store.canAccessMenuItem({ to: '/docs' })).toBe(true);
  });

  it('hides a menu item without the required capability', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['entity.read'] });
    expect(
      store.canAccessMenuItem({ to: '/audit', capability: 'audit.read' }),
    ).toBe(false);
  });

  it('hides a menu item denied by a URL rule even with the capability', () => {
    const store = useAuthStore();
    store.user = userProfile({
      capabilities: ['audit.read'],
      ui_rules: { rules: [{ path: '/audit', allow: false }], blanket: false },
    });
    expect(
      store.canAccessMenuItem({ to: '/audit', capability: 'audit.read' }),
    ).toBe(false);
  });

  it('grants when the capability is present and the URL allowed', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['audit.read'] });
    expect(
      store.canAccessMenuItem({ to: '/audit', capability: 'audit.read' }),
    ).toBe(true);
  });

  it('any-of capabilities: umbrella manage capability unlocks the item', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['users.manage'] });
    expect(
      store.canAccessMenuItem({
        to: '/admin/users',
        capabilities: ['user.read', 'users.manage'],
      }),
    ).toBe(true);
  });

  it('any-of capabilities: denies when none of them is granted', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['entity.read'] });
    expect(
      store.canAccessMenuItem({
        to: '/admin/users',
        capabilities: ['user.read', 'users.manage'],
      }),
    ).toBe(false);
  });

  it('capability + capabilities union: single required entry is enough', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['security.manage'] });
    expect(
      store.canAccessMenuItem({
        to: '/admin/dump',
        capability: 'security.manage',
      }),
    ).toBe(true);
  });
});

describe('capability middleware', () => {
  const to = { path: '/x', fullPath: '/x' };

  it('allows when the single capability is present', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['entity.read'] });
    const mw = capability('entity.read');
    expect(mw(to, {})).toBeUndefined();
  });

  it('throws 403 when the capability is missing', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['entity.read'] });
    const mw = capability('users.manage');
    expect(() => mw(to, {})).toThrowError(expect.objectContaining({ statusCode: 403 }));
  });

  it('any-of: allows when any listed capability is present', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['user.read'] });
    const mw = capability(['users.manage', 'user.read']);
    expect(mw(to, {})).toBeUndefined();
  });

  it('any-of: denies when none present', () => {
    const store = useAuthStore();
    store.user = userProfile({ capabilities: ['entity.read'] });
    const mw = capability(['users.manage', 'user.read']);
    expect(() => mw(to, {})).toThrowError(expect.objectContaining({ statusCode: 403 }));
  });
});

describe('auth middleware URL enforcement', () => {
  it('blocks rendering of a denied URL with 403', async () => {
    const store = useAuthStore();
    store.user = userProfile({
      ui_rules: { rules: [{ path: '/admin', allow: false }], blanket: false },
    });
    await expect(authMw({ path: '/admin', fullPath: '/admin' })).rejects
      .toMatchObject({ statusCode: 403 });
  });

  it('passes through allowed URLs', async () => {
    const store = useAuthStore();
    store.user = userProfile();
    await expect(
      authMw({ path: '/entities/x', fullPath: '/entities/x' }),
    ).resolves.toBeUndefined();
  });

  it('skips public paths entirely', async () => {
    const store = useAuthStore();
    await expect(authMw({ path: '/login', fullPath: '/login' })).resolves.toBeUndefined();
  });
});
