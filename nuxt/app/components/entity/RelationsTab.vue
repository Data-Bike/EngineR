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

    <!-- Sections by relation group -->
    <div v-for="group in relationGroups" :key="group.id" class="space-y-3">
      <button
        type="button"
        class="flex w-full items-center gap-2 text-left"
        @click="toggleGroup(group.id)"
      >
        <UIcon
          :name="collapsedGroups.has(group.id) ? 'i-lucide-chevron-right' : 'i-lucide-chevron-down'"
          class="size-4 text-[var(--ui-text-muted)]"
        />
        <h3 class="font-medium">{{ group.label }}</h3>
        <UBadge variant="subtle" size="sm">{{ group.count }}</UBadge>
      </button>

      <div v-show="!collapsedGroups.has(group.id)" class="space-y-4 ps-6">
        <div v-for="section in group.sections" :key="section.key" class="space-y-2">
          <div class="flex items-center gap-2">
            <h4 class="text-sm font-medium text-[var(--ui-text-muted)]">{{ section.label }}</h4>
            <UBadge variant="subtle" size="xs">
              {{ section.direction === 'outgoing' ? t('entity.graph.direction_outgoing') : t('entity.graph.direction_incoming') }}
            </UBadge>
            <span class="text-xs text-[var(--ui-text-muted)]">({{ section.items.length }})</span>
          </div>

          <div class="space-y-2">
            <div
              v-for="item in section.items"
              :key="item.id"
              class="rounded-md border border-[var(--ui-border)] p-3 hover:border-[var(--ui-border-muted)]"
              :class="item.trans ? 'border-s-2 border-s-[var(--ui-primary)]' : ''"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="flex-1 min-w-0">
                  <div class="flex flex-wrap items-center gap-2">
                    <NuxtLink :to="`/entities/${item.targetPublicId}`" class="font-medium hover:underline">
                      {{ item.title }}
                    </NuxtLink>
                    <UBadge v-if="item.targetType" variant="subtle" size="xs">
                      {{ t(`types.${item.targetType}`, item.targetType) }}
                    </UBadge>
                    <UBadge v-if="item.trans" color="primary" variant="subtle" size="xs" icon="i-lucide-waypoints">
                      {{ t('entity.relations.trans_object') }}
                    </UBadge>
                  </div>

                  <!-- Preview fields (the type's preview view) -->
                  <div v-if="item.preview.length" class="mt-1.5 flex flex-wrap gap-x-4 gap-y-1">
                    <span
                      v-for="field in item.preview"
                      :key="field.field"
                      class="text-xs text-[var(--ui-text-muted)]"
                    >
                      <span class="font-medium">{{ fieldLabel(field.field) }}:</span>
                      {{ formatValue(field.value) }}
                    </span>
                  </div>

                  <p v-if="item.validFrom" class="text-xs text-[var(--ui-text-muted)] mt-1.5">
                    {{ t('entity.relations.valid_from') }}: {{ formatDateTime(item.validFrom) }}
                    <span v-if="item.validTo"> · {{ t('entity.relations.valid_to') }}: {{ formatDateTime(item.validTo) }}</span>
                  </p>
                  <p v-if="item.source" class="text-xs text-[var(--ui-text-muted)] mt-1">
                    {{ t('entity.relations.source') }}: {{ item.source }}
                  </p>

                  <!-- Trans-object: onward edges (the real destinations) -->
                  <div v-if="item.trans" class="mt-2">
                    <button
                      v-if="item.through.length"
                      type="button"
                      class="flex items-center gap-1 text-xs text-[var(--ui-primary)]"
                      @click="toggleThrough(item.id)"
                    >
                      <UIcon
                        :name="collapsedThrough.has(item.id) ? 'i-lucide-chevron-right' : 'i-lucide-chevron-down'"
                        class="size-3.5"
                      />
                      {{ t('entity.relations.via') }} ({{ item.through.length }})
                    </button>
                    <div
                      v-show="!collapsedThrough.has(item.id)"
                      class="mt-1.5 space-y-1.5 border-s border-[var(--ui-border)] ps-3"
                    >
                      <div
                        v-for="edge in item.through"
                        :key="edge.relationId"
                        class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm"
                      >
                        <UIcon name="i-lucide-corner-down-right" class="size-3.5 text-[var(--ui-text-muted)]" />
                        <span class="text-xs text-[var(--ui-text-muted)]">{{ predicateLabel(edge.predicate) }}:</span>
                        <NuxtLink :to="`/entities/${edge.targetPublicId}`" class="font-medium hover:underline">
                          {{ edge.title }}
                        </NuxtLink>
                        <UBadge variant="subtle" size="xs">{{ t(`types.${edge.targetType}`, edge.targetType) }}</UBadge>
                        <span
                          v-for="field in edge.preview"
                          :key="field.field"
                          class="text-xs text-[var(--ui-text-muted)]"
                        >
                          {{ fieldLabel(field.field) }}: {{ formatValue(field.value) }}
                        </span>
                      </div>
                      <p v-if="!item.through.length" class="text-xs text-[var(--ui-text-muted)]">
                        {{ t('entity.relations.no_through') }}
                      </p>
                    </div>
                  </div>
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
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="relationGroups.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
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
import type { EntityRelation, RelationThroughEdge } from '@enginer/api-client';

const props = defineProps<{
  entityId: string;
  typeId: string;
}>();

const { t, te } = useI18n();
const toast = useToast();
const api = useApi();

interface PreviewItem {
  field: string;
  value: unknown;
}

interface ThroughItem {
  relationId: string;
  predicate: string;
  direction: 'outgoing' | 'incoming';
  targetPublicId: string;
  title: string;
  targetType: string;
  preview: PreviewItem[];
}

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
  group: string;
  preview: PreviewItem[];
  trans: boolean;
  through: ThroughItem[];
}

interface PredicateSection {
  key: string;
  predicateId: string;
  label: string;
  direction: 'outgoing' | 'incoming';
  items: RelationItem[];
}

interface RelationGroup {
  id: string;
  label: string;
  count: number;
  sections: PredicateSection[];
}

const relationGroups = ref<RelationGroup[]>([]);
const collapsedGroups = ref<Set<string>>(new Set());
const collapsedThrough = ref<Set<string>>(new Set());
const showHistorical = ref(false);
const schemaPredicates = ref<Array<{
  alias: string;
  label: string;
  domain_type: string;
  range_type: string;
  is_edge: boolean;
}>>([]);
const schemaTypes = ref<Array<{ id: string; display_name: string }>>([]);
/** Relation-group registry from the published schema: id → {order, label}. */
const groupRegistry = ref(new Map<string, { order: number; label: string }>());

function groupLabel(id: string): string {
  const key = `entity.relations.group.${id}`;
  if (te(key)) return t(key);
  return groupRegistry.value.get(id)?.label || id;
}

function predicateLabel(alias: string): string {
  const key = `entity.relations.predicate.${alias}`;
  if (te(key)) return t(key);
  return schemaPredicates.value.find((p) => p.alias === alias)?.label || alias;
}

function fieldLabel(alias: string): string {
  const key = `entity.relations.preview.${alias}`;
  if (te(key)) return t(key);
  return schemaPredicates.value.find((p) => p.alias === alias)?.label || alias;
}

function formatValue(value: unknown): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (typeof value === 'object') {
    const obj = value as Record<string, unknown>;
    if (obj.amount !== undefined && obj.currency) return `${obj.amount} ${obj.currency}`;
    if (obj.amount !== undefined && obj.unit) return `${obj.amount} ${obj.unit}`;
  }
  return JSON.stringify(value);
}

function toggleGroup(id: string) {
  const next = new Set(collapsedGroups.value);
  if (next.has(id)) next.delete(id); else next.add(id);
  collapsedGroups.value = next;
}

function toggleThrough(id: string) {
  const next = new Set(collapsedThrough.value);
  if (next.has(id)) next.delete(id); else next.add(id);
  collapsedThrough.value = next;
}

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
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
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
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
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
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

function mapThrough(edge: RelationThroughEdge): ThroughItem {
  return {
    relationId: edge.relation_id,
    predicate: edge.predicate,
    direction: edge.direction === 'outgoing' ? 'outgoing' : 'incoming',
    targetPublicId: edge.target_entity_id,
    title: edge.target_display_name,
    targetType: edge.target_type,
    preview: (edge.preview ?? []).map((f) => ({ field: f.field, value: f.value })),
  };
}

async function fetchRelations() {
  try {
    const res = await api.entities.getRelations(props.entityId, {
      as_of: showHistorical.value ? undefined : new Date().toISOString().slice(0, 10),
    });
    const relations = (res.data ?? []) as EntityRelation[];
    const groupMap = new Map<string, Map<string, PredicateSection>>();
    for (const rel of relations) {
      const predicateId = rel.predicate ?? 'unknown';
      const direction = rel.direction === 'outgoing' ? 'outgoing' : 'incoming';
      const groupId = rel.group || 'general';
      if (!groupMap.has(groupId)) groupMap.set(groupId, new Map());
      const sections = groupMap.get(groupId)!;
      const key = `${predicateId}:${direction}`;
      if (!sections.has(key)) {
        sections.set(key, {
          key,
          predicateId,
          label: predicateLabel(predicateId),
          direction,
          items: [],
        });
      }
      const relId = rel.relation_id ?? '';
      if (!relId) {
        console.warn('Relation missing relation_id', rel);
      }
      sections.get(key)!.items.push({
        id: relId,
        predicateId,
        direction,
        targetPublicId: rel.target_entity_id ?? '',
        title: rel.target_display_name ?? '',
        targetType: rel.target_type ?? '',
        validFrom: rel.valid_from,
        validTo: rel.valid_to ?? undefined,
        source: rel.source,
        group: groupId,
        preview: (rel.preview ?? []).map((f) => ({ field: f.field, value: f.value })),
        trans: rel.trans ?? false,
        through: (rel.through ?? []).map(mapThrough),
      });
    }
    relationGroups.value = Array.from(groupMap.entries())
      .map(([id, sections]) => ({
        id,
        label: groupLabel(id),
        count: Array.from(sections.values()).reduce((n, s) => n + s.items.length, 0),
        sections: Array.from(sections.values()),
      }))
      .sort((a, b) => {
        const oa = groupRegistry.value.get(a.id)?.order;
        const ob = groupRegistry.value.get(b.id)?.order;
        if (oa !== undefined || ob !== undefined) {
          return (oa ?? Number.MAX_SAFE_INTEGER) - (ob ?? Number.MAX_SAFE_INTEGER);
        }
        return a.label.localeCompare(b.label);
      });
    collapsedGroups.value = new Set();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
    relationGroups.value = [];
  }
}

function setHistorical(value: boolean) {
  if (showHistorical.value === value) return;
  showHistorical.value = value;
  fetchRelations();
}

async function fetchRelationMetadata() {
  try {
    const [predicatesResponse, typesResponse, published] = await Promise.all([
      api.schemas.listPredicates(),
      api.schemas.listTypes(),
      api.schemas.listPublished(),
    ]);
    groupRegistry.value = new Map(
      (((published.data?.[0]?.relation_groups ?? []) as any[]) ?? [])
        .map((g) => [String(g.id ?? ''), { order: Number(g.order ?? 0), label: String(g.label ?? '') }])
        .filter(([id]) => id) as [string, { order: number; label: string }][],
    );
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
      description: displayError(cause, t, te),
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
