<template>
  <div class="p-6 space-y-4">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ typeAlias ?? t('nav.objects') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('entity.search') }}</p>
      </div>
      <UButton
        icon="i-lucide-plus"
        @click="showCreate = true"
      >
        {{ t('entity.create') }}
      </UButton>
    </div>

    <!-- Search bar -->
    <div class="flex gap-2">
      <UInput
        v-model="searchQuery"
        :placeholder="t('entity.search')"
        icon="i-lucide-search"
        class="flex-1"
        @keyup.enter="doSearch"
      />
      <UButton
        variant="outline"
        icon="i-lucide-sliders-horizontal"
        @click="showAdvanced = !showAdvanced"
      >
        {{ t('entity.advanced_search') }}
      </UButton>
    </div>

    <!-- Results table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('entity.loading') }}
      </div>
      <div v-else-if="error" class="py-8 text-center text-red-500">
        {{ error }}
        <UButton size="sm" variant="outline" @click="doSearch" class="ml-2">
          {{ t('common.retry') }}
        </UButton>
      </div>
      <div v-else-if="rows.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('entity.empty') }}
      </div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th
              v-for="col in columns"
              :key="col.alias"
              class="py-2 px-3 font-medium text-[var(--ui-text-muted)]"
            >
              {{ col.alias }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.public_id"
            class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)] cursor-pointer"
            @click="openEntity(row.public_id)"
          >
            <td
              v-for="col in columns"
              :key="col.alias"
              class="py-2 px-3"
            >
              {{ formatValue(row[col.alias]) }}
            </td>
          </tr>
        </tbody>
      </table>

      <!-- Cursor pagination -->
      <div v-if="cursor" class="mt-4 flex justify-center">
        <UButton variant="outline" @click="loadMore" :loading="loadingMore">
          {{ t('common.retry') }}
        </UButton>
      </div>
    </UCard>

    <!-- Create dialog placeholder -->
    <UModal v-model:open="showCreate">
      <template #content>
        <div class="p-6">
          <h2 class="text-lg font-bold mb-4">{{ t('entity.create') }}</h2>
          <p class="text-[var(--ui-text-muted)]">EntityFormRenderer — TBD</p>
          <div class="flex justify-end gap-2 mt-4">
            <UButton variant="outline" @click="showCreate = false">
              {{ t('common.cancel') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
definePageMeta({
  middleware: 'auth',
});

const { t } = useI18n();
const route = useRoute();
const api = useApi();

const typeAlias = computed(() => route.params.typeAlias as string | undefined);
const searchQuery = ref('');
const showAdvanced = ref(false);
const showCreate = ref(false);
const loading = ref(false);
const loadingMore = ref(false);
const error = ref<string | null>(null);

interface ResultRow {
  public_id: string;
  [key: string]: unknown;
}

const rows = ref<ResultRow[]>([]);
const columns = ref<{ alias: string }[]>([{ alias: 'public_id' }]);
const cursor = ref<string | undefined>();

async function doSearch() {
  loading.value = true;
  error.value = null;
  try {
    const res = await api.entities.query({
      query: {
        from: { type_id: typeAlias.value ?? 'unknown', alias: 'e' },
        select: [{ field: { system: 'public_id' }, alias: 'public_id' }],
        sort: [{ field: { system: 'public_id' }, direction: 'asc' }],
        page: { size: 50 },
      },
    });
    rows.value = res.data.rows as ResultRow[];
    cursor.value = res.data.cursor;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : t('entity.error');
  } finally {
    loading.value = false;
  }
}

async function loadMore() {
  if (!cursor.value) return;
  loadingMore.value = true;
  try {
    const res = await api.entities.query({
      query: {
        from: { type_id: typeAlias.value ?? 'unknown', alias: 'e' },
        select: [{ field: { system: 'public_id' }, alias: 'public_id' }],
        sort: [{ field: { system: 'public_id' }, direction: 'asc' }],
        page: { size: 50, after: cursor.value },
      },
    });
    rows.value.push(...(res.data.rows as ResultRow[]));
    cursor.value = res.data.cursor;
  } finally {
    loadingMore.value = false;
  }
}

function openEntity(publicId: string) {
  navigateTo(`/entities/${publicId}`);
}

function formatValue(v: unknown): string {
  if (v === null || v === undefined) return '';
  if (typeof v === 'string') return v;
  return String(v);
}

onMounted(doSearch);
</script>
