<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('entity.create') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ typeAlias }}</p>
      </div>
      <UButton variant="outline" icon="i-lucide-x" @click="goBack">
        {{ t('common.cancel') }}
      </UButton>
    </div>

    <!-- Loading view definition -->
    <div v-if="loadingView" class="text-center py-12 text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>

    <!-- Error loading view -->
    <div v-else-if="viewError" class="text-center py-12">
      <p class="text-red-500 mb-2">{{ viewError }}</p>
      <UButton variant="outline" @click="fetchView">{{ t('common.retry') }}</UButton>
    </div>

    <!-- Form -->
    <EntityFormRenderer
      v-else-if="view"
      :view="view"
      :validation-errors="validationErrors"
      :submitting="submitting"
      :allow-draft="true"
      @submit="onSubmit"
      @cancel="goBack"
      @save-draft="onSaveDraft"
    />

    <!-- Duplicate suggestion dialog (docs/17 §5) -->
    <DuplicateSuggestionDialog
      v-model:open="showDuplicate"
      :matches="duplicateMatches"
      :identity-mode="identityMode"
      @open-existing="openExisting"
      @create-distinct="createDistinct"
      @cancel="showDuplicate = false"
    />
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type {
  ViewDefinition,
  FormSubmission,
  FieldValidationError,
  QueryBody,
} from '@enginer/api-client';

definePageMeta({
  middleware: ['auth', capability('entity.create')],
});

const { t } = useI18n();
const route = useRoute();
const toast = useToast();
const api = useApi();

const typeAlias = computed(() => route.params.typeAlias as string);
const view = ref<ViewDefinition | null>(null);
const loadingView = ref(false);
const viewError = ref<string | null>(null);
const submitting = ref(false);
const validationErrors = ref<Record<string, FieldValidationError>>({});

// Duplicate suggestion state (docs/17 §5)
const showDuplicate = ref(false);
const duplicateMatches = ref<DuplicateMatch[]>([]);
const identityMode = ref<'return_existing' | 'reject' | 'create_candidate'>('return_existing');
const pendingSubmission = ref<FormSubmission | null>(null);

interface DuplicateMatch {
  public_id: string;
  display_name?: string;
  type_id: string;
  match_type: 'exact' | 'fuzzy';
  score?: number;
  explanation?: string;
}

async function fetchView() {
  loadingView.value = true;
  viewError.value = null;
  try {
    const res = await api.schemas.getTypeView(typeAlias.value, 'form');
    view.value = res.data as ViewDefinition;
  } catch (e: unknown) {
    viewError.value = e instanceof Error ? e.message : t('entity.error');
    toast.add({ title: t('common.error'), description: viewError.value!, color: 'error' });
  } finally {
    loadingView.value = false;
  }
}

async function onSubmit(body: FormSubmission) {
  submitting.value = true;
  validationErrors.value = {};
  try {
    // Step 1: check for duplicates before commit (docs/17 §5)
    const dupCheck = await checkDuplicates(body);
    if (dupCheck.length > 0) {
      duplicateMatches.value = dupCheck;
      pendingSubmission.value = body;
      showDuplicate.value = true;
      submitting.value = false;
      return;
    }

    // Step 2: no duplicates — create directly
    await createEntity(body);
  } catch (e: unknown) {
    const err = e as { validation_errors?: Record<string, FieldValidationError> };
    if (err.validation_errors) {
      validationErrors.value = err.validation_errors;
    } else {
      viewError.value = e instanceof Error ? e.message : t('entity.error');
    }
  } finally {
    submitting.value = false;
  }
}

async function checkDuplicates(body: FormSubmission): Promise<DuplicateMatch[]> {
  // Query for entities with matching identity fields (docs/17 §5)
  // Use /entities/query with type filter and identity field comparisons

  // Collect identity fields from the view definition
  const identityFields: Array<{ predicate_id: string; value: unknown }> = [];
  if (view.value?.sections) {
    for (const section of view.value.sections) {
      for (const field of section.fields) {
        if (field.identity && !field.hidden) {
          const val = body.fields.find((f) => f.predicate_id === field.alias)?.value;
          if (val !== undefined && val !== null && String(val).trim() !== '') {
            identityFields.push({ predicate_id: field.alias, value: val });
          }
        }
      }
    }
  }

  if (identityFields.length === 0) return [];

  try {
    // Query entities of the same type
    const queryBody: QueryBody = {
      from: { type_id: typeAlias.value, alias: 'e' },
      select: [
        { field: { system: 'public_id' }, alias: 'public_id' },
        { field: { system: 'display_name' }, alias: 'display_name' },
      ],
      where: {
        or: identityFields.map((f) => ({
          comparison: {
            field: { predicate: f.predicate_id },
            op: 'eq',
            value: String(f.value),
          },
        })),
      },
      page: { size: 5 },
    };
    const res = await api.entities.query(queryBody);
    const rows = res.data?.rows ?? [];
    const strongAliases = new Set(['inn', 'has_tax_id', 'has_ogrn', 'ogrn', 'snils', 'snilz', 'document_number']);
    // Field label lookup from the loaded view so the explanation shows
    // localized names instead of raw predicate aliases.
    const labelOf = (predicateId: string): string => {
      for (const section of view.value?.sections ?? []) {
        const field = section.fields.find((f) => f.predicate_id === predicateId || f.alias === predicateId);
        if (field) return t(field.label_key);
      }
      return predicateId;
    };
    return rows.map((r) => {
      const matchedFields = identityFields.filter((field) => (
        String(r[field.predicate_id] ?? '').trim().toLowerCase()
        === String(field.value).trim().toLowerCase()
      ));
      const exact = matchedFields.some((field) => strongAliases.has(field.predicate_id))
        || matchedFields.length === identityFields.length;
      return {
        public_id: r.public_id as string,
        display_name: r.display_name as string | undefined,
        type_id: typeAlias.value,
        match_type: exact ? 'exact' as const : 'fuzzy' as const,
        score: exact ? 1 : 0.75,
        explanation: matchedFields.map((field) => labelOf(field.predicate_id)).join(', '),
      };
    });
  } catch {
    // On error, don't block creation
    return [];
  }
}

async function createEntity(body: FormSubmission) {
  // Convert form fields to statements (docs/06 §6)
  const statements = body.fields.map((f) => {
    const v = f.value;
    let statementValue: { kind: 'text'; value: string } | { kind: 'boolean'; value: boolean };
    if (typeof v === 'boolean') {
      statementValue = { kind: 'boolean', value: v };
    } else {
      statementValue = { kind: 'text', value: String(v ?? '') };
    }
    return {
      predicate_id: f.predicate_id,
      value: statementValue,
    };
  });
  const res = await api.entities.create({
    type_id: body.type_id,
    schema_revision: body.schema_revision,
    identity_mode: body.identity_mode,
    statements,
  });
  toast.add({
    title: res.data.outcome === 'deduplicated'
      ? t('entity.duplicate.open_existing')
      : t('entity.create_success'),
    color: res.data.outcome === 'deduplicated' ? 'warning' : 'success',
  });
  // Navigate to the created entity card
  await navigateTo(`/entities/${res.data.public_id}`);
}

function openExisting(publicId: string) {
  showDuplicate.value = false;
  navigateTo(`/entities/${publicId}`);
}

async function createDistinct() {
  showDuplicate.value = false;
  if (pendingSubmission.value) {
    // Re-submit with create_candidate mode to force distinct creation
    pendingSubmission.value.identity_mode = 'create_candidate';
    submitting.value = true;
    try {
      await createEntity(pendingSubmission.value);
    } catch (e: unknown) {
      const err = e as { validation_errors?: Record<string, FieldValidationError> };
      if (err.validation_errors) {
        validationErrors.value = err.validation_errors;
      } else {
        viewError.value = e instanceof Error ? e.message : t('entity.error');
      }
    } finally {
      submitting.value = false;
    }
  }
}

function onSaveDraft(values: Record<string, unknown>) {
  // Save draft to localStorage for now (docs/17 §5: save draft option)
  if (import.meta.client) {
    localStorage.setItem(`entity_draft_${typeAlias.value}`, JSON.stringify(values));
  }
}

function goBack() {
  navigateTo(`/objects/${typeAlias.value}`);
}

onMounted(fetchView);
</script>
