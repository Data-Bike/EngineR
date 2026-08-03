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
          <div class="py-4">
                        <EntityOverview v-if="item.value === 'overview'" :entity="entity" />
            <RelationsTab v-else-if="item.value === 'relations'" :entity-id="entity.public_id" :type-id="entity.type_id" />
            <GraphView v-else-if="item.value === 'graph'" :root-entity-id="entity.public_id" :root-type-id="entity.type_id" />
            <ChecksTab v-else-if="item.value === 'checks'" :entity-id="entity.public_id" />
            <RisksTab v-else-if="item.value === 'risks'" :entity-id="entity.public_id" />
            <TasksTab v-else-if="item.value === 'tasks'" :entity-id="entity.public_id" />
            <DocumentsTab v-else-if="item.value === 'documents'" :entity-id="entity.public_id" />
            <HistoryTab v-else-if="item.value === 'history'" :entity-id="entity.public_id" :current-revision="entity.revision" />
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
const toast = useToast();

const publicId = computed(() => route.params.publicId as string);
const entity = ref<Entity | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const activeTab = ref('overview');
const showRetire = ref(false);
const retireReason = ref('');
const retiring = ref(false);

const tabItems = computed(() => [
  { value: 'overview', label: t('entity.tabs.overview') },
  { value: 'relations', label: t('entity.tabs.relations') },
  { value: 'graph', label: t('entity.tabs.graph') },
  { value: 'checks', label: t('entity.tabs.checks') },
  { value: 'risks', label: t('entity.tabs.risks') },
  { value: 'tasks', label: t('entity.tabs.tasks') },
  { value: 'documents', label: t('entity.tabs.documents') },
  { value: 'history', label: t('entity.tabs.history') },
]);

async function fetchEntity() {
  loading.value = true;
  error.value = null;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: Entity }>(`${baseUrl.replace(/\/$/, '')}/entities/${publicId.value}`, {
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
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
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/entities/${publicId.value}/retire`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: { reason: retireReason.value },
    });
    showRetire.value = false;
    retireReason.value = '';
    toast.add({ title: t('entity.retired_success'), color: 'success' });
    await fetchEntity();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    retiring.value = false;
  }
}

onMounted(fetchEntity);
</script>
