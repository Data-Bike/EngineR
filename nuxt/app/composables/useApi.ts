/**
 * useApi — provides the typed EngineR API client to components.
 *
 * Components never call $fetch directly (docs/16 §4). They go through this
 * composable which wraps `@enginer/api-client` with the Nuxt runtime config
 * for base URL, locale, and timezone.
 */

import { createEnginerClient, type EnginerClient } from '@enginer/api-client';
import { useAuthStore } from './useAuth';

export function useApi(): EnginerClient {
  const config = useRuntimeConfig();
  const { locale } = useI18n();
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const authStore = useAuthStore();

  return createEnginerClient({
    baseUrl: (config.public.apiBaseUrl as string) || '/api/v1',
    locale: locale.value,
    timezone,
    csrfToken: authStore.user?.csrf_token ?? (config.public.apiCsrfToken as string) ?? '',
    onRefresh: async () => {
      // The session is gone: clear the local profile *before* navigating,
      // otherwise /login mounts while isAuthenticated is still true and
      // renders the default layout (sidebar + stale profile chip).
      authStore.user = null;
      authStore.error = null;
      const route = useRoute();
      await navigateTo({ path: '/login', query: { redirect: route.fullPath } });
    },
  });
}
