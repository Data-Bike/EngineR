/**
 * Vitest setup: mock vue-i18n and stub Nuxt auto-import globals.
 */

import { vi } from 'vitest';
import { ref } from 'vue';

const tMock = (key: string, params?: Record<string, unknown>) => {
  if (params && 'count' in params) return `${key}:${params.count}`;
  return key;
};

const i18nInstance = {
  t: tMock,
  te: (key: string) => !!key,
  locale: { value: 'en' },
  locales: { value: [{ code: 'en', name: 'English', dir: 'ltr' }] },
  setLocale: vi.fn(),
};

// Mock vue-i18n for explicit `import { useI18n } from 'vue-i18n'`
vi.mock('vue-i18n', () => ({
  useI18n: () => i18nInstance,
  createI18n: vi.fn(() => ({})),
}));

// Stub Nuxt auto-imports as globals
;(globalThis as any).useApi = () => ({
  entities: {
    query: vi.fn().mockResolvedValue({ data: { rows: [], cursor: undefined } }),
    get: vi.fn(),
    retire: vi.fn(),
  },
  schemas: {
    listTypes: vi.fn(),
    createDraft: vi.fn(),
    validateDraft: vi.fn(),
    impactDraft: vi.fn(),
    publishDraft: vi.fn(),
    retireRevision: vi.fn(),
  },
});
;(globalThis as any).__useApi = (...args: unknown[]) => (globalThis as any).useApi(...args);
;(globalThis as any).useCapability = () => ({
  requireCapability: vi.fn().mockReturnValue(true),
  hasCapability: vi.fn().mockReturnValue(true),
});
;(globalThis as any).__useCapability = (...args: unknown[]) => (globalThis as any).useCapability(...args);
;(globalThis as any).navigateTo = vi.fn();
;(globalThis as any).__navigateTo = (...args: unknown[]) => (globalThis as any).navigateTo(...args);
;(globalThis as any).definePageMeta = vi.fn();
;(globalThis as any).__definePageMeta = (...args: unknown[]) => (globalThis as any).definePageMeta(...args);
;(globalThis as any).onBeforeRouteLeave = vi.fn();
;(globalThis as any).__onBeforeRouteLeave = (...args: unknown[]) => (globalThis as any).onBeforeRouteLeave(...args);
;(globalThis as any).useRoute = () => ({ params: {}, query: {} });
;(globalThis as any).__useRoute = (...args: unknown[]) => (globalThis as any).useRoute(...args);
// Router stub: resolve returns empty `matched` for paths listed in
// globalThis.__routerDeny (defaults to /entities — no index route exists).
;(globalThis as any).__routerDeny = ['/entities'];
;(globalThis as any).useRouter = () => ({
  resolve: (p: string) => ({
    matched: (globalThis as any).__routerDeny.includes(p) ? [] : [{}],
  }),
});
;(globalThis as any).__useRouter = (...args: unknown[]) => (globalThis as any).useRouter(...args);
;(globalThis as any).useAuth = () => ({
  isAuthenticated: { value: false },
  fetchContext: vi.fn(),
  hasCapability: vi.fn().mockReturnValue(true),
  canUrl: vi.fn().mockReturnValue(true),
  canAccessMenuItem: vi.fn().mockReturnValue(true),
});
;(globalThis as any).__useAuth = (...args: unknown[]) => (globalThis as any).useAuth(...args);
;(globalThis as any).useToast = () => ({ add: vi.fn() });
;(globalThis as any).__useToast = (...args: unknown[]) => (globalThis as any).useToast(...args);
// Catalog selection composable (docs/28 §6). Per-test overrides replace
// globalThis.useCatalog before mounting a component.
// Real refs — pages `watch(selected)` on catalog switch; plain objects
// would make Vue warn "Invalid watch source" and silently not fire.
;(globalThis as any).useCatalog = () => ({
  catalogParams: () => ({}),
  selected: ref(''),
  available: ref([]),
  hasMultiple: ref(false),
  select: vi.fn(),
});
;(globalThis as any).__useCatalog = (...args: unknown[]) => (globalThis as any).useCatalog(...args);
;(globalThis as any).useAuthStore = () => ({ hasCapability: vi.fn().mockReturnValue(true) });
;(globalThis as any).__useAuthStore = (...args: unknown[]) => (globalThis as any).useAuthStore(...args);
;(globalThis as any).defineNuxtRouteMiddleware = (fn: unknown) => fn;
;(globalThis as any).createError = (e: unknown) => e;
