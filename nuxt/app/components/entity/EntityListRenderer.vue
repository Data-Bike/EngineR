<template>
  <div class="space-y-4">
    <!-- Filters bar (docs/17 §3) -->
    <div v-if="view.filters.length > 0" class="flex flex-wrap gap-2 items-end">
      <div v-for="filter in view.filters" :key="filter.predicate_id" class="min-w-[180px]">
        <label class="block text-xs text-[var(--ui-text-muted)] mb-1">
          {{ t(filter.label_key) }}
        </label>
        <UInput
          v-model="filterValues[filter.predicate_id]"
          :placeholder="t(filter.label_key)"
          size="sm"
          class="w-full"
          @keyup.enter="applyFilters"
        />
      </div>
      <UButton size="sm" icon="i-lucide-search" @click="applyFilters">
        {{ t('entity.search') }}
      </UButton>
      <UButton size="sm" variant="ghost" @click="clearFilters">
        {{ t('common.cancel') }}
      </UButton>
    </div>

    <!-- Facets (docs/17 §3) -->
    <div v-if="view.facets.length > 0 && facets" class="flex flex-wrap gap-4">
      <div v-for="facet in view.facets" :key="facet.predicate_id" class="space-y-1">
        <p class="text-xs font-medium text-[var(--ui-text-muted)]">{{ t(facet.label_key) }}</p>
        <div class="space-y-1">
          <label
            v-for="bucket in facets[facet.predicate_id] ?? []"
            :key="bucket.value"
            class="flex items-center gap-2 text-sm"
          >
            <input
              type="checkbox"
              :checked="selectedFacets[facet.predicate_id]?.includes(bucket.value)"
              @change="toggleFacet(facet.predicate_id, bucket.value)"
            />
            <span>{{ bucket.label }}</span>
            <span class="text-[var(--ui-text-muted)]">({{ bucket.count }})</span>
          </label>
        </div>
      </div>
    </div>

    <!-- Table -->
    <div class="overflow-x-auto rounded-lg border border-[var(--ui-border)]">
      <table class="w-full text-sm" role="table">
        <caption class="sr-only">{{ t(view.name_key) }}</caption>
        <thead>
          <tr class="border-b border-[var(--ui-border)] bg-[var(--ui-bg-muted)]" role="row">
            <!-- Bulk selection checkbox -->
            <th v-if="view.bulk_actions.length > 0" class="py-2 px-3 w-10">
              <input
                type="checkbox"
                :checked="allSelected"
                :indeterminate="someSelected"
                @change="toggleSelectAll"
                :aria-label="t('entity.select_all')"
              />
            </th>
            <!-- Columns -->
            <th
              v-for="col in view.columns"
              :key="col.predicate_id"
              class="py-2 px-3 text-left font-medium text-[var(--ui-text-muted)]"
              role="columnheader"
              :aria-sort="getSortOrder(col.predicate_id)"
            >
              <button
                v-if="isSortable(col)"
                class="flex items-center gap-1 hover:text-[var(--ui-text)]"
                @click="toggleSort(col.predicate_id)"
              >
                {{ t(col.label_key) }}
                <UIcon
                  v-if="sortState[col.predicate_id]"
                  :name="sortState[col.predicate_id] === 'asc' ? 'i-lucide-arrow-up' : 'i-lucide-arrow-down'"
                  class="text-xs"
                />
              </button>
              <span v-else>{{ t(col.label_key) }}</span>
            </th>
            <!-- Row actions -->
            <th v-if="view.row_actions.length > 0" class="py-2 px-3 w-32" role="columnheader">
              {{ t('entity.actions') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <!-- Loading -->
          <tr v-if="loading" role="row">
            <td :colspan="totalColumns" class="py-8 text-center text-[var(--ui-text-muted)]">
              {{ t('entity.loading') }}
            </td>
          </tr>
          <!-- Empty -->
          <tr v-else-if="rows.length === 0" role="row">
            <td :colspan="totalColumns" class="py-8 text-center text-[var(--ui-text-muted)]">
              {{ t('entity.empty') }}
            </td>
          </tr>
          <!-- Error -->
          <tr v-else-if="error" role="row">
            <td :colspan="totalColumns" class="py-8 text-center text-red-500">
              {{ error }}
              <UButton size="sm" variant="outline" @click="$emit('retry')" class="ml-2">
                {{ t('common.retry') }}
              </UButton>
            </td>
          </tr>
          <!-- Data rows -->
          <tr
            v-for="row in rows"
            :key="getRowKey(row)"
            class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)] cursor-pointer"
            role="row"
            @click="$emit('row-click', row)"
          >
            <td v-if="view.bulk_actions.length > 0" class="py-2 px-3" @click.stop>
              <input
                type="checkbox"
                :checked="isSelected(row)"
                @change="toggleSelectRow(row)"
                :aria-label="t('entity.select_row')"
              />
            </td>
            <td
              v-for="col in view.columns"
              :key="col.predicate_id"
              class="py-2 px-3"
              role="cell"
            >
              {{ formatCellValue(row[col.predicate_id]) }}
            </td>
            <td v-if="view.row_actions.length > 0" class="py-2 px-3" @click.stop>
              <div class="flex gap-1">
                <UButton
                  v-for="action in availableRowActions"
                  :key="action.action_id"
                  :icon="action.icon"
                  size="xs"
                  variant="ghost"
                  @click="$emit('row-action', { action_id: action.action_id, row })"
                >
                  {{ t(action.label_key) }}
                </UButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Cursor pagination (docs/17 §3) -->
    <div v-if="cursor" class="flex justify-center">
      <UButton variant="outline" :loading="loadingMore" @click="$emit('load-more')">
        {{ t('entity.load_more') }}
      </UButton>
    </div>

    <!-- Selection summary (docs/17 §3: bulk selection uses query token, not millions of IDs) -->
    <div v-if="selectedCount > 0" class="flex items-center justify-between rounded-md bg-[var(--ui-bg-muted)] px-4 py-2">
      <span class="text-sm">
        {{ t('entity.selected_count', { count: selectedCount }) }}
      </span>
      <div class="flex gap-2">
        <UButton
          v-for="action in view.bulk_actions"
          :key="action.action_id"
          size="sm"
          @click="$emit('bulk-action', action.action_id)"
        >
          {{ t(action.label_key) }}
        </UButton>
        <UButton size="sm" variant="ghost" @click="clearSelection">
          {{ t('common.cancel') }}
        </UButton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ViewDefinition, RowAction } from '@enginer/api-client';

const props = defineProps<{
  view: ViewDefinition;
  rows: Record<string, unknown>[];
  loading?: boolean;
  loadingMore?: boolean;
  error?: string | null;
  cursor?: string;
  facets?: Record<string, FacetBucket[]>;
}>();

const emit = defineEmits<{
  'row-click': [row: Record<string, unknown>];
  'row-action': [payload: { action_id: string; row: Record<string, unknown> }];
  'bulk-action': [actionId: string];
  'load-more': [];
  'retry': [];
  'filter-change': [filters: Record<string, string>];
  'facet-change': [facets: Record<string, string[]>];
  'sort-change': [sort: Record<string, 'asc' | 'desc'>];
}>();

const { t } = useI18n();

interface FacetBucket {
  value: string;
  label: string;
  count: number;
}

// --- Filters ---
const filterValues = ref<Record<string, string>>({});

function applyFilters() {
  emit('filter-change', { ...filterValues.value });
}

function clearFilters() {
  filterValues.value = {};
  emit('filter-change', {});
}

// --- Facets ---
const selectedFacets = ref<Record<string, string[]>>({});

function toggleFacet(predicateId: string, value: string) {
  if (!selectedFacets.value[predicateId]) {
    selectedFacets.value[predicateId] = [];
  }
  const idx = selectedFacets.value[predicateId].indexOf(value);
  if (idx >= 0) {
    selectedFacets.value[predicateId].splice(idx, 1);
  } else {
    selectedFacets.value[predicateId].push(value);
  }
  emit('facet-change', { ...selectedFacets.value });
}

// --- Sort ---
const sortState = ref<Record<string, 'asc' | 'desc'>>({});

function isSortable(col: { kind: string }): boolean {
  return !['rich_text', 'file', 'repeating_group'].includes(col.kind);
}

function toggleSort(predicateId: string) {
  const current = sortState.value[predicateId];
  if (current === 'asc') {
    sortState.value[predicateId] = 'desc';
  } else if (current === 'desc') {
    delete sortState.value[predicateId];
  } else {
    sortState.value[predicateId] = 'asc';
  }
  emit('sort-change', { ...sortState.value });
}

function getSortOrder(predicateId: string): 'ascending' | 'descending' | 'none' {
  const s = sortState.value[predicateId];
  if (s === 'asc') return 'ascending';
  if (s === 'desc') return 'descending';
  return 'none';
}

// --- Selection ---
const selectedKeys = ref<Set<string>>(new Set());

const allSelected = computed(() => {
  return props.rows.length > 0 && props.rows.every((r) => isSelected(r));
});

const someSelected = computed(() => {
  return selectedKeys.value.size > 0 && !allSelected.value;
});

const selectedCount = computed(() => selectedKeys.value.size);

function getRowKey(row: Record<string, unknown>): string {
  return String(row.public_id ?? row.entity_id ?? JSON.stringify(row));
}

function isSelected(row: Record<string, unknown>): boolean {
  return selectedKeys.value.has(getRowKey(row));
}

function toggleSelectRow(row: Record<string, unknown>) {
  const key = getRowKey(row);
  if (selectedKeys.value.has(key)) {
    selectedKeys.value.delete(key);
  } else {
    selectedKeys.value.add(key);
  }
}

function toggleSelectAll() {
  if (allSelected.value) {
    selectedKeys.value.clear();
  } else {
    props.rows.forEach((r) => selectedKeys.value.add(getRowKey(r)));
  }
}

function clearSelection() {
  selectedKeys.value.clear();
}

// --- Available actions (capability-filtered) ---
const { hasCapability } = useCapability();

const availableRowActions = computed<RowAction[]>(() =>
  props.view.row_actions.filter((a) => hasCapability(a.capability)),
);

// --- Formatting ---
function formatCellValue(v: unknown): string {
  if (v === null || v === undefined) return '';
  if (typeof v === 'string') return v;
  if (typeof v === 'number') return String(v);
  if (typeof v === 'boolean') return v ? t('common.yes') : t('common.no');
  return String(v);
}

// --- Computed ---
const totalColumns = computed(() => {
  let count = props.view.columns.length;
  if (props.view.bulk_actions.length > 0) count++;
  if (props.view.row_actions.length > 0) count++;
  return count;
});
</script>
