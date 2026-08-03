<template>
  <div class="p-6 space-y-6">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('schema.title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('nav.admin') }}</p>
      </div>
      <UButton icon="i-lucide-plus" @click="showCreateDraft = true">
        {{ t('schema.create_draft') }}
      </UButton>
    </div>

    <!-- Tabs: Drafts / Published -->
    <UTabs v-model="activeTab" :items="tabs">
      <template #content="{ item }">
        <!-- Drafts -->
        <div v-if="item.value === 'drafts'" class="py-4">
          <div v-if="draftsLoading" class="text-center py-8 text-[var(--ui-text-muted)]">
            {{ t('common.loading') }}
          </div>
          <div v-else-if="drafts.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
            {{ t('entity.empty') }}
          </div>
          <div v-else class="space-y-2">
            <UCard
              v-for="draft in drafts"
              :key="draft.id"
              class="hover:border-[var(--ui-border-muted)]"
            >
              <div class="flex items-center justify-between">
                <div>
                  <p class="font-medium">{{ draft.name }}</p>
                  <p class="text-sm text-[var(--ui-text-muted)]">
                    rev. {{ draft.revision }} · {{ draft.state }}
                  </p>
                </div>
                <div class="flex gap-2">
                  <UButton
                    size="sm"
                    variant="outline"
                    @click="validateDraft(draft.id)"
                  >
                    {{ t('schema.validate') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    @click="impactDraft(draft.id)"
                  >
                    {{ t('schema.impact') }}
                  </UButton>
                  <UButton
                    size="sm"
                    @click="publishDraft(draft.id)"
                  >
                    {{ t('schema.publish') }}
                  </UButton>
                </div>
              </div>
            </UCard>
          </div>
        </div>

        <!-- Published -->
        <div v-else-if="item.value === 'published'" class="py-4">
          <div v-if="publishedLoading" class="text-center py-8 text-[var(--ui-text-muted)]">
            {{ t('common.loading') }}
          </div>
          <div v-else-if="published.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
            {{ t('entity.empty') }}
          </div>
          <div v-else class="space-y-2">
            <UCard
              v-for="rev in published"
              :key="rev.revision_id"
            >
              <div class="flex items-center justify-between">
                <div>
                  <p class="font-medium">Revision {{ rev.revision_id }}</p>
                  <p class="text-sm text-[var(--ui-text-muted)]">
                    {{ rev.semantic_version }} · {{ rev.state }}
                  </p>
                </div>
                <UButton
                  size="sm"
                  variant="outline"
                  color="error"
                  @click="retireRevision(String(rev.revision_id))"
                >
                  {{ t('schema.retire') }}
                </UButton>
              </div>
            </UCard>
          </div>
        </div>
      </template>
    </UTabs>

    <!-- Create draft dialog -->
    <UModal v-model:open="showCreateDraft">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('schema.create_draft') }}</h2>
          <UFormField label="Name">
            <UInput v-model="newDraftName" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showCreateDraft = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton @click="createDraft" :loading="creating">
              {{ t('common.save') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import type { SchemaRevision } from '@enginer/api-client';

definePageMeta({
  middleware: ['auth'],
});

const { t } = useI18n();
const { requireCapability } = useCapability();
requireCapability('admin.view');
const api = useApi();
const toast = useToast();

const activeTab = ref('drafts');
const tabs = computed(() => [
  { value: 'drafts', label: t('schema.drafts') },
  { value: 'published', label: t('schema.published') },
]);

// Drafts
interface Draft {
  id: string;
  name: string;
  revision: number;
  state: string;
}

const drafts = ref<Draft[]>([]);
const draftsLoading = ref(false);
const showCreateDraft = ref(false);
const newDraftName = ref('');
const creating = ref(false);

async function fetchDrafts() {
  draftsLoading.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: any[] }>(`${baseUrl.replace(/\/$/, '')}/schemas/drafts`, {
      method: 'GET',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    drafts.value = (res.data ?? []).map((d: any) => ({
      id: d.revision_id ?? d.id ?? '',
      name: d.display_name ?? d.name ?? 'Без названия',
      revision: d.version ?? d.revision ?? 1,
      state: d.status ?? d.state ?? 'draft',
    }));
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    draftsLoading.value = false;
  }
}

async function createDraft() {
  creating.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/schemas/drafts`, {
      method: 'POST',
      body: { name: newDraftName.value },
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    showCreateDraft.value = false;
    newDraftName.value = '';
    toast.add({ title: t('schema.create_draft'), color: 'success' });
    await fetchDrafts();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    creating.value = false;
  }
}

async function validateDraft(id: string) {
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/schemas/drafts/${id}/validate`, {
      method: 'POST',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({ title: t('schema.validate'), color: 'success' });
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

async function impactDraft(id: string) {
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/schemas/drafts/${id}/impact`, {
      method: 'POST',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({ title: t('schema.impact'), color: 'success' });
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

async function publishDraft(id: string) {
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/schemas/drafts/${id}/publish`, {
      method: 'POST',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({ title: t('schema.publish'), color: 'success' });
    await fetchDrafts();
    await fetchPublished();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

// Published
const published = ref<SchemaRevision[]>([]);
const publishedLoading = ref(false);

async function fetchPublished() {
  publishedLoading.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: any[] }>(`${baseUrl.replace(/\/$/, '')}/schemas/published`, {
      method: 'GET',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    published.value = (res.data ?? []).map((r: any) => ({
      revision_id: Number(r.revision_id) || 0,
      tenant_id: r.tenant_id ?? 'default',
      state: r.status ?? r.state ?? 'published',
      semantic_version: String(r.version ?? '1.0.0'),
      content_hash: r.content_hash ?? '',
      types: [],
      predicates: [],
    }));
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    publishedLoading.value = false;
  }
}

async function retireRevision(id: string) {
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/schemas/drafts/${id}/retire`, {
      method: 'POST',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({ title: t('schema.retire'), color: 'success' });
    await fetchPublished();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

onMounted(() => {
  fetchDrafts();
  fetchPublished();
});
</script>
