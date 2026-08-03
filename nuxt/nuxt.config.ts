// https://nuxt.com/docs/api/configuration/nuxt-config
import { resolve } from 'node:path';

export default defineNuxtConfig({
  compatibilityDate: '2025-01-01',
  future: {
    compatibilityVersion: 4,
  },

  // SPA mode for authenticated intranet/admin (docs/16 §2).
  // Public help/login pages MAY use SSR with separate cache policy.
  ssr: false,

  devtools: { enabled: true },

  // Components — disable path prefix so subdirectory components
  // (entity/FormField.vue, graph/GraphView.vue, etc.) are available
  // by their file name without the directory prefix.
  components: [
    { path: '~/components', pathPrefix: false },
  ],

  modules: [
    '@nuxt/ui',
    '@nuxtjs/i18n',
    '@pinia/nuxt',
  ],

  // CSS — Nuxt UI 4 ships Tailwind v4
  css: ['~/assets/css/main.css'],

  // Alias for the typed API client (docs/16 §4)
  alias: {
    '@enginer/api-client': resolve(__dirname, '../web/src/index.ts'),
  },

  // i18n configuration (docs/16, docs/19)
  // 8 locales: ru (default), en, ar (RTL), he (RTL), fr, de, es, zh
  i18n: {
    defaultLocale: 'ru',
    strategy: 'prefix_except_default',
    locales: [
      { code: 'ru', name: 'Русский', file: 'ru.ts', dir: 'ltr' },
      { code: 'en', name: 'English', file: 'en.ts', dir: 'ltr' },
      { code: 'ar', name: 'العربية', file: 'ar.ts', dir: 'rtl' },
      { code: 'he', name: 'עברית', file: 'he.ts', dir: 'rtl' },
      { code: 'fr', name: 'Français', file: 'fr.ts', dir: 'ltr' },
      { code: 'de', name: 'Deutsch', file: 'de.ts', dir: 'ltr' },
      { code: 'es', name: 'Español', file: 'es.ts', dir: 'ltr' },
      { code: 'zh', name: '中文', file: 'zh.ts', dir: 'ltr' },
    ],
    langDir: '../i18n/locales',
    detectBrowserLanguage: {
      useCookie: true,
      cookieKey: 'enginer_locale',
      redirectOn: 'root',
    },
  },

  // Runtime config — public values only (docs/16 §13)
  // apiBaseUrl is relative (/api/v1) so the browser makes same-origin
  // requests to the Nuxt server, which proxies them to the API (nitro
  // server route in server/routes/api/v1/[...].ts). This avoids CORS.
  runtimeConfig: {
    public: {
      apiBaseUrl: process.env.NUXT_PUBLIC_API_BASE_URL ?? '/api/v1',
    },
    // Internal: API URL for server-side proxy (set via ENGINER_API_URL env)
    apiInternalUrl: process.env.ENGINER_API_URL ?? 'http://localhost:8010',
  },

  // TypeScript strict (docs/16 §1)
  typescript: {
    strict: true,
    typeCheck: false, // run via `npm run typecheck` separately
  },

  // Security headers (docs/16 §13)
  app: {
    head: {
      title: 'EngineR',
      meta: [
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { name: 'X-Frame-Options', content: 'DENY' },
      ],
    },
  },
});
