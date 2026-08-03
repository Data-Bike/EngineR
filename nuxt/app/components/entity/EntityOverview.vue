<template>
  <div class="space-y-4">
    <dl class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div v-for="field in fields" :key="field.label" class="border-b border-[var(--ui-border)] pb-2">
        <dt class="text-sm text-[var(--ui-text-muted)]">{{ field.label }}</dt>
        <dd class="font-medium">{{ field.value }}</dd>
      </div>
    </dl>
  </div>
</template>

<script setup lang="ts">
import type { Entity } from '@enginer/api-client';

const props = defineProps<{ entity: Entity }>();

const fields = computed(() => [
  { label: 'Public ID', value: props.entity.public_id },
  { label: 'Entity ID', value: String(props.entity.entity_id) },
  { label: 'Type ID', value: props.entity.type_id },
  { label: 'Class', value: props.entity.entity_class },
  { label: 'Revision', value: String(props.entity.revision) },
  { label: 'Status', value: props.entity.status },
  { label: 'Created at', value: props.entity.created_at },
  { label: 'Retired at', value: props.entity.retired_at ?? '—' },
]);
</script>
