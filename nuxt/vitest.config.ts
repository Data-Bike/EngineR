import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [
    vue(),
    // Inject Nuxt auto-imports into SFC <script setup> blocks
    {
      name: 'nuxt-auto-import-injector',
      transform(code, id) {
        if (id.endsWith('.vue')) {
          // Prepend imports that Nuxt would auto-import
          const preamble = `
import { useI18n as __useI18n } from 'vue-i18n';
const useI18n = __useI18n;
const useApi = globalThis.__useApi;
const useCapability = globalThis.__useCapability;
const navigateTo = globalThis.__navigateTo;
const definePageMeta = globalThis.__definePageMeta;
const onBeforeRouteLeave = globalThis.__onBeforeRouteLeave;
const useRoute = globalThis.__useRoute;
const useAuth = globalThis.__useAuth;
`;
          // Insert after the last import statement or at the start of <script setup>
          // The Vue compiler wraps <script setup> content; we inject at the top
          return code.replace(
            /(import\s+.*?from\s+['"][^'"]+['"];?\n)/,
            `$1${preamble}`,
          );
        }
        return code;
      },
    },
  ],
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.test.ts'],
    setupFiles: ['tests/setup.ts'],
  },
});
