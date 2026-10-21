<template>
  <div class="p-4 sm:p-6 space-y-4 max-w-4xl">
    <div>
      <h1 class="text-2xl font-bold">{{ t('entity.search') }}</h1>
      <UInput
        v-model="input"
        :placeholder="t('entity.search_placeholder')"
        icon="i-lucide-search"
        class="mt-3 w-full max-w-md"
        autofocus
        @keyup.enter="applySearch"
      />
      <p v-if="q" class="mt-2 text-[var(--ui-text-muted)]">«{{ q }}»</p>
    </div>

    <div v-if="loading" class="py-12 text-center text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>
    <div v-else-if="error" class="py-12 text-center">
      <p class="mb-3 text-red-500">{{ error }}</p>
      <UButton variant="outline" @click="runSearch">{{ t('common.retry') }}</UButton>
    </div>
    <div v-else-if="results.length === 0" class="py-12 text-center text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>

    <div v-else class="space-y-2">
      <NuxtLink
        v-for="row in results"
        :key="row.public_id"
        :to="`/entities/${row.public_id}`"
        class="block"
      >
        <UCard class="hover:border-[var(--ui-primary)] transition-colors">
          <div class="flex items-center justify-between gap-3">
            <div class="min-w-0">
              <p class="font-medium truncate">{{ row.display_name ?? row.public_id }}</p>
              <p class="text-xs text-[var(--ui-text-muted)] font-mono truncate">
                {{ row.type_id }}<template v-if="row.catalog"> · {{ row.catalog }}</template>
              </p>
            </div>
            <UBadge variant="subtle" size="sm">{{ row.status }}</UBadge>
          </div>
        </UCard>
      </NuxtLink>
      <div v-if="nextCursor" class="flex justify-center pt-2">
        <UButton variant="outline" :loading="loadingMore" @click="loadMore">
          {{ t('entity.load_more') }}
        </UButton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { capability } from '~/middleware/capability';

definePageMeta({ middleware: ['auth', capability('entity.read')] });

const { t, te } = useI18n();
const route = useRoute();
const toast = useToast();
const api = useApi();
const { catalogParams, selected } = useCatalog();

interface SearchRow {
  public_id: string;
  display_name?: string;
  type_id?: string;
  catalog?: string;
  status?: string;
}

const q = computed(() => (route.query.q as string) ?? '');
const input = ref(q.value);
const results = ref<SearchRow[]>([]);
const loading = ref(false);
const loadingMore = ref(false);
const error = ref<string | null>(null);
const nextCursor = ref<string | null>(null);

function applySearch() {
  navigateTo({ path: '/search', query: input.value.trim() ? { q: input.value.trim() } : {} });
}

async function runSearch() {
  if (!q.value.trim()) return;
  loading.value = true;
  error.value = null;
  try {
    const res = await api.entities.list({
      search: q.value.trim(),
      limit: 50,
      catalog: catalogParams().catalog || undefined,
    });
    results.value = res.data as unknown as SearchRow[];
    nextCursor.value = (res.meta as { next_cursor?: string } | undefined)?.next_cursor ?? null;
  } catch (e: unknown) {
    error.value = displayError(e, t, te);
    toast.add({ title: t('common.error'), description: error.value, color: 'error' });
  } finally {
    loading.value = false;
  }
}

async function loadMore() {
  if (!nextCursor.value) return;
  loadingMore.value = true;
  error.value = null;
  try {
    const res = await api.entities.list({
      search: q.value.trim(),
      limit: 50,
      cursor: nextCursor.value,
      catalog: catalogParams().catalog || undefined,
    });
    results.value.push(...(res.data as unknown as SearchRow[]));
    nextCursor.value = (res.meta as { next_cursor?: string } | undefined)?.next_cursor ?? null;
  } catch (e: unknown) {
    error.value = displayError(e, t, te);
    toast.add({ title: t('common.error'), description: error.value, color: 'error' });
  } finally {
    loadingMore.value = false;
  }
}

watch(
  () => route.query.q,
  () => {
    input.value = q.value;
    runSearch();
  },
  { immediate: true },
);
// Header catalog switch re-scopes the current query.
watch(selected, runSearch);
</script>
