<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('admin.predicates_title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('admin.predicates_desc') }}</p>
      </div>
      <UButton icon="i-lucide-plus" @click="openCreateDialog">
        {{ t('admin.create_predicate') }}
      </UButton>
    </div>

    <!-- Info banner about direction -->
    <div class="bg-[var(--ui-bg-muted)] border border-[var(--ui-border)] rounded-md p-4 text-sm">
      <p class="font-medium mb-1">{{ t('admin.direction_info_title') }}</p>
      <p class="text-[var(--ui-text-muted)]">{{ t('admin.direction_info_text') }}</p>
    </div>

    <!-- Filters -->
    <div class="flex gap-2">
      <USelect v-model="filterType" :items="filterTypeOptions" class="w-48" />
      <USelect v-model="filterDirection" :items="directionOptions" class="w-48" />
    </div>

    <!-- Predicates list -->
    <div v-if="loading" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
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
        <div class="p-6 space-y-4 max-h-[80vh] overflow-auto">
          <h2 class="text-lg font-bold">{{ editing ? t('common.edit') : t('admin.create_predicate') }}</h2>

          <UFormField :label="t('admin.predicate_alias')" :hint="t('admin.predicate_alias_hint')">
            <UInput v-model="formData.alias" :disabled="editing" class="w-full" placeholder="person.has_contact" />
          </UFormField>

          <UFormField :label="t('admin.predicate_label')">
            <UInput v-model="formData.label" class="w-full" placeholder="Имеет контакт" />
          </UFormField>

          <UFormField :label="t('admin.predicate_type')">
            <div class="flex gap-4">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="radio" v-model="formData.is_edge" :value="false" />
                <span>{{ t('admin.scalar_predicate') }}</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="radio" v-model="formData.is_edge" :value="true" />
                <span>{{ t('admin.edge_predicate') }}</span>
              </label>
            </div>
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

          <!-- Cardinality -->
          <div class="grid grid-cols-2 gap-2">
            <UFormField :label="t('admin.min_cardinality')">
              <UInput v-model.number="formData.min_cardinality" type="number" min="0" class="w-full" />
            </UFormField>
            <UFormField :label="t('admin.max_cardinality')">
              <UInput v-model.number="formData.max_cardinality" type="number" min="-1" class="w-full" :placeholder="t('admin.unlimited')" />
            </UFormField>
          </div>

          <div class="grid grid-cols-2 gap-2">
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';

definePageMeta({
  middleware: ['auth'],
});

const { t } = useI18n();
const { requireCapability } = useCapability();
requireCapability('admin.view');
const toast = useToast();

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
}

const loading = ref(false);
const predicates = ref<Predicate[]>([]);
const showDialog = ref(false);
const editing = ref(false);
const saving = ref(false);
const filterType = ref('');
const filterDirection = ref('');

const typeOptions = computed(() => [
  { label: t('types.person', 'Физическое лицо'), value: 'person' },
  { label: t('types.legal_entity', 'Юридическое лицо'), value: 'legal_entity' },
  { label: t('types.address', 'Адрес'), value: 'address' },
  { label: t('types.contact_point', 'Контакт'), value: 'contact_point' },
  { label: t('types.document', 'Документ'), value: 'document' },
  { label: t('types.holding_view', 'Холдинг'), value: 'holding_view' },
]);

const filterTypeOptions = computed(() => [
  { label: t('admin.all_types', 'Все типы'), value: '' },
  ...typeOptions.value,
]);

const directionOptions = [
  { label: t('admin.all_directions', 'Все направления'), value: '' },
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
});

const filteredPredicates = computed(() => {
  return predicates.value.filter(p => {
    if (filterType.value && p.domain_type !== filterType.value) return false;
    if (filterDirection.value && p.direction !== filterDirection.value) return false;
    return true;
  });
});

function directionLabel(dir: string): string {
  if (dir === 'directed') return t('admin.direction_directed');
  if (dir === 'symmetric') return t('admin.direction_symmetric');
  return dir;
}

function directionColor(dir: string): string {
  if (dir === 'symmetric') return 'success';
  return 'primary';
}

async function fetchPredicates() {
  loading.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: any[] }>(`${baseUrl.replace(/\/$/, '')}/schemas/predicates`, {
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    predicates.value = (res.data ?? []).map((p: any) => ({
      alias: p.alias ?? p.predicate_id ?? '',
      label: p.label ?? p.display_name ?? p.alias,
      is_edge: p.is_edge ?? false,
      direction: p.direction ?? 'directed',
      domain_type: p.domain_type_id ?? p.domain_type ?? '',
      range_type: p.range_type_id ?? p.range_type ?? '',
      scalar_kind: p.scalar_kind ?? 'text',
      min_cardinality: p.min_cardinality ?? 0,
      max_cardinality: p.max_cardinality ?? 1,
      participates_in_identity: p.participates_in_identity ?? false,
      confidentiality_class: p.confidentiality_class ?? 'public',
      inverse_label: p.inverse_label ?? '',
    }));
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
  };
  showDialog.value = true;
}

function editPredicate(pred: Predicate) {
  editing.value = true;
  formData.value = JSON.parse(JSON.stringify(pred));
  showDialog.value = true;
}

async function savePredicate() {
  saving.value = true;
  try {
    if (editing.value) {
      const idx = predicates.value.findIndex(p => p.alias === formData.value.alias);
      if (idx >= 0) predicates.value[idx] = { ...formData.value };
    } else {
      predicates.value.push({ ...formData.value });
    }
    showDialog.value = false;
    toast.add({ title: t('admin.predicate_saved'), color: 'success' });
  } finally {
    saving.value = false;
  }
}

async function deletePredicate(pred: Predicate) {
  const idx = predicates.value.findIndex(p => p.alias === pred.alias);
  if (idx >= 0) {
    predicates.value.splice(idx, 1);
    toast.add({ title: t('admin.predicate_deleted'), color: 'success' });
  }
}

onMounted(fetchPredicates);
</script>
