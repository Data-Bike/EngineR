/**
 * Vitest setup: mock vue-i18n and stub Nuxt auto-import globals.
 */

import { vi } from 'vitest';

const tMock = (key: string, params?: Record<string, unknown>) => {
  if (params && 'count' in params) return `${key}:${params.count}`;
  return key;
};

const i18nInstance = {
  t: tMock,
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
;(globalThis as any).useCapability = () => ({
  requireCapability: vi.fn().mockReturnValue(true),
  hasCapability: vi.fn().mockReturnValue(true),
});
;(globalThis as any).navigateTo = vi.fn();
;(globalThis as any).definePageMeta = vi.fn();
;(globalThis as any).onBeforeRouteLeave = vi.fn();
;(globalThis as any).useRoute = () => ({ params: {}, query: {} });
;(globalThis as any).useAuth = () => ({
  isAuthenticated: { value: false },
  fetchContext: vi.fn(),
});
