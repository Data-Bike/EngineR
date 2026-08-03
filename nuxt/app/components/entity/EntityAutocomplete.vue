<template>
  <UInputMenu
    :model-value="{ label: modelValue, value: modelValue }"
    :items="results"
    :loading="loading"
    :placeholder="t('entity.search')"
    :disabled="disabled"
    :search="search"
    class="w-full"
    @update:model-value="onSelect"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps<{
  modelValue: string;
  targetTypeId?: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  'update:model-value': [value: string];
}>();

const { t } = useI18n();
const api = useApi();

const loading = ref(false);
const results = ref<{ label: string; value: string }[]>([]);

async function search(query: string): Promise<{ label: string; value: string }[]> {
  if (!query.trim() || !props.targetTypeId) return [];
  loading.value = true;
  try {
    const res = await api.entities.query({
      query: {
        from: { type_id: props.targetTypeId, alias: 'e' },
        select: [{ field: { system: 'public_id' }, alias: 'public_id' }],
        where: {
          comparison: {
            field: { system: 'public_id' },
            op: 'prefix',
            value: query,
          },
        },
        sort: [{ field: { system: 'public_id' }, direction: 'asc' }],
        page: { size: 10 },
      },
    });
    results.value = (res.data.rows as Array<{ public_id: string }>).map((r) => ({
      label: r.public_id,
      value: r.public_id,
    }));
    return results.value;
  } catch {
    return [];
  } finally {
    loading.value = false;
  }
}

function onSelect(item: { label: string; value: string } | string) {
  const value = typeof item === 'string' ? item : item.value;
  emit('update:model-value', value);
}
</script>
