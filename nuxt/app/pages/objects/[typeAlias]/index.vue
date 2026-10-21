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
        :to="{ path: `/objects/${typeAlias}/create`, query: route.query.catalog ? { catalog: route.query.catalog } : {} }"
      >
        {{ t('entity.create') }}
      </UButton>
    </div>

    <!-- Search bar -->
    <div class="flex gap-2">
      <UInput
        v-model="searchQuery"
        :placeholder="t('entity.search_placeholder', 'Поиск по названию, ИНН, ОГРН...')"
        icon="i-lucide-search"
        class="flex-1"
        @keyup.enter="doSearch"
      />
      <UButton
        variant="outline"
        icon="i-lucide-search"
        @click="doSearch"
      >
        {{ t('entity.search_btn', 'Найти') }}
      </UButton>
      <UButton
        variant="outline"
        icon="i-lucide-sliders-horizontal"
        @click="showAdvanced = !showAdvanced"
      >
        {{ t('entity.advanced_search') }}
      </UButton>
    </div>

    <!-- Advanced search -->
    <div v-if="showAdvanced" class="border border-[var(--ui-border)] rounded-md p-4 space-y-3">
      <p class="font-medium text-sm">{{ t('entity.advanced_search') }}</p>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <UFormField v-for="col in searchableColumns" :key="col.alias" :label="columnLabel(col)">
          <UInput v-model="advancedFilters[col.alias]" :placeholder="columnLabel(col)" class="w-full" />
        </UFormField>
      </div>
      <div class="flex justify-end gap-2">
        <UButton variant="outline" size="sm" @click="clearAdvanced">{{ t('common.reset', 'Сбросить') }}</UButton>
        <UButton size="sm" @click="doSearch">{{ t('entity.search_btn', 'Найти') }}</UButton>
      </div>
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
      <div v-else class="overflow-x-auto"><table class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th
              v-for="col in columns"
              :key="col.alias"
              class="py-2 px-3 font-medium text-[var(--ui-text-muted)]"
            >
              {{ columnLabel(col) }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.public_id"
            class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)] cursor-pointer"
            tabindex="0"
            @click="openEntity(row.public_id)"
            @keydown.enter="openEntity(row.public_id)"
          >
            <td
              v-for="(col, colIdx) in columns"
              :key="col.alias"
              class="py-2 px-3 max-w-48 truncate"
              :title="formatValue(row[col.alias], col.alias)"
            >
              <NuxtLink
                v-if="colIdx === 0"
                :to="`/entities/${row.public_id}`"
                class="text-[var(--ui-primary)] hover:underline"
                @click.stop
              >
                {{ formatValue(row[col.alias], col.alias) }}
              </NuxtLink>
              <template v-else>{{ formatValue(row[col.alias], col.alias) }}</template>
            </td>
          </tr>
        </tbody>
      </table></div>

      <!-- Cursor pagination -->
      <div v-if="cursor" class="mt-4 flex justify-center">
        <UButton variant="outline" @click="loadMore" :loading="loadingMore">
          {{ t('entity.load_more') }}
        </UButton>
      </div>
    </UCard>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import type { FieldRef, SystemField } from '@enginer/api-client';
definePageMeta({
  middleware: ['auth', capability('entity.read')],
});

const { t, te } = useI18n();
const route = useRoute();
const toast = useToast();
const api = useApi();
const { catalogParams, selected } = useCatalog();
// A type card carries its own catalog in the route query — an alias may
// exist in several catalogs, so the row's catalog wins over the header
// selection for every call on this page.
const scopedCatalogParams = () => {
  const q = route.query.catalog;
  return typeof q === 'string' && q ? { catalog: q } : catalogParams();
};

const typeAlias = computed(() => route.params.typeAlias as string | undefined);
const searchQuery = ref('');
const showAdvanced = ref(false);
const loading = ref(false);
const loadingMore = ref(false);
const error = ref<string | null>(null);
const advancedFilters = ref<Record<string, string>>({});

interface ResultRow {
  public_id: string;
  [key: string]: unknown;
}

const rows = ref<ResultRow[]>([]);
const columns = ref<{ alias: string; label_key?: string }[]>([
  { alias: 'public_id' },
  { alias: 'display_name' },
  { alias: 'status' },
]);
const cursor = ref<string | undefined>();

const searchableColumns = computed(() => columns.value.filter(c => c.alias !== 'public_id' && c.alias !== 'status'));

function columnLabel(column: { alias: string; label_key?: string }): string {
  if (column.label_key && te(column.label_key)) return t(column.label_key);
  // System fields have dedicated i18n keys rather than raw machine names.
  const systemKeys: Record<string, string> = {
    display_name: 'fields.common.name',
    status: 'requests.col.status',
    created_at: 'requests.col.created',
    type_id: 'fields.common.type',
  };
  const key = systemKeys[column.alias];
  if (key && te(key)) return t(key);
  if (column.alias === 'public_id' || column.alias === 'entity_id') return 'ID';
  // Unknown field: humanize the alias instead of showing a raw key.
  return column.alias.replace(/_/g, ' ').replace(/^./, (c) => c.toUpperCase());
}

function queryField(alias: string): FieldRef {
  const systemFields: SystemField[] = ['public_id', 'entity_id', 'display_name', 'type_id', 'created_at', 'status', 'revision'];
  return systemFields.includes(alias as SystemField)
    ? { system: alias as SystemField }
    : { predicate_id: alias };
}

function clearAdvanced() {
  advancedFilters.value = {};
  searchQuery.value = '';
  doSearch();
}

async function fetchViewColumns() {
  try {
    const res = await api.schemas.getTypeView(typeAlias.value ?? 'unknown', 'list', scopedCatalogParams());
    const data = res.data as { columns?: Array<{ alias: string; label_key?: string }> } | undefined;
    if (data?.columns && data.columns.length > 0) {
      columns.value = data.columns.map((c) => ({ alias: c.alias, label_key: c.label_key }));
    }
  } catch {
    // Keep default columns
  }
}

async function doSearch() {
  loading.value = true;
  error.value = null;
  cursor.value = undefined; // Reset cursor on new search
  rows.value = []; // Clear previous results
  try {
    const filters: Record<string, string> = {};
    for (const [k, v] of Object.entries(advancedFilters.value)) {
      if (v.trim()) filters[k] = v.trim();
    }
    const res = await api.entities.query({
      from: { type_id: typeAlias.value ?? 'unknown', alias: 'e' },
      select: columns.value.map((col) => ({ field: queryField(col.alias), alias: col.alias })),
      search: searchQuery.value || undefined,
      filters: Object.keys(filters).length > 0 ? filters : undefined,
      sort: [{ field: { system: 'display_name' }, direction: 'asc' }],
      page: { size: 10 },
    }, scopedCatalogParams());
    rows.value = res.data.rows || [];
    cursor.value = res.data.cursor;
  } catch (e: unknown) {
    error.value = displayError(e, t, te);
    toast.add({ title: t('common.error'), description: error.value!, color: 'error' });
  } finally {
    loading.value = false;
  }
}

async function loadMore() {
  if (!cursor.value) return;
  loadingMore.value = true;
  error.value = null;
  try {
    const res = await api.entities.query({
      from: { type_id: typeAlias.value ?? 'unknown', alias: 'e' },
      select: columns.value.map((col) => ({ field: queryField(col.alias), alias: col.alias })),
      sort: [{ field: { system: 'display_name' }, direction: 'asc' }],
      page: { size: 10, after: cursor.value },
    }, scopedCatalogParams());
    rows.value.push(...(res.data.rows || []));
    cursor.value = res.data.cursor;
  } catch (e: unknown) {
    error.value = displayError(e, t, te);
    toast.add({ title: t('common.error'), description: error.value!, color: 'error' });
  } finally {
    loadingMore.value = false;
  }
}

function openEntity(publicId: string) {
  navigateTo(`/entities/${publicId}`);
}

function formatValue(v: unknown, alias?: string): string {
  if (v === null || v === undefined) return '';
  if (alias === 'status' && typeof v === 'string' && te(`entity.status.${v}`)) {
    return t(`entity.status.${v}`);
  }
  if (typeof v === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(v)) {
    const d = new Date(v);
    if (!Number.isNaN(d.getTime())) return d.toLocaleString();
  }
  if (typeof v === 'object') return JSON.stringify(v);
  return String(v);
}

onMounted(() => {
  fetchViewColumns();
  doSearch();
});
// Header catalog switch: a pinned ?catalog= follows the new selection so
// the list stays scoped to what the selector shows; without a pin the
// list simply refetches with the new scope.
watch(selected, (alias) => {
  if (typeof route.query.catalog === 'string' && route.query.catalog) {
    navigateTo({
      query: { ...route.query, catalog: alias || undefined },
      replace: true,
    });
  } else {
    fetchViewColumns();
    doSearch();
  }
});
// Another type card or a shared link may change the alias or the pin —
// reload everything scoped to the new scope.
watch([typeAlias, () => route.query.catalog], () => {
  fetchViewColumns();
  doSearch();
});
</script>
