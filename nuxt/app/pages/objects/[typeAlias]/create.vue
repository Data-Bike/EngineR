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

    <!-- Submit error — inline, the form state survives (422/409 failures
         without a per-field error map used to replace the whole form). -->
    <UAlert
      v-if="submitError && view"
      color="error"
      variant="soft"
      icon="i-lucide-alert-triangle"
      :title="t('common.error')"
      :description="submitError"
      :close-button="{ icon: 'i-lucide-x', color: 'gray', variant: 'link' }"
      @close="submitError = null"
    />

    <!-- Form — own v-if, not chained to the submit-error alert above:
         the alert must coexist with the form instead of replacing it. -->
    <EntityFormRenderer
      ref="formRef"
      v-if="!loadingView && !viewError && view"
      :key="draftKey"
      :view="view"
      :initial-values="draftValues"
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
import { ref, computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type {
  ViewDefinition,
  FormSubmission,
  FieldValidationError,
  QueryBody,
  RawStatementValue,
} from '@enginer/api-client';

definePageMeta({
  middleware: ['auth', capability('entity.create')],
});

const { t, te } = useI18n();
const route = useRoute();
const toast = useToast();
const api = useApi();
const { catalogParams, selected } = useCatalog();
// The type card on /objects may pin a catalog in the route query; it wins
// over the header selection so the create form targets the same catalog
// the user clicked.
const scopedCatalogParams = () => {
  const q = route.query.catalog;
  return typeof q === 'string' && q ? { catalog: q } : catalogParams();
};

const typeAlias = computed(() => route.params.typeAlias as string);
const view = ref<ViewDefinition | null>(null);
const loadingView = ref(false);
const viewError = ref<string | null>(null);
const submitError = ref<string | null>(null);
const submitting = ref(false);
const validationErrors = ref<Record<string, FieldValidationError>>({});
const formRef = ref<{ markSaved: () => void } | null>(null);

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
    const res = await api.schemas.getTypeView(typeAlias.value, 'form', scopedCatalogParams());
    view.value = res.data as ViewDefinition;
  } catch (e: unknown) {
    viewError.value = displayError(e, t, te);
    toast.add({ title: t('common.error'), description: viewError.value!, color: 'error' });
  } finally {
    loadingView.value = false;
  }
}

async function onSubmit(body: FormSubmission) {
  submitting.value = true;
  validationErrors.value = {};
  submitError.value = null;
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
      submitError.value = displayError(e, t, te);
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
    const res = await api.entities.query(queryBody, scopedCatalogParams());
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
  // Send raw form values (docs/06 §6): coerce_field_input on the server
  // applies the declared field kind — forcing text here would store
  // dates/money/geo/entity references as plain text scalars.
  const statements = body.fields.map((f) => ({
    predicate_id: f.predicate_id,
    value: (f.value ?? '') as RawStatementValue,
  }));
  const res = await api.entities.create({
    type_id: body.type_id,
    schema_revision: body.schema_revision,
    identity_mode: body.identity_mode,
    statements,
  }, scopedCatalogParams());
  if (import.meta.client) {
    localStorage.removeItem(draftStorageKey());
  }
  toast.add({
    title: res.data.outcome === 'deduplicated'
      ? t('entity.duplicate.open_existing')
      : t('entity.create_success'),
    color: res.data.outcome === 'deduplicated' ? 'warning' : 'success',
  });
  formRef.value?.markSaved();
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
        submitError.value = displayError(e, t, te);
      }
    } finally {
      submitting.value = false;
    }
  }
}

// Draft persistence (docs/17 §5): saved to localStorage under a per-type
// key, restored into the form on next visit, cleared on successful create.
const draftValues = ref<Record<string, unknown> | undefined>(undefined);
const draftLoaded = ref(false);
const draftKey = computed(() => `${typeAlias.value}:${draftLoaded.value}`);

function draftStorageKey() {
  return `entity_draft_${typeAlias.value}`;
}

function loadDraft() {
  if (!import.meta.client) return;
  const raw = localStorage.getItem(draftStorageKey());
  if (!raw) return;
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (parsed && typeof parsed === 'object' && Object.keys(parsed).length > 0) {
      draftValues.value = parsed;
      draftLoaded.value = true;
      toast.add({ title: t('entity.draft_restored'), color: 'info' });
    }
  } catch {
    localStorage.removeItem(draftStorageKey());
  }
}

function onSaveDraft(values: Record<string, unknown>) {
  if (import.meta.client) {
    localStorage.setItem(draftStorageKey(), JSON.stringify(values));
    draftValues.value = { ...values };
    toast.add({ title: t('entity.draft_saved'), color: 'success' });
  }
}

function goBack() {
  navigateTo(`/objects/${typeAlias.value}`);
}

onMounted(() => {
  loadDraft();
  fetchView();
});
// Header catalog switch: keep the pinned ?catalog= in sync when present
// (the form view then refetches via the pin watcher below), otherwise
// reload the form scoped to the new selection.
watch(selected, (alias) => {
  if (typeof route.query.catalog === 'string' && route.query.catalog) {
    navigateTo({
      query: { ...route.query, catalog: alias || undefined },
      replace: true,
    });
  } else {
    fetchView();
  }
});
watch(() => route.query.catalog, fetchView);
</script>
