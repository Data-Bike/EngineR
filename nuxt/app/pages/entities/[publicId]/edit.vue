<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-5xl">
    <div class="flex items-center justify-between gap-4">
      <div>
        <h1 class="text-2xl font-bold">{{ t('entity.edit') }}</h1>
        <p class="mt-1 text-[var(--ui-text-muted)]">
          {{ entity?.display_name ?? publicId }}
        </p>
      </div>
      <UButton variant="outline" icon="i-lucide-x" @click="goBack">
        {{ t('common.cancel') }}
      </UButton>
    </div>

    <div v-if="loading" class="py-12 text-center text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>
    <div v-else-if="error" class="py-12 text-center">
      <p class="mb-3 text-red-500">{{ error }}</p>
      <UButton variant="outline" @click="load">{{ t('common.retry') }}</UButton>
    </div>
    <EntityFormRenderer
      ref="formRef"
      v-else-if="entity && view"
      :view="view"
      :initial-values="entity.fields ?? {}"
      :validation-errors="validationErrors"
      :submitting="submitting"
      :submit-label="t('common.save')"
      @submit="save"
      @cancel="goBack"
    />
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { useBreadcrumbLabels } from '~/composables/useBreadcrumbLabels';
import type {
  Entity,
  FieldValidationError,
  FormSubmission,
  RawStatementValue,
  ViewDefinition,
} from '@enginer/api-client';

definePageMeta({ middleware: ['auth', capability('entity.edit')] });

const { t, te } = useI18n();
const route = useRoute();
const api = useApi();
// Entity-scoped calls resolve the entity by id — the catalog selector is a
// listing filter, not a security scope. Schema lookups use the entity's own
// catalog so an alias shared between catalogs resolves to the right type.
const toast = useToast();
const { setBreadcrumbLabel } = useBreadcrumbLabels();

const publicId = computed(() => route.params.publicId as string);
const entity = ref<Entity | null>(null);
const view = ref<ViewDefinition | null>(null);
const loading = ref(false);
const submitting = ref(false);
const error = ref<string | null>(null);
const validationErrors = ref<Record<string, FieldValidationError>>({});
const formRef = ref<{ markSaved: () => void } | null>(null);

async function load() {
  loading.value = true;
  error.value = null;
  try {
    const entityResponse = await api.entities.get(publicId.value);
    entity.value = entityResponse.data;
    setBreadcrumbLabel(
      publicId.value,
      entityResponse.data.display_name ?? publicId.value,
    );
    const viewResponse = await api.schemas.getTypeView(
      entityResponse.data.type_id,
      'form',
      entityResponse.data.catalog ? { catalog: entityResponse.data.catalog } : undefined,
    );
    view.value = viewResponse.data as ViewDefinition;
  } catch (cause: unknown) {
    error.value = displayError(cause, t, te);
  } finally {
    loading.value = false;
  }
}

async function save(submission: FormSubmission) {
  if (!entity.value) return;
  submitting.value = true;
  validationErrors.value = {};
  try {
    const response = await api.entities.update(publicId.value, {
      revision: entity.value.revision,
      // Send the raw form value: the server coerces it by the declared
      // field kind (coerce_field_input) — wrapping everything in text would
      // silently store dates/money/geo as text scalars.
      fields: submission.fields.map((field) => ({
        predicate_id: field.predicate_id,
        value: (field.value ?? '') as RawStatementValue,
      })),
    });
    entity.value = response.data;
    formRef.value?.markSaved();
    toast.add({ title: t('common.save'), color: 'success' });
    await navigateTo(`/entities/${publicId.value}`);
  } catch (cause: unknown) {
    const apiError = cause as { validation_errors?: Record<string, FieldValidationError> };
    if (apiError.validation_errors) {
      validationErrors.value = apiError.validation_errors;
    } else {
      toast.add({
        title: t('common.error'),
        description: displayError(cause, t, te),
        color: 'error',
      });
    }
  } finally {
    submitting.value = false;
  }
}

function goBack() {
  navigateTo(`/entities/${publicId.value}`);
}

onMounted(load);
</script>
