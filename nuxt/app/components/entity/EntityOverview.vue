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
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: { sections?: Array<{ fields?: Array<{ predicate_id: string; label_key: string }> }> } }>(
      `${baseUrl.replace(/\/$/, '')}/schemas/types/${props.entity.type_id}/view`,
      {
        query: { view_type: 'form' },
        credentials: 'include',
        headers: { Accept: 'application/json' },
      },
    );
    const fields: Record<string, string> = {};
    if (res.data?.sections) {
      for (const section of res.data.sections) {
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
    value: String(value ?? ''),
  }));
});

onMounted(fetchViewFields);
watch(() => props.entity.type_id, fetchViewFields);
</script>
