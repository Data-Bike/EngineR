<template>
  <form @submit.prevent="onSubmit" class="space-y-6">
    <!-- Sections (docs/16 §6, docs/17 §5) -->
    <div v-for="section in visibleSections" :key="section.label_key" class="space-y-4">
      <!-- Section header -->
      <div class="flex items-center gap-2">
        <h3 class="text-lg font-semibold">{{ t(section.label_key) }}</h3>
        <UButton
          v-if="section.collapsible"
          :icon="isCollapsed(section) ? 'i-lucide-chevron-down' : 'i-lucide-chevron-up'"
          size="xs"
          variant="ghost"
          @click="toggleSection(section)"
        />
      </div>

      <!-- Section fields -->
      <div v-show="!isCollapsed(section)" class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div
          v-for="field in section.fields"
          :key="field.predicate_id"
          :class="{ 'md:col-span-2': isWideField(field) }"
        >
          <FormField
            :field="field"
            :model-value="values[field.predicate_id]"
            :error="validationErrors?.[field.predicate_id]"
            @update:value="updateValue(field.predicate_id, $event)"
          />
        </div>
      </div>
    </div>

    <!-- Unsaved changes guard (docs/17 §5) -->
    <div v-if="hasUnsavedChanges" class="text-sm text-[var(--ui-text-muted)]">
      *
    </div>

    <!-- Validation errors display -->
    <div v-if="validationErrors && Object.keys(validationErrors).length > 0" class="rounded-md border border-red-300 bg-red-50 p-3 text-sm text-red-700">
      <ul>
        <li v-for="(err, key) in validationErrors" :key="key">
          {{ t(err.message_key) }}
        </li>
      </ul>
    </div>

    <!-- Actions -->
    <div class="flex justify-end gap-2 pt-4 border-t border-[var(--ui-border)]">
      <UButton variant="outline" @click="onCancel">
        {{ t('common.cancel') }}
      </UButton>
      <UButton
        v-if="allowDraft"
        variant="outline"
        @click="onSaveDraft"
      >
        {{ t('entity.save_draft') }}
      </UButton>
      <UButton type="submit" :loading="submitting">
        {{ submitLabel ?? t('entity.create') }}
      </UButton>
    </div>
  </form>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import type {
  FieldDefinition,
  FieldValidationError,
  FormSubmission,
  ViewDefinition,
  ViewSection,
} from '@enginer/api-client';

const props = defineProps<{
  view: ViewDefinition;
  initialValues?: Record<string, unknown>;
  validationErrors?: Record<string, FieldValidationError>;
  submitting?: boolean;
  allowDraft?: boolean;
  submitLabel?: string;
}>();

const emit = defineEmits<{
  submit: [body: FormSubmission];
  cancel: [];
  saveDraft: [values: Record<string, unknown>];
}>();

const { t } = useI18n();

// --- State ---
const values = ref<Record<string, unknown>>({ ...props.initialValues ?? {} });
const collapsedSections = ref<Set<string>>(new Set());
const submitted = ref(false);

// --- Computed ---
const visibleSections = computed(() =>
  props.view.sections
    .filter((s) => s.fields.some((f) => !f.hidden))
    .sort((a, b) => a.order - b.order),
);

const hasUnsavedChanges = computed(() => {
  if (submitted.value) return false;
  if (!props.initialValues) return Object.keys(values.value).length > 0;
  return JSON.stringify(values.value) !== JSON.stringify(props.initialValues);
});

// --- Methods ---
function isCollapsed(section: ViewSection): boolean {
  return collapsedSections.value.has(section.label_key) || section.default_collapsed;
}

function toggleSection(section: ViewSection) {
  if (collapsedSections.value.has(section.label_key)) {
    collapsedSections.value.delete(section.label_key);
  } else {
    collapsedSections.value.add(section.label_key);
  }
}

function isWideField(field: FieldDefinition): boolean {
  return ['textarea', 'rich_text', 'repeating_group', 'address', 'relation_editor'].includes(
    field.kind,
  );
}

function updateValue(predicateId: string, value: unknown) {
  values.value[predicateId] = value;
}

function onSubmit() {
  const fields = Object.entries(values.value).map(([predicateId, value]) => ({
    predicate_id: predicateId,
    value,
  }));
  submitted.value = true;
  emit('submit', {
    type_id: props.view.type_id,
    schema_revision: props.view.schema_revision,
    identity_mode: 'return_existing',
    fields,
  });
}

function onCancel() {
  emit('cancel');
}

function onSaveDraft() {
  emit('saveDraft', { ...values.value });
}

// Warn before navigate away with unsaved changes (docs/17 §5)
onBeforeRouteLeave(() => {
  if (hasUnsavedChanges.value) {
    return window.confirm(t('entity.unsaved_changes'));
  }
  return true;
});
</script>
