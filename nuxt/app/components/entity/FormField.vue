<template>
  <UFormField
    :label="t(field.label_key)"
    :required="field.required"
    :help="field.help_key ? t(field.help_key) : undefined"
    :error="error ? t(error.message_key) : undefined"
  >
    <!-- Text -->
    <UInput
      v-if="field.kind === 'text'"
      :model-value="modelValue as string"
      :placeholder="field.placeholder_key ? t(field.placeholder_key) : undefined"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Textarea -->
    <UTextarea
      v-else-if="field.kind === 'textarea'"
      :model-value="modelValue as string"
      :placeholder="field.placeholder_key ? t(field.placeholder_key) : undefined"
      :disabled="field.read_only"
      :rows="4"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Boolean -->
    <USwitch
      v-else-if="field.kind === 'boolean'"
      :model-value="modelValue as boolean"
      :disabled="field.read_only"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Integer / Decimal / Money / Percent / Measurement -->
    <UInput
      v-else-if="['integer', 'decimal', 'money', 'percent', 'measurement'].includes(field.kind)"
      :model-value="scalarValue as string"
      type="number"
      :placeholder="field.placeholder_key ? t(field.placeholder_key) : undefined"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Url / Email / Phone / Duration -->
    <UInput
      v-else-if="['url', 'email', 'phone', 'duration'].includes(field.kind)"
      :model-value="modelValue as string"
      :type="field.kind === 'url' ? 'url' : field.kind === 'email' ? 'email' : field.kind === 'phone' ? 'tel' : 'text'"
      :placeholder="field.placeholder_key ? t(field.placeholder_key) : undefined"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Date -->
    <UInput
      v-else-if="field.kind === 'date'"
      :model-value="modelValue as string"
      type="date"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Time -->
    <UInput
      v-else-if="field.kind === 'time'"
      :model-value="modelValue as string"
      type="time"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Datetime -->
    <UInput
      v-else-if="['datetime', 'local_datetime'].includes(field.kind)"
      :model-value="scalarValue as string"
      type="datetime-local"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Enum / Dictionary -->
    <USelect
      v-else-if="field.kind === 'enum' || field.kind === 'dictionary'"
      :model-value="modelValue as string"
      :items="enumOptions"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Entity reference (autocomplete, docs/17 §5) -->
    <EntityAutocomplete
      v-else-if="field.kind === 'entity_reference'"
      :model-value="modelValue as string"
      :target-type-id="field.target_type_id"
      :disabled="field.read_only"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Multi-value / Repeating group / Tags -->
    <MultiValueField
      v-else-if="['multi_value', 'repeating_group', 'tags'].includes(field.kind)"
      :field="field"
      :model-value="modelValue"
      :disabled="field.read_only"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- File -->
    <UInput
      v-else-if="field.kind === 'file'"
      type="file"
      :disabled="field.read_only"
      class="w-full"
      @update:model-value="emit('update:value', $event)"
    />

    <!-- Derived / read-only -->
    <div
      v-else-if="field.kind === 'derived' || field.read_only"
      class="px-3 py-2 rounded-md bg-[var(--ui-bg-muted)] text-[var(--ui-text-muted)]"
    >
      {{ scalarValue ?? '—' }}
    </div>

    <!-- Unknown kind: safe unsupported component (docs/16 §6) -->
    <div
      v-else
      class="px-3 py-2 rounded-md border border-dashed border-[var(--ui-border)] text-sm text-[var(--ui-text-muted)]"
    >
      {{ t('entity.unsupported_field') }}: {{ field.kind }}
    </div>

    <!-- Identity preview (raw/canonical, docs/17 §5) -->
    <template v-if="field.identity && modelValue" #hint>
      <span class="text-xs text-[var(--ui-text-muted)]">
        {{ t('entity.canonical_preview') }}: {{ canonicalPreview }}
      </span>
    </template>
  </UFormField>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FieldDefinition, FieldOption, FieldValidationError } from '@enginer/api-client';

const props = defineProps<{
  field: FieldDefinition;
  modelValue: unknown;
  error?: FieldValidationError;
}>();

const emit = defineEmits<{
  'update:value': [value: unknown];
}>();

const { t } = useI18n();

const enumOptions = computed(() =>
  (props.field.options ?? []).map((opt: FieldOption) => ({
    label: t(opt.label_key),
    value: opt.value,
  })),
);

/**
 * Wire values for structured kinds arrive as objects — money
 * `{amount, currency}`, measurement `{amount, unit}`, local datetime
 * `{datetime, zone}`, blob `{blob}` (docs/26). Scalar inputs need the
 * inner primitive; the zone/currency suffix stays server-side.
 */
const scalarValue = computed(() => {
  const v = props.modelValue;
  if (v && typeof v === 'object' && !Array.isArray(v)) {
    const o = v as Record<string, unknown>;
    if (typeof o.amount === 'string' || typeof o.amount === 'number') return String(o.amount);
    if (typeof o.datetime === 'string') return o.datetime.replace(' ', 'T').slice(0, 16);
    if (typeof o.blob === 'string') return o.blob;
  }
  return v;
});

const canonicalPreview = computed(() => {
  const v = scalarValue.value;
  if (typeof v === 'string') {
    return v.normalize('NFC').trim().toLowerCase();
  }
  return String(v ?? '');
});
</script>
