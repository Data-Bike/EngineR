import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  resolve: {
    alias: {
      '@enginer/api-client': fileURLToPath(new URL('../web/src/index.ts', import.meta.url)),
      '~': fileURLToPath(new URL('./app', import.meta.url)),
      '~~': fileURLToPath(new URL('.', import.meta.url)),
    },
  },
  plugins: [
    vue(),
    // Inject Nuxt auto-imports into SFC <script setup> blocks
    {
      name: 'nuxt-auto-import-injector',
      transform(code, id) {
        if (id.endsWith('.vue')) {
          // Prepend imports that Nuxt would auto-import. Skip any name the
          // SFC imports itself — a duplicate const declaration is a syntax
          // error.
          const autoImports: [string, string][] = [
            ['useI18n', `import { useI18n as __useI18n } from 'vue-i18n'; const useI18n = __useI18n;`],
            ['ref', `import { ref as __ref } from 'vue'; const ref = __ref;`],
            ['computed', `import { computed as __computed } from 'vue'; const computed = __computed;`],
            ['watch', `import { watch as __watch } from 'vue'; const watch = __watch;`],
            ['reactive', `import { reactive as __reactive } from 'vue'; const reactive = __reactive;`],
            ['onMounted', `import { onMounted as __onMounted } from 'vue'; const onMounted = __onMounted;`],
            ['onUnmounted', `import { onUnmounted as __onUnmounted } from 'vue'; const onUnmounted = __onUnmounted;`],
            ['useApi', `const useApi = globalThis.__useApi;`],
            ['useCapability', `const useCapability = globalThis.__useCapability;`],
            ['navigateTo', `const navigateTo = globalThis.__navigateTo;`],
            ['definePageMeta', `const definePageMeta = globalThis.__definePageMeta;`],
            ['onBeforeRouteLeave', `const onBeforeRouteLeave = globalThis.__onBeforeRouteLeave;`],
            ['useRoute', `const useRoute = globalThis.__useRoute;`],
            ['useRouter', `const useRouter = globalThis.__useRouter;`],
            ['useLocalePath', `const useLocalePath = globalThis.__useLocalePath;`],
            ['useAuth', `const useAuth = globalThis.__useAuth;`],
            ['useToast', `const useToast = globalThis.__useToast;`],
            ['useCatalog', `const useCatalog = globalThis.__useCatalog;`],
            ['useAuthStore', `const useAuthStore = globalThis.__useAuthStore;`],
          ];
          // Names already imported by the SFC must not be redeclared —
          // check them only inside the import statements themselves.
          const importRegion =
            code.match(/import\s[\s\S]*?from\s*['"][^'"]+['"]/g)?.join('\n') ?? '';
          const preamble = autoImports
            .filter(([name]) => !new RegExp(`\\b${name}\\b`).test(importRegion))
            .map(([, line]) => line)
            .join('\n');
          // Insert after the first import statement; files without imports
          // get the preamble at the top.
          if (/(import\s+.*?from\s+['"][^'"]+['"];?\n)/.test(code)) {
            return code.replace(
              /(import\s+.*?from\s+['"][^'"]+['"];?\n)/,
              `$1\n${preamble}\n`,
            );
          }
          return `${preamble}\n${code}`;
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
