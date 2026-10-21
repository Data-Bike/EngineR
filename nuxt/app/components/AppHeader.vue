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
        :aria-label="t('nav.toggle_sidebar')"
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

    <!-- Right: catalog, locale, profile, notifications -->
    <div class="flex items-center gap-2 flex-wrap justify-end">
      <!-- Mobile search entry (the input itself is md+ only) -->
      <UButton
        icon="i-lucide-search"
        color="neutral"
        variant="ghost"
        size="sm"
        class="md:hidden"
        :aria-label="t('entity.search')"
        to="/search"
      />

      <!-- Catalog selector (docs/28 §6): shown when the principal can read
           more than one catalog. '' = all accessible catalogs. -->
      <USelect
        v-if="hasMultiple"
        v-model="catalogChoice"
        :items="catalogOptions"
        size="sm"
        icon="i-lucide-layers"
        class="w-28 sm:w-44"
        :aria-label="t('catalog.selector')"
      />

      <!-- Locale selector -->
      <USelect
        :model-value="locale"
        :items="localeOptions"
        size="sm"
        @update:model-value="changeLocale"
      />

      <!-- Profile dropdown -->
      <UDropdownMenu :items="profileItems">
        <UButton
          color="neutral"
          variant="ghost"
          size="sm"
          icon="i-lucide-user"
          :label="user?.display_name ?? t('auth.login')"
          class="max-w-32 sm:max-w-48 truncate"
        />
      </UDropdownMenu>
    </div>
  </header>
</template>

<script setup lang="ts">
const { t, locale, locales, setLocale } = useI18n();
const { user, isAuthenticated, logout } = useAuth();
const { available, selected, hasMultiple, select } = useCatalog();
const searchQuery = ref('');

// Reka SelectItem rejects empty-string values — '' is the "clear" state.
// The sentinel stands for the empty selection (all accessible catalogs);
// catalog aliases must start with a latin letter, so '__all__' cannot
// collide.
const ALL_CATALOGS = '__all__';
const catalogChoice = computed({
  get: () => selected.value || ALL_CATALOGS,
  set: (v: string) => select(v === ALL_CATALOGS ? '' : (v ?? '')),
});
const catalogOptions = computed(() => [
  { label: t('catalog.all_catalogs'), value: ALL_CATALOGS },
  ...available.value.map((c) => ({ label: c.name || c.alias, value: c.alias })),
]);

const emit = defineEmits<{ 'toggle-sidebar': [] }>();

function toggleSidebar() {
  emit('toggle-sidebar');
}

const localeOptions = computed(() =>
  (locales.value as Array<{ code: string; name: string; dir?: string }>).map((l) => ({
    label: l.name,
    value: l.code,
  })),
);

async function changeLocale(code: string) {
  // <html lang>/<html dir> are synced by plugins/html-locale.client.ts.
  await setLocale(code as 'ru' | 'en' | 'ar' | 'he' | 'fr' | 'de' | 'es' | 'zh');
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
