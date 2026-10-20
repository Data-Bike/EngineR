<template>
  <UInputMenu
    :model-value="modelValue || undefined"
    v-model:search-term="searchTerm"
    :items="menuItems"
    :loading="loading"
    :placeholder="placeholder ?? t('entity.search_placeholder')"
    :disabled="disabled"
    value-key="value"
    label-key="label"
    ignore-filter
    trailing-icon="i-lucide-search"
    class="w-full"
    @update:model-value="onSelect"
    @focus="ensureInitialOptions"
  />
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps<{
  modelValue: string;
  targetTypeId?: string;
  disabled?: boolean;
  placeholder?: string;
}>();

const emit = defineEmits<{
  'update:model-value': [value: string];
}>();

const { t } = useI18n();
const api = useApi();
const route = useRoute();
const { catalogParams } = useCatalog();
// Relation fields search within the page's catalog scope — the route query
// may pin a specific catalog (type card on /objects), otherwise the header
// selection applies.
const scopedCatalog = () => {
  const q = route.query.catalog;
  return typeof q === 'string' && q ? q : catalogParams().catalog;
};

interface Option {
  label: string;
  value: string;
}

const loading = ref(false);
const results = ref<Option[]>([]);
const searchTerm = ref('');
const knownLabels = ref<Record<string, string>>({});
const initialized = ref(false);
let debounceTimer: ReturnType<typeof setTimeout> | undefined;
let requestSeq = 0;

const selectedItem = computed<Option | undefined>(() => {
  if (!props.modelValue) return undefined;
  const label = knownLabels.value[props.modelValue]
    ?? results.value.find((item) => item.value === props.modelValue)?.label
    ?? props.modelValue;
  return { label, value: props.modelValue };
});

/** Search results plus the current selection, so UInputMenu can always
 * resolve the label even when the value is not among fetched options. */
const menuItems = computed<Option[]>(() => {
  const sel = selectedItem.value;
  if (sel && !results.value.some((item) => item.value === sel.value)) {
    return [sel, ...results.value];
  }
  return results.value;
});

async function fetchOptions(query: string): Promise<void> {
  const seq = ++requestSeq;
  loading.value = true;
  try {
    const res = await api.entities.list({
      type: props.targetTypeId || undefined,
      search: query.trim() || undefined,
      limit: 20,
      catalog: scopedCatalog() || undefined,
    });
    if (seq !== requestSeq) return;
    results.value = res.data.map((r) => ({
      label: `${r.display_name ?? r.public_id}${r.type_id ? ` · ${r.type_id}` : ''}`,
      value: r.public_id,
    }));
    for (const item of results.value) knownLabels.value[item.value] = item.label;
  } catch {
    if (seq === requestSeq) results.value = [];
  } finally {
    if (seq === requestSeq) loading.value = false;
  }
}

function ensureInitialOptions() {
  if (initialized.value) return;
  initialized.value = true;
  fetchOptions('');
}

watch(searchTerm, (term) => {
  initialized.value = true;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => fetchOptions(term), 250);
});

// Resolve the label of a pre-selected value so the field does not show a bare id.
watch(
  () => props.modelValue,
  async (value) => {
    if (!value || knownLabels.value[value]) return;
    try {
      const res = await api.entities.get(value);
      knownLabels.value[value] = res.data.display_name ?? value;
    } catch {
      // keep the raw id as label
    }
  },
  { immediate: true },
);

function onSelect(item: Option | string | undefined) {
  const value = typeof item === 'string' ? item : item?.value ?? '';
  emit('update:model-value', value);
}

onMounted(ensureInitialOptions);
</script>
