<template>
  <div class="p-6 space-y-6">
    <!-- Loading state -->
    <div v-if="loading" class="text-center py-12 text-[var(--ui-text-muted)]">
      {{ t('entity.loading') }}
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="text-center py-12">
      <p class="text-red-500 mb-2">{{ error }}</p>
      <UButton variant="outline" @click="fetchEntity">
        {{ t('common.retry') }}
      </UButton>
    </div>

    <!-- Entity card -->
    <template v-else-if="entity">
      <!-- Header (docs/17 §4) -->
      <div class="flex items-start justify-between">
        <div>
          <h1 class="text-2xl font-bold">
            {{ entity.public_id }}
          </h1>
          <div class="flex items-center gap-2 mt-2">
            <UBadge :color="entity.status === 'active' ? 'success' : 'neutral'">
              {{ t(`entity.status.${entity.status}`) }}
            </UBadge>
            <UBadge variant="subtle">
              {{ t(`entity.tabs.overview`) }} rev. {{ entity.revision }}
            </UBadge>
          </div>
        </div>
        <div class="flex gap-2">
          <UButton
            v-if="entity.status === 'active'"
            icon="i-lucide-archive"
            variant="outline"
            @click="showRetire = true"
          >
            {{ t('entity.retire') }}
          </UButton>
        </div>
      </div>

      <!-- Tabs (docs/17 §4) -->
      <UTabs :items="tabItems" v-model="activeTab">
        <template #content="{ item }">
          <div v-if="item.key === 'overview'" class="py-4">
            <EntityOverview :entity="entity" />
          </div>
          <div v-else-if="item.key === 'relations'" class="py-4">
            <RelationsTab :entity-id="entity.public_id" :type-id="entity.type_id" />
          </div>
          <div v-else-if="item.key === 'graph'" class="py-4">
            <GraphView :root-entity-id="entity.public_id" :root-type-id="entity.type_id" />
          </div>
          <div v-else-if="item.key === 'checks'" class="py-4">
            <ChecksTab :entity-id="entity.public_id" />
          </div>
          <div v-else-if="item.key === 'risks'" class="py-4">
            <RisksTab :entity-id="entity.public_id" />
          </div>
          <div v-else-if="item.key === 'tasks'" class="py-4">
            <TasksTab :entity-id="entity.public_id" />
          </div>
          <div v-else-if="item.key === 'documents'" class="py-4">
            <DocumentsTab :entity-id="entity.public_id" />
          </div>
          <div v-else-if="item.key === 'history'" class="py-4">
            <HistoryTab :entity-id="entity.public_id" :current-revision="entity.revision" />
          </div>
        </template>
      </UTabs>
    </template>

    <!-- Retire dialog -->
    <UModal v-model:open="showRetire">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.retire') }}</h2>
          <UFormField label="Reason">
            <UInput v-model="retireReason" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showRetire = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton
              color="error"
              :loading="retiring"
              @click="doRetire"
            >
              {{ t('entity.retire') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import type { Entity } from '@enginer/api-client';

definePageMeta({
  middleware: 'auth',
});

const { t } = useI18n();
const route = useRoute();
const api = useApi();

const publicId = computed(() => route.params.publicId as string);
const entity = ref<Entity | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const activeTab = ref(0);
const showRetire = ref(false);
const retireReason = ref('');
const retiring = ref(false);

const tabItems = computed(() => [
  { key: 'overview', label: t('entity.tabs.overview') },
  { key: 'relations', label: t('entity.tabs.relations') },
  { key: 'graph', label: t('entity.tabs.graph') },
  { key: 'checks', label: t('entity.tabs.checks') },
  { key: 'risks', label: t('entity.tabs.risks') },
  { key: 'tasks', label: t('entity.tabs.tasks') },
  { key: 'documents', label: t('entity.tabs.documents') },
  { key: 'history', label: t('entity.tabs.history') },
]);

async function fetchEntity() {
  loading.value = true;
  error.value = null;
  try {
    const res = await api.entities.get(publicId.value);
    entity.value = res.data;
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : t('entity.error');
  } finally {
    loading.value = false;
  }
}

async function doRetire() {
  if (!entity.value) return;
  retiring.value = true;
  try {
    await api.entities.retire(publicId.value, { reason: retireReason.value });
    showRetire.value = false;
    retireReason.value = '';
    await fetchEntity();
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : t('entity.error');
  } finally {
    retiring.value = false;
  }
}

onMounted(fetchEntity);
</script>
