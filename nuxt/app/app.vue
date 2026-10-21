<template>
  <UApp>
    <NuxtLayout>
      <NuxtPage />
    </NuxtLayout>
  </UApp>
</template>

<script setup lang="ts">
// Global setup: fetch session context on app load, but skip on public pages
const { fetchContext, isAuthenticated } = useAuth();
const route = useRoute();
const publicPaths = ['/login', '/health'];

// Keep <html lang> and <html dir> in sync with the active locale — on
// first paint (persisted preference) and on every switch (docs/16 §12,
// docs/19).
const { locale, locales } = useI18n();
const applyHtmlLocale = (code: string) => {
  const dir = (locales.value as Array<{ code: string; dir?: string }>).find(
    (l) => l.code === code,
  )?.dir ?? 'ltr';
  document.documentElement.lang = code;
  document.documentElement.dir = dir;
};

onMounted(() => {
  applyHtmlLocale(locale.value);
  watch(locale, applyHtmlLocale);
  if (!isAuthenticated.value && !publicPaths.includes(route.path)) {
    fetchContext();
  }
});
</script>
