<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('admin.predicates_title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('admin.predicates_desc') }}</p>
      </div>
      <div class="flex gap-2">
        <UButton variant="outline" icon="i-lucide-layers" @click="openGroupsDialog">
          {{ t('admin.manage_groups') }}
        </UButton>
        <UButton icon="i-lucide-plus" @click="openCreateDialog">
          {{ t('admin.create_predicate') }}
        </UButton>
      </div>
    </div>

    <!-- Info banner about direction -->
    <div class="bg-[var(--ui-bg-muted)] border border-[var(--ui-border)] rounded-md p-4 text-sm">
      <p class="font-medium mb-1">{{ t('admin.direction_info_title') }}</p>
      <p class="text-[var(--ui-text-muted)]">{{ t('admin.direction_info_text') }}</p>
    </div>

    <!-- Filters -->
    <div class="flex flex-wrap gap-2">
      <USelect v-model="filterType" :items="filterTypeOptions" class="w-full sm:w-48" />
      <USelect v-model="filterDirection" :items="directionOptions" class="w-full sm:w-48" />
    </div>

    <!-- Predicates list -->
    <div v-if="loading" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>
    <div v-else-if="filteredPredicates.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>
    <div v-else class="space-y-2">
      <UCard v-for="pred in filteredPredicates" :key="pred.alias">
        <div class="flex items-start justify-between">
          <div class="flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <p class="font-medium">{{ pred.label }}</p>
              <UBadge variant="subtle" size="sm">{{ pred.alias }}</UBadge>
              <UBadge :variant="pred.is_edge ? 'solid' : 'outline'" size="sm" :color="pred.is_edge ? 'primary' : 'neutral'">
                {{ pred.is_edge ? t('admin.edge_predicate') : t('admin.scalar_predicate') }}
              </UBadge>
              <UBadge v-if="pred.is_edge" variant="soft" size="sm" :color="directionColor(pred.direction)">
                {{ directionLabel(pred.direction) }}
              </UBadge>
              <UBadge v-if="pred.participates_in_identity" variant="soft" size="sm" color="warning">
                {{ t('admin.identity_field') }}
              </UBadge>
              <UBadge v-if="pred.is_edge && pred.group" variant="soft" size="sm" color="info" icon="i-lucide-layers">
                {{ groupLabel(pred.group) }}
              </UBadge>
            </div>
            <div class="mt-2 text-sm text-[var(--ui-text-muted)] space-y-0.5">
              <p v-if="pred.is_edge">
                <strong>{{ t('admin.domain') }}:</strong> {{ pred.domain_type }} →
                <strong>{{ t('admin.range') }}:</strong> {{ pred.range_type }}
              </p>
              <p v-else>
                <strong>{{ t('admin.domain') }}:</strong> {{ pred.domain_type }} ·
                <strong>{{ t('admin.kind') }}:</strong> {{ pred.scalar_kind }}
              </p>
              <p>
                <strong>{{ t('admin.cardinality') }}:</strong>
                {{ pred.min_cardinality }}..{{ pred.max_cardinality === -1 ? '∞' : pred.max_cardinality }}
                · <strong>{{ t('admin.confidentiality') }}:</strong> {{ pred.confidentiality_class }}
              </p>
              <p v-if="pred.is_edge && pred.inverse_label">
                <strong>{{ t('admin.inverse_label') }}:</strong> {{ pred.inverse_label }}
              </p>
            </div>
          </div>
          <div class="flex gap-2">
            <UButton size="sm" variant="outline" @click="editPredicate(pred)">
              {{ t('common.edit') }}
            </UButton>
            <UButton size="sm" variant="ghost" color="error" icon="i-lucide-trash" @click="deletePredicate(pred)" />
          </div>
        </div>
      </UCard>
    </div>

    <!-- Create/Edit Dialog -->
    <UModal v-model:open="showDialog">
      <template #content>
        <div class="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
          <h2 class="text-lg font-bold">{{ editing ? t('common.edit') : t('admin.create_predicate') }}</h2>

          <UFormField :label="t('admin.predicate_alias')" :hint="t('admin.predicate_alias_hint')">
            <UInput v-model="formData.alias" :disabled="editing" class="w-full" placeholder="person.has_contact" />
          </UFormField>

          <UFormField :label="t('admin.predicate_label')">
            <UInput v-model="formData.label" class="w-full" placeholder="Имеет контакт" />
          </UFormField>

          <UFormField :label="t('admin.predicate_type')">
            <USelect :model-value="formData.is_edge ? 'edge' : 'scalar'" :items="predicateTypeOptions" class="w-full" @update:model-value="(v: string) => formData.is_edge = v === 'edge'" />
          </UFormField>

          <UFormField :label="t('admin.domain_type')">
            <USelect v-model="formData.domain_type" :items="typeOptions" class="w-full" />
          </UFormField>

          <UFormField v-if="formData.is_edge" :label="t('admin.range_type')">
            <USelect v-model="formData.range_type" :items="typeOptions" class="w-full" />
          </UFormField>

          <UFormField v-else :label="t('admin.scalar_kind')">
            <USelect v-model="formData.scalar_kind" :items="scalarKindOptions" class="w-full" />
          </UFormField>

          <!-- Direction (only for edges) -->
          <div v-if="formData.is_edge" class="space-y-3 border border-[var(--ui-border)] rounded-md p-3">
            <p class="font-medium text-sm">{{ t('admin.direction_section') }}</p>
            <UFormField :label="t('admin.direction')">
              <USelect v-model="formData.direction" :items="directionOptionsDetailed" class="w-full" />
            </UFormField>
            <UFormField :label="t('admin.inverse_label')" :hint="t('admin.inverse_label_hint')">
              <UInput v-model="formData.inverse_label" class="w-full" placeholder="Управляется" />
            </UFormField>
            <div v-if="formData.direction === 'directed'" class="bg-[var(--ui-bg-muted)] rounded p-2 text-xs text-[var(--ui-text-muted)]">
              <p>{{ t('admin.direction_directed_hint') }}</p>
            </div>
            <div v-else-if="formData.direction === 'symmetric'" class="bg-[var(--ui-bg-muted)] rounded p-2 text-xs text-[var(--ui-text-muted)]">
              <p>{{ t('admin.direction_symmetric_hint') }}</p>
            </div>
          </div>

          <UFormField v-if="formData.is_edge" :label="t('admin.relation_group')" :hint="t('admin.relation_group_hint')">
            <USelect v-model="formData.group" :items="groupOptions" class="w-full" />
          </UFormField>

          <!-- Cardinality -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <UFormField :label="t('admin.min_cardinality')">
              <UInput v-model.number="formData.min_cardinality" type="number" min="0" class="w-full" />
            </UFormField>
            <UFormField :label="t('admin.max_cardinality')">
              <UInput v-model.number="formData.max_cardinality" type="number" min="-1" class="w-full" :placeholder="t('admin.unlimited')" />
            </UFormField>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <UFormField :label="t('admin.confidentiality')">
              <USelect v-model="formData.confidentiality_class" :items="confidentialityOptions" class="w-full" />
            </UFormField>
            <UFormField :label="t('admin.identity_field')">
              <USwitch v-model="formData.participates_in_identity" />
            </UFormField>
          </div>

          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showDialog = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton @click="savePredicate" :loading="saving">
              {{ t('common.save') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Relation groups registry editor -->
    <UModal v-model:open="showGroupsDialog">
      <template #content>
        <div class="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
          <h2 class="text-lg font-bold">{{ t('admin.manage_groups') }}</h2>
          <p class="text-sm text-[var(--ui-text-muted)]">{{ t('admin.manage_groups_hint') }}</p>

          <div v-for="(g, idx) in groupsForm" :key="idx" class="flex items-end gap-2 border border-[var(--ui-border)] rounded-md p-3">
            <UFormField :label="t('admin.group_id')" class="flex-1">
              <UInput v-model="g.id" class="w-full" placeholder="employment" :disabled="g.id === 'general'" />
            </UFormField>
            <UFormField :label="t('admin.group_label')" class="flex-1">
              <UInput v-model="g.label" class="w-full" :placeholder="t('admin.group_label_placeholder')" />
            </UFormField>
            <UFormField :label="t('admin.group_order')" class="w-24">
              <UInput v-model.number="g.order" type="number" class="w-full" />
            </UFormField>
            <UButton size="sm" variant="ghost" color="error" icon="i-lucide-x" :disabled="g.id === 'general'" @click="groupsForm.splice(idx, 1)" />
          </div>

          <UButton variant="outline" size="sm" icon="i-lucide-plus" @click="groupsForm.push({ id: '', label: '', order: (groupsForm.at(-1)?.order ?? 0) + 10 })">
            {{ t('admin.add_group') }}
          </UButton>

          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showGroupsDialog = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton :loading="savingGroups" @click="saveGroups">
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

const { t } = useI18n();
const toast = useToast();
const api = useApi();

interface Predicate {
  alias: string;
  label: string;
  is_edge: boolean;
  direction: string;
  domain_type: string;
  range_type: string;
  scalar_kind: string;
  min_cardinality: number;
  max_cardinality: number;
  participates_in_identity: boolean;
  confidentiality_class: string;
  inverse_label: string;
  /** Relation group id from the schema's relation_groups registry. */
  group: string;
}

interface RelationGroup {
  id: string;
  label: string;
  order: number;
}

const loading = ref(false);
const predicates = ref<Predicate[]>([]);
const showDialog = ref(false);
const editing = ref(false);
const saving = ref(false);
const filterType = ref('all');
const filterDirection = ref('all');
const schemaTypes = ref<Array<{ id: string; display_name: string }>>([]);

const typeOptions = computed(() => schemaTypes.value.map((schemaType) => ({
  label: schemaType.display_name,
  value: schemaType.id,
})));

const filterTypeOptions = computed(() => [
  { label: t('admin.all_types', 'Все типы'), value: 'all' },
  ...typeOptions.value,
]);

const directionOptions = [
  { label: t('admin.all_directions', 'Все направления'), value: 'all' },
  { label: t('admin.direction_directed', 'Направленная'), value: 'directed' },
  { label: t('admin.direction_symmetric', 'Симметричная'), value: 'symmetric' },
];

const directionOptionsDetailed = [
  { label: t('admin.direction_directed', 'Направленная (прямая и обратная связь)'), value: 'directed' },
  { label: t('admin.direction_symmetric', 'Симметричная (без направления)'), value: 'symmetric' },
];

const scalarKindOptions = [
  { label: 'Text', value: 'text' },
  { label: 'Date', value: 'date' },
  { label: 'Integer', value: 'integer' },
  { label: 'Decimal', value: 'decimal' },
  { label: 'Boolean', value: 'boolean' },
  { label: 'Money', value: 'money' },
  { label: 'UUID', value: 'uuid' },
  { label: 'URI', value: 'uri' },
];

const confidentialityOptions = [
  { label: t('admin.conf_public', 'Публичные'), value: 'public' },
  { label: t('admin.conf_restricted', 'Ограниченные'), value: 'restricted' },
  { label: t('admin.conf_confidential', 'Конфиденциальные'), value: 'confidential' },
];

const predicateTypeOptions = [
  { label: t('admin.edge_predicate'), value: 'edge' },
  { label: t('admin.scalar_predicate'), value: 'scalar' },
];

const formData = ref<Predicate>({
  alias: '',
  label: '',
  is_edge: true,
  direction: 'directed',
  domain_type: 'person',
  range_type: 'person',
  scalar_kind: 'text',
  min_cardinality: 0,
  max_cardinality: 1,
  participates_in_identity: false,
  confidentiality_class: 'public',
  inverse_label: '',
  group: '',
});

const relationGroups = ref<RelationGroup[]>([]);
const showGroupsDialog = ref(false);
const savingGroups = ref(false);
const groupsForm = ref<RelationGroup[]>([]);

/** Localized group label: i18n key first, registry label as fallback. */
function groupLabel(id: string): string {
  const key = `entity.relations.group.${id}`;
  const translated = t(key);
  if (translated !== key) return translated;
  return relationGroups.value.find((g) => g.id === id)?.label || id;
}

const groupOptions = computed(() => [
  { label: t('admin.no_group'), value: '' },
  ...[...relationGroups.value]
    .sort((a, b) => a.order - b.order)
    .map((g) => ({ label: groupLabel(g.id), value: g.id })),
]);

const filteredPredicates = computed(() => {
  return predicates.value.filter(p => {
    if (filterType.value && filterType.value !== 'all' && p.domain_type !== filterType.value) return false;
    if (filterDirection.value && filterDirection.value !== 'all' && p.direction !== filterDirection.value) return false;
    return true;
  });
});

function directionLabel(dir: string): string {
  if (dir === 'directed') return t('admin.direction_directed');
  if (dir === 'symmetric') return t('admin.direction_symmetric');
  return dir;
}

function directionColor(dir: string): 'success' | 'primary' {
  if (dir === 'symmetric') return 'success';
  return 'primary';
}

async function fetchPredicates() {
  loading.value = true;
  try {
    const [res, typesResponse, published] = await Promise.all([
      api.schemas.listPredicates(),
      api.schemas.listTypes(),
      api.schemas.listPublished(),
    ]);
    relationGroups.value = ((published.data?.[0]?.relation_groups ?? []) as any[])
      .map((g) => ({
        id: String(g.id ?? ''),
        label: String(g.label ?? g.id ?? ''),
        order: Number(g.order ?? 0),
      }))
      .filter((g) => g.id);
    schemaTypes.value = (typesResponse.data ?? []).map((schemaType: any) => ({
      id: schemaType.id ?? schemaType.type_id ?? '',
      display_name: schemaType.display_name ?? schemaType.id ?? '',
    }));

    const SCALAR_KINDS = new Set(['text', 'date', 'integer', 'decimal', 'boolean', 'money', 'uuid', 'uri', 'string', 'number']);

    predicates.value = (res.data ?? []).map((p: any) => {
      const range = p.range_type_id ?? p.range_type ?? p.range ?? '';
      const isEdge = p.is_edge ?? (range !== '' && !SCALAR_KINDS.has(range));
      return {
        alias: p.alias ?? p.predicate_id ?? '',
        label: p.label ?? p.display_name ?? p.alias ?? '',
        is_edge: isEdge,
        direction: p.direction ?? 'directed',
        domain_type: p.domain_type_id ?? p.domain_type ?? p.domain ?? '',
        range_type: isEdge ? range : '',
        scalar_kind: p.scalar_kind || (isEdge ? 'text' : range),
        min_cardinality: p.min_cardinality ?? p.cardinality_min ?? 0,
        max_cardinality: p.max_cardinality ?? p.cardinality_max ?? 1,
        participates_in_identity: p.participates_in_identity ?? p.identity ?? false,
        confidentiality_class: p.confidentiality_class ?? 'public',
        inverse_label: p.inverse_label ?? '',
        group: p.group ?? '',
      };
    });
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    loading.value = false;
  }
}

function openCreateDialog() {
  editing.value = false;
  formData.value = {
    alias: '',
    label: '',
    is_edge: true,
    direction: 'directed',
    domain_type: 'person',
    range_type: 'person',
    scalar_kind: 'text',
    min_cardinality: 0,
    max_cardinality: 1,
    participates_in_identity: false,
    confidentiality_class: 'public',
    inverse_label: '',
    group: '',
  };
  showDialog.value = true;
}

function openGroupsDialog() {
  groupsForm.value = JSON.parse(JSON.stringify(
    [...relationGroups.value].sort((a, b) => a.order - b.order),
  ));
  showGroupsDialog.value = true;
}

function editPredicate(pred: Predicate) {
  editing.value = true;
  formData.value = JSON.parse(JSON.stringify(pred));
  showDialog.value = true;
}

const { applySchemaMutation } = useSchemaMutation();

function entryFromForm(): Record<string, unknown> {
  const base: Record<string, unknown> = {
    stable_alias: formData.value.alias,
    label: formData.value.label,
    direction: formData.value.direction,
    min_cardinality: formData.value.min_cardinality,
    max_cardinality: formData.value.max_cardinality,
    participates_in_identity: formData.value.participates_in_identity,
    confidentiality_class: formData.value.confidentiality_class,
    inverse_label: formData.value.inverse_label,
  };
  if (formData.value.is_edge) {
    base.is_edge = true;
    base.domain = formData.value.domain_type;
    base.range = formData.value.range_type;
    if (formData.value.group) base.group = formData.value.group;
  } else {
    base.is_edge = false;
    base.scalar_kind = formData.value.scalar_kind;
  }
  return base;
}

async function savePredicate() {
  saving.value = true;
  const alias = formData.value.alias;
  const wasEditing = editing.value;
  try {
    await applySchemaMutation({
      name: wasEditing
        ? `${t('common.edit')}: ${alias}`
        : `${t('admin.create_predicate')}: ${alias}`,
      reason: wasEditing ? `predicate ${alias} updated` : `predicate ${alias} added`,
      changes: [{
        kind: wasEditing ? 'update' : 'add',
        resource_kind: 'predicate',
        resource_id: alias,
        summary: formData.value.label || alias,
        breaking: false,
      }],
      mutate(_types, predicates) {
        const entry = entryFromForm();
        const idx = predicates.findIndex(
          (item) => (item.stable_alias ?? item.alias) === alias,
        );
        if (idx >= 0) predicates[idx] = { ...predicates[idx], ...entry };
        else predicates.push(entry);
      },
    });
    showDialog.value = false;
    toast.add({ title: t('admin.predicate_saved'), color: 'success' });
    await fetchPredicates();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    saving.value = false;
  }
}

async function saveGroups() {
  savingGroups.value = true;
  try {
    const entries = groupsForm.value
      .filter((g) => g.id.trim())
      .map((g) => ({ id: g.id.trim(), label: g.label.trim() || g.id.trim(), order: g.order }));
    // The general bucket must always exist — it is the fallback for
    // predicates without an explicit group.
    if (!entries.some((g) => g.id === 'general')) {
      entries.push({ id: 'general', label: t('admin.no_group'), order: 999 });
    }
    await applySchemaMutation({
      name: t('admin.manage_groups'),
      reason: 'relation groups updated',
      changes: [{
        kind: 'update',
        resource_kind: 'relation_groups',
        resource_id: 'relation_groups',
        summary: t('admin.manage_groups'),
        breaking: false,
      }],
      mutate(_types, _predicates, groups) {
        groups.splice(0, groups.length, ...entries);
      },
    });
    showGroupsDialog.value = false;
    toast.add({ title: t('admin.groups_saved'), color: 'success' });
    await fetchPredicates();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    savingGroups.value = false;
  }
}

async function deletePredicate(pred: Predicate) {
  try {
    await applySchemaMutation({
      name: `${t('common.delete')}: ${pred.alias}`,
      reason: `predicate ${pred.alias} retired`,
      changes: [{
        kind: 'retire',
        resource_kind: 'predicate',
        resource_id: pred.alias,
        summary: pred.label || pred.alias,
        breaking: true,
      }],
      mutate(_types, predicates) {
        const idx = predicates.findIndex(
          (item) => (item.stable_alias ?? item.alias) === pred.alias,
        );
        if (idx >= 0) predicates.splice(idx, 1);
      },
    });
    toast.add({ title: t('admin.predicate_deleted'), color: 'success' });
    await fetchPredicates();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  }
}

onMounted(fetchPredicates);
</script>
