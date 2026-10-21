<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('admin.types_title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('admin.types_desc') }}</p>
      </div>
      <UButton icon="i-lucide-plus" @click="openCreateDialog">
        {{ t('admin.create_type') }}
      </UButton>
    </div>

    <!-- Types list -->
    <div v-if="loading" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>
    <div v-else class="space-y-3">
      <UCard v-for="type in types" :key="type.type_id">
        <div class="flex items-start justify-between">
          <div class="flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <p class="font-medium">{{ type.display_name }}</p>
              <UBadge variant="subtle" size="sm">{{ type.type_id }}</UBadge>
              <UBadge v-if="type.entity_class" variant="outline" size="sm">{{ type.entity_class }}</UBadge>
              <UBadge v-if="type.trans" variant="soft" size="sm" color="primary" icon="i-lucide-waypoints">
                {{ t('admin.trans_object') }}
              </UBadge>
              <UBadge v-if="type.entity_count !== undefined" variant="soft" size="sm" color="primary">
                {{ type.entity_count }} {{ t('admin.objects_count') }}
              </UBadge>
            </div>
            <p class="text-sm text-[var(--ui-text-muted)] mt-1">{{ type.description }}</p>
            <div v-if="type.fields && type.fields.length > 0" class="mt-2">
              <p class="text-xs font-medium text-[var(--ui-text-muted)] mb-1">{{ t('admin.fields_count', { n: type.fields.length }) }}</p>
              <div class="flex flex-wrap gap-1">
                <UBadge v-for="field in type.fields.slice(0, 8)" :key="field.alias" variant="subtle" size="sm">
                  {{ field.label ? t(field.label, field.alias) : field.alias }}
                </UBadge>
                <UBadge v-if="type.fields.length > 8" variant="outline" size="sm">
                  +{{ type.fields.length - 8 }}
                </UBadge>
              </div>
            </div>
          </div>
          <div class="flex gap-2">
            <UButton size="sm" variant="outline" @click="editType(type)">
              {{ t('common.edit') }}
            </UButton>
            <UButton size="sm" variant="ghost" color="error" icon="i-lucide-trash" :aria-label="t('common.delete')" :title="t('common.delete')" @click="deleteType(type)" />
          </div>
        </div>
      </UCard>
    </div>

    <!-- Create/Edit Dialog -->
    <UModal v-model:open="showDialog">
      <template #content>
        <div class="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
          <h2 class="text-lg font-bold">{{ editing ? t('common.edit') : t('admin.create_type') }}</h2>

          <UFormField :label="t('admin.type_id')" :hint="t('admin.type_id_hint')">
            <UInput v-model="formData.type_id" :disabled="editing" class="w-full" placeholder="person, legal_entity, ..." />
          </UFormField>

          <UFormField :label="t('admin.type_display_name')">
            <UInput v-model="formData.display_name" class="w-full" placeholder="Физическое лицо" />
          </UFormField>

          <UFormField :label="t('admin.type_description')">
            <UTextarea v-model="formData.description" class="w-full" :rows="2" />
          </UFormField>

          <UFormField :label="t('admin.entity_class')">
            <USelect v-model="formData.entity_class" :items="entityClassOptions" class="w-full" />
          </UFormField>

          <UFormField :label="t('admin.trans_object')" :hint="t('admin.trans_object_hint')">
            <USwitch v-model="formData.trans" />
          </UFormField>

          <!-- Fields section -->
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <p class="font-medium">{{ t('admin.fields') }}</p>
              <UButton size="xs" variant="outline" icon="i-lucide-plus" @click="addField">
                {{ t('admin.add_field') }}
              </UButton>
            </div>
            <div v-for="(field, idx) in formData.fields" :key="idx" class="border border-[var(--ui-border)] rounded-md p-3 space-y-2">
              <div class="flex items-center justify-between">
                <p class="text-sm font-medium">{{ t('admin.field_n', { n: idx + 1 }) }}</p>
                <UButton size="xs" variant="ghost" color="error" icon="i-lucide-x" @click="removeField(idx)" />
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <UFormField :label="t('admin.field_alias')">
                  <UInput v-model="field.alias" class="w-full" placeholder="inn, full_name, ..." />
                </UFormField>
                <UFormField :label="t('admin.field_label')">
                  <UInput v-model="field.label" class="w-full" placeholder="ИНН, ФИО, ..." />
                </UFormField>
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-2">
                <UFormField :label="t('admin.field_kind')">
                  <USelect v-model="field.kind" :items="fieldKindOptions" class="w-full" />
                </UFormField>
                <UFormField :label="t('admin.field_required')">
                  <USwitch v-model="field.required" />
                </UFormField>
                <UFormField :label="t('admin.field_identity')">
                  <USwitch v-model="field.identity" />
                </UFormField>
                <UFormField :label="t('admin.field_preview')" :title="t('admin.field_preview_hint')">
                  <USwitch v-model="field.preview" />
                </UFormField>
              </div>
              <UFormField v-if="field.kind === 'entity_reference'" :label="t('admin.range_type')">
                <USelect v-model="field.target_type_id" :items="typeOptions" class="w-full" />
              </UFormField>
            </div>
            <p v-if="previewAliases.length" class="text-xs text-[var(--ui-text-muted)]">
              {{ t('admin.preview_order_hint') }}:
              <span class="font-mono">{{ previewAliases.join(' · ') }}</span>
            </p>
          </div>

          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showDialog = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton @click="saveType" :loading="saving">
              {{ t('common.save') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';

definePageMeta({
  middleware: ['auth', capability('admin.pages.access')],
});

const { t, te } = useI18n();
const toast = useToast();
const api = useApi();
const { catalogParams, selected: selectedCatalog } = useCatalog();

interface TypeField {
  alias: string;
  label: string;
  kind: string;
  required: boolean;
  identity: boolean;
  /** Included in the type's `preview` view (compact cards, relation lists). */
  preview?: boolean;
  target_type_id?: string;
}

interface SchemaType {
  type_id: string;
  display_name: string;
  description: string;
  entity_class: string;
  entity_count?: number;
  /** Ordered field aliases of the preview view (from the published schema). */
  preview?: string[];
  /** Trans-object (junction) flag. */
  trans: boolean;
  fields: TypeField[];
}

const loading = ref(false);
const types = ref<SchemaType[]>([]);
const showDialog = ref(false);
const editing = ref(false);
const saving = ref(false);

const entityClassOptions = [
  { label: t('admin.class_data', 'Данные'), value: 'data' },
  { label: t('admin.class_domain', 'Доменный'), value: 'domain' },
  { label: t('admin.class_meta', 'Мета'), value: 'meta' },
];

const fieldKindOptions = [
  { label: 'Text', value: 'text' },
  { label: 'Date', value: 'date' },
  { label: 'Integer', value: 'integer' },
  { label: 'Decimal', value: 'decimal' },
  { label: 'Boolean', value: 'boolean' },
  { label: 'Money', value: 'money' },
  { label: 'Entity Reference', value: 'entity_reference' },
  { label: 'Document', value: 'document' },
];

const typeOptions = computed(() => types.value.map((schemaType) => ({
  label: schemaType.display_name,
  value: schemaType.type_id,
})));

const formData = ref<SchemaType>({
  type_id: '',
  display_name: '',
  description: '',
  entity_class: 'data',
  trans: false,
  fields: [],
});

/** Preview field aliases in declared order — the type's `preview` view. */
const previewAliases = computed(() =>
  formData.value.fields.filter((f) => f.preview && f.alias).map((f) => f.alias),
);

async function fetchTypes() {
  loading.value = true;
  try {
    const res = await api.schemas.listTypes(catalogParams());

    // Map basic type info — API returns `id` not `type_id`
    const baseTypes = (res.data ?? []).map((t: any) => ({
      type_id: t.type_id ?? t.id ?? t.alias ?? t.name ?? '',
      display_name: t.display_name ?? t.name ?? t.id ?? '',
      description: t.description ?? '',
      entity_class: t.entity_class ?? 'data',
      entity_count: t.entity_count as number | undefined,
      preview: (t.preview ?? []) as string[],
      trans: t.trans ?? false,
      fields: [] as TypeField[],
    }));

    // Load fields for each type via view API (in parallel)
    const enriched = await Promise.all(baseTypes.map(async (tp) => {
      try {
        const vres = await api.schemas.getTypeView(tp.type_id, 'form', catalogParams());
        const data = vres.data as { sections?: Array<{ fields?: Array<any> }> } | undefined;
        const fields: TypeField[] = [];
        if (data?.sections) {
          for (const section of data.sections) {
            if (section.fields) {
              for (const f of section.fields) {
                const alias = f.alias ?? f.predicate_id ?? '';
                fields.push({
                  alias,
                  label: f.label_key ?? f.alias ?? f.predicate_id ?? '',
                  kind: f.kind ?? 'text',
                  required: f.required ?? false,
                  identity: f.identity ?? false,
                  preview: (tp.preview ?? []).includes(alias),
                  target_type_id: f.target_type_id,
                });
              }
            }
          }
        }
        return { ...tp, fields };
      } catch {
        return tp;
      }
    }));

    types.value = enriched;
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    loading.value = false;
  }
}

function openCreateDialog() {
  editing.value = false;
  formData.value = {
    type_id: '',
    display_name: '',
    description: '',
    entity_class: 'data',
    trans: false,
    fields: [],
  };
  showDialog.value = true;
}

function editType(type: SchemaType) {
  editing.value = true;
  formData.value = JSON.parse(JSON.stringify(type));
  // Fields loaded from the form view don't carry the preview flag —
  // restore it from the type's preview list.
  const previewSet = new Set(type.preview ?? []);
  for (const f of formData.value.fields) f.preview = previewSet.has(f.alias);
  showDialog.value = true;
}

function addField() {
  formData.value.fields.push({
    alias: '',
    label: '',
    kind: 'text',
    required: false,
    identity: false,
    preview: false,
  });
}

function removeField(idx: number) {
  formData.value.fields.splice(idx, 1);
}

const { applySchemaMutation } = useSchemaMutation();

function entryFromForm(): Record<string, unknown> {
  return {
    type_id: formData.value.type_id,
    stable_alias: formData.value.type_id,
    entity_class: formData.value.entity_class,
    label: formData.value.display_name,
    description: formData.value.description,
    trans: formData.value.trans,
    // Always present: an empty list explicitly clears the declared preview —
    // the relations API then falls back to its field-order heuristic, and the
    // merge in `mutate` must not resurrect the previous list.
    preview: previewAliases.value,
    fields: formData.value.fields.map((field) => ({
      alias: field.alias,
      label: field.label,
      kind: field.kind,
      required: field.required,
      identity: field.identity,
      ...(field.target_type_id ? { target_type_id: field.target_type_id } : {}),
    })),
  };
}

async function saveType() {
  saving.value = true;
  const typeId = formData.value.type_id;
  const wasEditing = editing.value;
  try {
    await applySchemaMutation({
      name: wasEditing
        ? `${t('common.edit')}: ${typeId}`
        : `${t('admin.create_type')}: ${typeId}`,
      reason: wasEditing ? `type ${typeId} updated` : `type ${typeId} added`,
      changes: [{
        kind: wasEditing ? 'update' : 'add',
        resource_kind: 'type',
        resource_id: typeId,
        summary: formData.value.display_name || typeId,
        breaking: false,
      }],
      mutate(types) {
        const entry = entryFromForm();
        const idx = types.findIndex(
          (item) => (item.type_id ?? item.stable_alias) === typeId,
        );
        if (idx >= 0) types[idx] = { ...types[idx], ...entry };
        else types.push(entry);
      },
    });
    showDialog.value = false;
    toast.add({ title: t('admin.type_saved'), color: 'success' });
    await fetchTypes();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    saving.value = false;
  }
}

async function deleteType(type: SchemaType) {
  try {
    await applySchemaMutation({
      name: `${t('common.delete')}: ${type.type_id}`,
      reason: `type ${type.type_id} retired`,
      changes: [{
        kind: 'retire',
        resource_kind: 'type',
        resource_id: type.type_id,
        summary: type.display_name || type.type_id,
        breaking: true,
      }],
      mutate(types) {
        const idx = types.findIndex(
          (item) => (item.type_id ?? item.stable_alias) === type.type_id,
        );
        if (idx >= 0) types.splice(idx, 1);
      },
    });
    toast.add({ title: t('admin.type_deleted'), color: 'success' });
    await fetchTypes();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

onMounted(fetchTypes);
// Header catalog switch re-scopes the type list.
watch(selectedCatalog, fetchTypes);
</script>
