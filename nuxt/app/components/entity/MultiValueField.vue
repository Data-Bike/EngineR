<template>
  <div class="space-y-2">
    <!-- Existing values -->
    <div
      v-for="(item, index) in items"
      :key="index"
      class="flex items-center gap-2"
    >
      <UInput
        :model-value="item"
        :disabled="disabled"
        class="flex-1"
        @update:model-value="updateItem(index, $event)"
      />
      <UButton
        v-if="!disabled && canRemove"
        icon="i-lucide-x"
        size="sm"
        variant="ghost"
        color="error"
        @click="removeItem(index)"
      />
    </div>

    <!-- Add button -->
    <UButton
      v-if="!disabled && canAdd"
      icon="i-lucide-plus"
      size="sm"
      variant="outline"
      @click="addItem"
    >
      {{ t('entity.add_value') }}
    </UButton>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FieldDefinition } from '@enginer/api-client';

const props = defineProps<{
  field: FieldDefinition;
  modelValue: unknown;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  'update:model-value': [value: unknown];
}>();

const { t } = useI18n();

const items = computed<string[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as string[];
  if (props.modelValue === null || props.modelValue === undefined) return [];
  return [String(props.modelValue)];
});

const canAdd = computed(() => {
  const max = props.field.max_cardinality;
  return max === undefined || items.value.length < max;
});

const canRemove = computed(() => items.value.length > props.field.min_cardinality);

function addItem() {
  const next = [...items.value, ''];
  emit('update:model-value', next);
}

function removeItem(index: number) {
  const next = items.value.filter((_, i) => i !== index);
  emit('update:model-value', next);
}

function updateItem(index: number, value: string) {
  const next = [...items.value];
  next[index] = value;
  emit('update:model-value', next);
}
</script>
