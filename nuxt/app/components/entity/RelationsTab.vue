<template>
  <div class="space-y-6">
    <!-- Top-level add button -->
    <div class="flex flex-wrap justify-between gap-2">
      <div class="flex gap-1" role="group" :aria-label="t('entity.tabs.relations')">
        <UButton
          size="sm"
          :variant="showHistorical ? 'ghost' : 'solid'"
          @click="setHistorical(false)"
        >{{ t('entity.relations.effective') }}</UButton>
        <UButton
          size="sm"
          :variant="showHistorical ? 'solid' : 'ghost'"
          @click="setHistorical(true)"
        >{{ t('entity.relations.historical') }}</UButton>
      </div>
      <div class="flex gap-2">
      <UButton size="sm" variant="outline" icon="i-lucide-plus" @click="openAddDialog">
        {{ t('entity.relations.add') }}
      </UButton>
      <UButton size="sm" variant="ghost" icon="i-lucide-link" @click="openLinkDialog">
        {{ t('entity.relations.link_existing') }}
      </UButton>
      </div>
    </div>

    <!-- Sections by relation predicate -->
    <div v-for="section in relationSections" :key="section.key" class="space-y-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <h3 class="font-medium">{{ section.label }}</h3>
          <UBadge variant="subtle" size="sm">
            {{ section.direction === 'outgoing' ? t('entity.graph.direction_outgoing') : t('entity.graph.direction_incoming') }}
          </UBadge>
          <span class="text-sm text-[var(--ui-text-muted)]">({{ section.items.length }})</span>
        </div>
      </div>

      <div v-if="section.items.length === 0" class="text-sm text-[var(--ui-text-muted)] py-2">
        {{ t('entity.empty') }}
      </div>
      <div v-else class="space-y-2">
        <div
          v-for="item in section.items"
          :key="item.id"
          class="rounded-md border border-[var(--ui-border)] p-3 hover:border-[var(--ui-border-muted)]"
        >
          <div class="flex items-start justify-between">
            <div class="flex-1">
              <NuxtLink :to="`/entities/${item.targetPublicId}`" class="font-medium hover:underline">
                {{ item.title }}
              </NuxtLink>
              <p v-if="item.targetType" class="text-xs text-[var(--ui-text-muted)] mt-0.5">
                {{ t(`types.${item.targetType}`, item.targetType) }}
              </p>
              <p v-if="item.validFrom" class="text-xs text-[var(--ui-text-muted)] mt-1">
                {{ t('entity.relations.valid_from') }}: {{ item.validFrom }}
                <span v-if="item.validTo"> · {{ t('entity.relations.valid_to') }}: {{ item.validTo }}</span>
              </p>
              <p v-if="item.source" class="text-xs text-[var(--ui-text-muted)] mt-1">
                {{ t('entity.relations.source') }}: {{ item.source }}
              </p>
            </div>
            <UButton
              size="xs"
              variant="ghost"
              color="error"
              icon="i-lucide-trash"
              @click="removeRelation(item)"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="relationSections.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>

    <!-- Add Relation Dialog -->
    <UModal v-model:open="showAddDialog">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.relations.add') }}</h2>
          <UFormField :label="t('entity.relations.predicate_label')">
            <USelect
              v-model="newRelation.predicate"
              :items="predicateOptions"
              class="w-full"
            />
          </UFormField>
          <UFormField :label="t('entity.relations.target_type')">
            <USelect
              v-model="newRelation.targetType"
              :items="typeOptions"
              class="w-full"
            />
          </UFormField>
          <UFormField :label="t('entity.relations.target_name')">
            <UInput v-model="newRelation.targetName" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.relations.valid_from')">
            <UInput v-model="newRelation.validFrom" type="date" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showAddDialog = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton @click="submitAddRelation" :loading="submitting">
              {{ t('common.save') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Link Existing Dialog -->
    <UModal v-model:open="showLinkDialog">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.relations.link_existing') }}</h2>
          <UFormField :label="t('entity.relations.predicate_label')">
            <USelect
              v-model="newRelation.predicate"
              :items="predicateOptions"
              class="w-full"
            />
          </UFormField>
          <UFormField :label="t('entity.relations.search_entity')">
            <UInput
              v-model="entitySearch"
              :placeholder="t('entity.relations.search_placeholder')"
              class="w-full"
              @input="searchEntities"
            />
          </UFormField>
          <div v-if="searchResults.length > 0" class="max-h-48 overflow-auto border border-[var(--ui-border)] rounded-md">
            <div
              v-for="result in searchResults"
              :key="result.public_id"
              class="p-2 hover:bg-[var(--ui-bg-muted)] cursor-pointer border-b border-[var(--ui-border)] last:border-0"
              @click="selectEntity(result)"
            >
              <p class="font-medium text-sm">{{ result.display_name }}</p>
              <p class="text-xs text-[var(--ui-text-muted)]">{{ result.type_id }} · {{ result.public_id }}</p>
            </div>
          </div>
          <div v-if="selectedEntity" class="p-2 bg-[var(--ui-bg-muted)] rounded-md">
            <p class="text-sm">{{ t('entity.relations.selected') }}: <strong>{{ selectedEntity.display_name }}</strong></p>
          </div>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showLinkDialog = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton @click="submitLinkRelation" :loading="submitting" :disabled="!selectedEntity">
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

const props = defineProps<{
  entityId: string;
  typeId: string;
}>();

const { t } = useI18n();
const toast = useToast();
const api = useApi();

interface RelationItem {
  id: string;
  predicateId: string;
  direction: 'outgoing' | 'incoming';
  targetPublicId: string;
  title: string;
  targetType: string;
  validFrom?: string;
  validTo?: string;
  source?: string;
}

interface RelationSection {
  key: string;
  predicateId: string;
  label: string;
  direction: 'outgoing' | 'incoming';
  items: RelationItem[];
}

const relationSections = ref<RelationSection[]>([]);
const showHistorical = ref(false);
const schemaPredicates = ref<Array<{
  alias: string;
  label: string;
  domain_type: string;
  range_type: string;
  is_edge: boolean;
}>>([]);
const schemaTypes = ref<Array<{ id: string; display_name: string }>>([]);

// Available predicates for this entity type
const predicateOptions = computed(() => schemaPredicates.value
  .filter((predicate) => predicate.is_edge && predicate.domain_type === props.typeId)
  .map((predicate) => ({ label: predicate.label, value: predicate.alias })));

const typeOptions = computed(() => schemaTypes.value.map((schemaType) => ({
  label: schemaType.display_name,
  value: schemaType.id,
})));

// Dialog state
const showAddDialog = ref(false);
const showLinkDialog = ref(false);
const submitting = ref(false);
const entitySearch = ref('');
const searchResults = ref<any[]>([]);
const selectedEntity = ref<any | null>(null);

const newRelation = ref({
  predicate: 'managed_by',
  targetType: 'person',
  targetName: '',
  validFrom: new Date().toISOString().split('T')[0],
});

function openAddDialog() {
  const defaultPredicate = schemaPredicates.value.find((predicate) => predicate.is_edge && predicate.domain_type === props.typeId);
  newRelation.value = {
    predicate: defaultPredicate?.alias ?? '',
    targetType: defaultPredicate?.range_type ?? '',
    targetName: '',
    validFrom: new Date().toISOString().split('T')[0],
  };
  showAddDialog.value = true;
}

function openLinkDialog() {
  const defaultPredicate = schemaPredicates.value.find((predicate) => predicate.is_edge && predicate.domain_type === props.typeId);
  newRelation.value = {
    predicate: defaultPredicate?.alias ?? '',
    targetType: defaultPredicate?.range_type ?? '',
    targetName: '',
    validFrom: new Date().toISOString().split('T')[0],
  };
  selectedEntity.value = null;
  entitySearch.value = '';
  searchResults.value = [];
  showLinkDialog.value = true;
}

async function searchEntities() {
  if (entitySearch.value.length < 2) {
    searchResults.value = [];
    return;
  }
  try {
    const res = await api.entities.query({
      from: { type_id: newRelation.value.targetType, alias: 'e' },
      search: entitySearch.value,
      page: { size: 10 },
    });
    searchResults.value = res.data?.rows ?? [];
  } catch {
    searchResults.value = [];
  }
}

function selectEntity(entity: any) {
  selectedEntity.value = entity;
  searchResults.value = [];
  entitySearch.value = entity.display_name ?? entity.public_id;
}

async function submitAddRelation() {
  submitting.value = true;
  try {
    await api.entities.addRelation(props.entityId, {
      predicate: newRelation.value.predicate,
      direction: 'outgoing',
      target_display_name: newRelation.value.targetName,
      target_type: newRelation.value.targetType,
      valid_from: newRelation.value.validFrom,
      source: 'manual',
    });
    showAddDialog.value = false;
    toast.add({ title: t('entity.relations.added'), color: 'success' });
    await fetchRelations();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    submitting.value = false;
  }
}

async function submitLinkRelation() {
  if (!selectedEntity.value) return;
  submitting.value = true;
  try {
    await api.entities.addRelation(props.entityId, {
      predicate: newRelation.value.predicate,
      direction: 'outgoing',
      target_entity_id: selectedEntity.value.public_id,
      target_display_name: selectedEntity.value.display_name,
      target_type: selectedEntity.value.type_id,
      valid_from: newRelation.value.validFrom,
      source: 'manual',
    });
    showLinkDialog.value = false;
    toast.add({ title: t('entity.relations.linked'), color: 'success' });
    await fetchRelations();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    submitting.value = false;
  }
}

async function removeRelation(item: RelationItem) {
  try {
    await api.entities.deleteRelation(props.entityId, item.id);
    toast.add({ title: t('entity.relations.removed'), color: 'success' });
    await fetchRelations();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  }
}

interface ApiRelation {
  relation_id?: string;
  predicate?: string;
  predicate_id?: string;
  direction: string;
  target_entity_id?: string;
  target_public_id?: string;
  target_display_name?: string;
  target_title?: string;
  target_type?: string;
  valid_from?: string;
  valid_to?: string | null;
  source?: string;
}

async function fetchRelations() {
  try {
    const res = await api.entities.getRelations(props.entityId, {
      as_of: showHistorical.value ? undefined : new Date().toISOString().slice(0, 10),
    });
    const relations = (res.data ?? []) as ApiRelation[];
    const sectionMap = new Map<string, RelationSection>();
    for (const rel of relations) {
      const predicateId = rel.predicate_id ?? rel.predicate ?? 'unknown';
      const direction = (rel.direction === 'outbound' || rel.direction === 'outgoing') ? 'outgoing' : 'incoming';
      const key = `${predicateId}:${direction}`;
      if (!sectionMap.has(key)) {
        sectionMap.set(key, {
          key,
          predicateId,
          label: t(`entity.relations.predicate.${predicateId}`, predicateId),
          direction,
          items: [],
        });
      }
      const section = sectionMap.get(key)!;
      const relId = rel.relation_id ?? '';
      if (!relId) {
        console.warn('Relation missing relation_id', rel);
      }
      section.items.push({
        id: relId,
        predicateId,
        direction,
        targetPublicId: rel.target_public_id ?? rel.target_entity_id ?? '',
        title: rel.target_title ?? rel.target_display_name ?? '',
        targetType: rel.target_type ?? '',
        validFrom: rel.valid_from,
        validTo: rel.valid_to ?? undefined,
        source: rel.source,
      });
    }
    relationSections.value = Array.from(sectionMap.values());
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
    relationSections.value = [];
  }
}

function setHistorical(value: boolean) {
  if (showHistorical.value === value) return;
  showHistorical.value = value;
  fetchRelations();
}

async function fetchRelationMetadata() {
  try {
    const [predicatesResponse, typesResponse] = await Promise.all([
      api.schemas.listPredicates(),
      api.schemas.listTypes(),
    ]);
    schemaPredicates.value = (predicatesResponse.data ?? []).map((predicate: any) => ({
      alias: predicate.alias ?? predicate.predicate_id ?? '',
      label: predicate.label ?? predicate.display_name ?? predicate.alias ?? '',
      domain_type: predicate.domain_type ?? predicate.domain ?? '',
      range_type: predicate.range_type ?? predicate.range ?? '',
      is_edge: predicate.is_edge ?? false,
    }));
    schemaTypes.value = (typesResponse.data ?? []).map((schemaType: any) => ({
      id: schemaType.id ?? schemaType.type_id ?? '',
      display_name: schemaType.display_name ?? schemaType.id ?? '',
    }));
  } catch (cause: unknown) {
    toast.add({
      title: t('common.error'),
      description: cause instanceof Error ? cause.message : String(cause),
      color: 'error',
    });
  }
}

watch(() => newRelation.value.predicate, (alias) => {
  const predicate = schemaPredicates.value.find((item) => item.alias === alias);
  if (predicate) newRelation.value.targetType = predicate.range_type;
});

onMounted(() => {
  fetchRelationMetadata();
  fetchRelations();
});
</script>
