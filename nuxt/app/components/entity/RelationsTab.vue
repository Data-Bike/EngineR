<template>
  <div class="space-y-6">
    <!-- Sections by relation predicate (docs/17 §6) -->
    <div v-for="section in relationSections" :key="section.predicateId" class="space-y-3">
      <!-- Section header with direction label -->
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <h3 class="font-medium">{{ t(section.labelKey) }}</h3>
          <UBadge variant="subtle" size="sm">
            {{ section.direction === 'outgoing' ? t('entity.graph.direction_outgoing') : t('entity.graph.direction_incoming') }}
          </UBadge>
          <span class="text-sm text-[var(--ui-text-muted)]">({{ section.items.length }})</span>
        </div>
        <div class="flex gap-2">
          <!-- Effective/historical switch (docs/17 §6) -->
          <UTabs
            v-model="section.viewMode"
            :items="viewModeOptions"
            size="xs"
          />
          <!-- Add/link existing (docs/17 §6) -->
          <UButton size="sm" variant="outline" icon="i-lucide-plus" @click="addRelation(section)">
            {{ t('entity.relations.add') }}
          </UButton>
          <UButton size="sm" variant="ghost" icon="i-lucide-link" @click="linkExisting(section)">
            {{ t('entity.relations.link_existing') }}
          </UButton>
        </div>
      </div>

      <!-- Relation items -->
      <div v-if="section.items.length === 0" class="text-sm text-[var(--ui-text-muted)] py-2">
        {{ t('entity.empty') }}
      </div>
      <div v-else class="space-y-2">
        <div
          v-for="item in filteredItems(section)"
          :key="item.id"
          class="rounded-md border border-[var(--ui-border)] p-3 hover:border-[var(--ui-border-muted)]"
        >
          <div class="flex items-start justify-between">
            <div class="flex-1">
              <!-- Title -->
              <NuxtLink :to="`/entities/${item.targetPublicId}`" class="font-medium hover:underline">
                {{ item.title }}
              </NuxtLink>
              <!-- Qualifiers (docs/17 §6) -->
              <div v-if="item.qualifiers && item.qualifiers.length > 0" class="flex flex-wrap gap-1 mt-1">
                <UBadge
                  v-for="qual in item.qualifiers"
                  :key="qual.label"
                  variant="subtle"
                  size="sm"
                >
                  {{ qual.label }}: {{ qual.value }}
                </UBadge>
              </div>
              <!-- Validity period (docs/17 §6) -->
              <p v-if="item.validFrom || item.validTo" class="text-xs text-[var(--ui-text-muted)] mt-1">
                {{ t('entity.relations.valid_from') }}: {{ item.validFrom ?? '—' }}
                {{ t('entity.relations.valid_to') }}: {{ item.validTo ?? '—' }}
              </p>
              <!-- Source (docs/17 §6) -->
              <p v-if="item.source" class="text-xs text-[var(--ui-text-muted)] mt-1">
                {{ t('entity.relations.source') }}: {{ item.source }}
              </p>
            </div>
            <!-- Inline edit for simple safe relations only (docs/17 §6) -->
            <div v-if="item.inlineEditable" class="flex gap-1">
              <UButton size="xs" variant="ghost" icon="i-lucide-pencil" @click="editRelation(section, item)" />
              <UButton size="xs" variant="ghost" color="error" icon="i-lucide-trash" @click="removeRelation(section, item)" />
            </div>
          </div>
        </div>

        <!-- Cursor pagination for high-degree nodes (docs/17 §6) -->
        <div v-if="section.cursor" class="flex justify-center pt-2">
          <UButton size="sm" variant="outline" @click="loadMore(section)">
            {{ t('entity.load_more') }}
          </UButton>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="relationSections.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';

defineProps<{
  entityId: string;
  typeId: string;
}>();

const { t } = useI18n();

interface RelationQualifier {
  label: string;
  value: string;
}

interface RelationItem {
  id: string;
  targetPublicId: string;
  title: string;
  qualifiers: RelationQualifier[];
  validFrom?: string;
  validTo?: string;
  source?: string;
  inlineEditable: boolean;
  isHistorical: boolean;
}

interface RelationSection {
  predicateId: string;
  labelKey: string;
  direction: 'outgoing' | 'incoming';
  items: RelationItem[];
  cursor?: string;
  viewMode: 'effective' | 'historical';
}

const viewModeOptions = computed(() => [
  { key: 'effective', label: t('entity.relations.effective') },
  { key: 'historical', label: t('entity.relations.historical') },
]);

// Placeholder sections — would be fetched from API
const relationSections = ref<RelationSection[]>([]);

function filteredItems(section: RelationSection): RelationItem[] {
  if (section.viewMode === 'effective') {
    return section.items.filter((i) => !i.isHistorical);
  }
  return section.items;
}

async function fetchRelations() {
  // Would call api.entities.query with relation filters
  // Placeholder: empty
  relationSections.value = [];
}

function addRelation(_section: RelationSection) {
  // Open relation editor dialog
}

function linkExisting(_section: RelationSection) {
  // Open entity autocomplete to link existing entity
}

function editRelation(_section: RelationSection, _item: RelationItem) {
  // Inline edit for simple safe relations only (docs/17 §6)
}

function removeRelation(_section: RelationSection, _item: RelationItem) {
  // Remove relation
}

async function loadMore(_section: RelationSection) {
  // Cursor pagination
}

onMounted(fetchRelations);
</script>
