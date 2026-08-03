<template>
  <div class="p-6 space-y-6">
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
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type {
  ViewDefinition,
  FormSubmission,
  FieldValidationError,
} from '@enginer/api-client';

definePageMeta({
  middleware: 'auth',
});

const { t } = useI18n();
const route = useRoute();
const toast = useToast();

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
  type_id: string;
  match_type: 'exact' | 'fuzzy';
  score?: number;
  explanation?: string;
}

async function fetchView() {
  loadingView.value = true;
  viewError.value = null;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: ViewDefinition }>(
      `${baseUrl.replace(/\/$/, '')}/schemas/types/${typeAlias.value}/view`,
      {
        query: { view_type: 'form' },
        credentials: 'include',
        headers: { Accept: 'application/json' },
      },
    );
    view.value = res.data;
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

async function checkDuplicates(_body: FormSubmission): Promise<DuplicateMatch[]> {
  // Query for entities with matching identity fields (docs/17 §5)
  // Placeholder: would call api.entities.query with identity field filters
  return [];
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
  const config = useRuntimeConfig();
  const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
  const res = await $fetch<{ data: { public_id: string } }>(
    `${baseUrl.replace(/\/$/, '')}/entities`,
    {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: {
        type_id: body.type_id,
        schema_revision: body.schema_revision,
        identity_mode: body.identity_mode,
        statements,
      },
    },
  );
  toast.add({ title: t('entity.create_success'), color: 'success' });
  // Navigate to the created entity card
  navigateTo(`/entities/${res.data.public_id}`);
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
