/**
 * useApi — provides the typed EngineR API client to components.
 *
 * Components never call $fetch directly (docs/16 §4). They go through this
 * composable which wraps `@enginer/api-client` with the Nuxt runtime config
 * for base URL, locale, and timezone.
 */

import { createEnginerClient, type EnginerClient } from '@enginer/api-client';

export function useApi(): EnginerClient {
  const config = useRuntimeConfig();
  const { locale } = useI18n();
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

  return createEnginerClient({
    baseUrl: config.public.apiBaseUrl as string,
    locale: locale.value,
    timezone,
    onRefresh: async () => {
      await navigateTo('/login');
    },
  });
}
