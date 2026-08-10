<template>
  <div class="space-y-6">
    <!-- Data fields from view definition -->
    <div v-if="dataFields.length > 0">
      <h3 class="text-lg font-semibold mb-3">{{ t('sections.main') }}</h3>
      <dl class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div v-for="field in dataFields" :key="field.key" class="border-b border-[var(--ui-border)] pb-2">
          <dt class="text-sm text-[var(--ui-text-muted)]">{{ field.label }}</dt>
          <dd class="font-medium">{{ field.value || '—' }}</dd>
        </div>
      </dl>
    </div>

    <!-- System fields -->
    <div>
      <h3 class="text-lg font-semibold mb-3">{{ t('entity.tabs.overview') }}</h3>
      <dl class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div v-for="field in systemFields" :key="field.label" class="border-b border-[var(--ui-border)] pb-2">
          <dt class="text-sm text-[var(--ui-text-muted)]">{{ field.label }}</dt>
          <dd class="font-medium">{{ field.value }}</dd>
        </div>
      </dl>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import type { Entity } from '@enginer/api-client';

const props = defineProps<{ entity: Entity }>();

const { t } = useI18n();
const toast = useToast();
const api = useApi();

const viewFields = ref<Record<string, string>>({});

const systemFields = computed(() => [
  { label: 'Public ID', value: props.entity.public_id },
  { label: 'Type', value: props.entity.type_id },
  { label: 'Class', value: props.entity.entity_class },
  { label: 'Revision', value: String(props.entity.revision) },
  { label: 'Status', value: props.entity.status },
  { label: 'Created at', value: props.entity.created_at },
  { label: 'Retired at', value: props.entity.retired_at ?? '—' },
]);

// Entity data fields from the API response (entity.fields)
const entityDataFields = computed<Record<string, unknown>>(() => {
  return (props.entity as Entity & { fields?: Record<string, unknown> }).fields ?? {};
});

// Fetch view definition to get field labels
async function fetchViewFields() {
  try {
    const res = await api.schemas.getTypeView(props.entity.type_id, 'form');
    const data = res.data as { sections?: Array<{ fields?: Array<{ predicate_id: string; label_key: string }> }> } | undefined;
    const fields: Record<string, string> = {};
    if (data?.sections) {
      for (const section of data.sections) {
        if (section.fields) {
          for (const field of section.fields) {
            fields[field.predicate_id] = field.label_key;
          }
        }
      }
    }
    viewFields.value = fields;
  } catch {
    // Non-fatal — use raw field names as labels
  }
}

const dataFields = computed(() => {
  const data = entityDataFields.value;
  return Object.entries(data).map(([key, value]) => ({
    key,
    label: viewFields.value[key] ? t(viewFields.value[key]) : key,
    value: formatValue(value),
  }));
});

function formatValue(value: unknown): string {
  if (value === null || value === undefined) return '—';
  if (typeof value === 'string') return value;
  if (typeof value === 'boolean') return value ? t('common.yes') : t('common.no');
  if (typeof value === 'number') return String(value);
  // Object with nested value (e.g. { kind: 'text', value: 'Иван' })
  if (typeof value === 'object' && value !== null) {
    const obj = value as Record<string, unknown>;
    if ('value' in obj) return formatValue(obj.value);
    if ('text' in obj) return String(obj.text);
    // Fallback: stringify but avoid [object Object]
    try { return JSON.stringify(obj); } catch { return '—'; }
  }
  return String(value ?? '');
}

onMounted(fetchViewFields);
watch(() => props.entity.type_id, fetchViewFields);
</script>
