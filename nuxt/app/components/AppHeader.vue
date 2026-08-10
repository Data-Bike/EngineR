<template>
  <header
    class="flex items-center justify-between border-b border-[var(--ui-border)] px-4 py-3"
    role="banner"
  >
    <!-- Left: product name + workspace -->
    <div class="flex items-center gap-3">
      <UButton
        icon="i-lucide-menu"
        color="neutral"
        variant="ghost"
        size="sm"
        :aria-label="t('nav.objects')"
        @click="toggleSidebar"
      />
      <NuxtLink to="/" class="text-lg font-bold text-[var(--ui-text)]">
        {{ t('app.name') }}
      </NuxtLink>
    </div>

    <!-- Center: global search (docs/16 §9) -->
    <div class="hidden md:flex flex-1 max-w-md mx-4">
      <UInput
        v-model="searchQuery"
        :placeholder="t('entity.search')"
        icon="i-lucide-search"
        class="w-full"
        @keyup.enter="doSearch"
      />
    </div>

    <!-- Right: locale, profile, notifications -->
    <div class="flex items-center gap-2">
      <!-- Locale selector -->
      <USelect
        v-model="currentLocale"
        :items="localeOptions"
        size="sm"
        @update:model-value="changeLocale"
      />

      <!-- Profile dropdown -->
      <UDropdownMenu :options="profileItems">
        <UButton
          color="neutral"
          variant="ghost"
          size="sm"
          icon="i-lucide-user"
          :label="user?.display_name ?? t('auth.login')"
        />
      </UDropdownMenu>
    </div>
  </header>
</template>

<script setup lang="ts">
const { t, locale, locales, setLocale } = useI18n();
const { user, isAuthenticated, logout } = useAuth();
const searchQuery = ref('');

const emit = defineEmits<{ 'toggle-sidebar': [] }>();

function toggleSidebar() {
  emit('toggle-sidebar');
}

const currentLocale = computed({
  get: () => locale.value,
  set: (v: string) => v,
});

const localeOptions = computed(() =>
  (locales.value as Array<{ code: string; name: string; dir?: string }>).map((l) => ({
    label: l.name,
    value: l.code,
  })),
);

async function changeLocale(code: string) {
  await setLocale(code as 'ru' | 'en' | 'ar' | 'he' | 'fr' | 'de' | 'es' | 'zh');
  // Set <html dir> for RTL support (docs/16 §12, docs/19)
  const dir = (locales.value as Array<{ code: string; dir?: string }>).find(
    (l) => l.code === code,
  )?.dir ?? 'ltr';
  if (import.meta.client) {
    document.documentElement.dir = dir;
  }
}

function doSearch() {
  if (!searchQuery.value.trim()) return;
  navigateTo({
    path: '/search',
    query: { q: searchQuery.value },
  });
}

const profileItems = computed(() => {
  if (isAuthenticated.value) {
    return [
      { label: user.value?.display_name ?? '', disabled: true },
      { label: t('auth.logout'), onSelect: () => doLogout() },
    ];
  }
  return [{ label: t('auth.login'), to: '/login' }];
});

async function doLogout() {
  await logout();
  await navigateTo('/login');
}
</script>
