<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div class="flex items-center justify-between flex-wrap gap-2">
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
              <div class="flex items-center justify-between flex-wrap gap-2">
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2 flex-wrap">
                    <p class="font-medium">{{ draft.name }}</p>
                    <UBadge variant="subtle" size="sm">{{ draft.id }}</UBadge>
                    <UBadge
                      :color="draft.state === 'published' ? 'success' : draft.state === 'retired' ? 'neutral' : 'warning'"
                      variant="soft"
                      size="sm"
                    >
                      {{ draft.state }}
                    </UBadge>
                  </div>
                  <p class="text-sm text-[var(--ui-text-muted)] mt-1">
                    rev. {{ draft.revision }}
                  </p>
                </div>
                <div class="flex gap-2 flex-wrap">
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-check-circle"
                    :loading="validatingId === draft.id"
                    @click="validateDraft(draft.id)"
                  >
                    {{ t('schema.validate') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-git-compare"
                    :loading="impactingId === draft.id"
                    @click="impactDraft(draft.id)"
                  >
                    {{ t('schema.impact') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-pencil"
                    @click="editDraft(draft.id)"
                  >
                    {{ t('common.edit', 'Редактировать') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-eye"
                    @click="viewDraft(draft.id)"
                  >
                    {{ t('common.view', 'Просмотр') }}
                  </UButton>
                  <UButton
                    size="sm"
                    icon="i-lucide-upload"
                    :loading="publishingId === draft.id"
                    :disabled="draft.state !== 'draft'"
                    @click="publishDraft(draft.id)"
                  >
                    {{ t('schema.publish') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="ghost"
                    color="error"
                    icon="i-lucide-trash"
                    @click="deleteDraft(draft.id)"
                  />
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
              <div class="flex items-center justify-between flex-wrap gap-2">
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2 flex-wrap">
                    <p class="font-medium">{{ rev.display_name ?? `Revision ${rev.revision_id}` }}</p>
                    <UBadge variant="subtle" size="sm">{{ rev.revision_id }}</UBadge>
                    <UBadge variant="soft" size="sm" color="success">{{ rev.state }}</UBadge>
                  </div>
                  <p class="text-sm text-[var(--ui-text-muted)] mt-1">
                    {{ rev.semantic_version ?? `v${rev.version ?? 1}` }}
                  </p>
                </div>
                <div class="flex gap-2">
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-eye"
                    @click="viewDraft(String(rev.revision_id))"
                  >
                    {{ t('common.view', 'Просмотр') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    color="error"
                    icon="i-lucide-archive"
                    :loading="retiringId === String(rev.revision_id)"
                    @click="retireRevision(String(rev.revision_id))"
                  >
                    {{ t('schema.retire') }}
                  </UButton>
                </div>
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
          <UFormField :label="t('schema.draft_name', 'Название черновика')">
            <UInput v-model="newDraftName" class="w-full" placeholder="Например: Добавление поля ОКВЭД" />
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

    <!-- Validate result dialog -->
    <UModal v-model:open="showValidateResult">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold flex items-center gap-2">
            <UIcon name="i-lucide-check-circle" class="text-success" />
            {{ t('schema.validate') }}
          </h2>
          <div v-if="validateResult" class="space-y-2">
            <div class="flex items-center gap-2">
              <span class="text-[var(--ui-text-muted)]">Статус:</span>
              <UBadge :color="validateResult.valid ? 'success' : 'error'" variant="soft">
                {{ validateResult.valid ? 'Valid' : 'Invalid' }}
              </UBadge>
            </div>
            <div v-if="validateResult.warnings?.length" class="space-y-1">
              <p class="text-sm font-medium">Warnings:</p>
              <div v-for="(w, i) in validateResult.warnings" :key="i" class="text-sm bg-[var(--ui-bg-muted)] rounded p-2">
                {{ w }}
              </div>
            </div>
            <div v-if="validateResult.errors?.length" class="space-y-1">
              <p class="text-sm font-medium text-error">Errors:</p>
              <div v-for="(e, i) in validateResult.errors" :key="i" class="text-sm bg-error/10 rounded p-2">
                {{ e }}
              </div>
            </div>
            <div v-if="!validateResult.warnings?.length && !validateResult.errors?.length" class="text-sm text-[var(--ui-text-muted)]">
              Схема валидна, ошибок и предупреждений нет.
            </div>
          </div>
          <div class="flex justify-end">
            <UButton @click="showValidateResult = false">{{ t('common.close', 'Закрыть') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Impact result dialog -->
    <UModal v-model:open="showImpactResult">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold flex items-center gap-2">
            <UIcon name="i-lucide-git-compare" class="text-primary" />
            {{ t('schema.impact') }}
          </h2>
          <div v-if="impactResult" class="space-y-3">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div class="bg-[var(--ui-bg-muted)] rounded-lg p-3">
                <p class="text-2xl font-bold">{{ impactResult.affected_entities ?? 0 }}</p>
                <p class="text-sm text-[var(--ui-text-muted)]">Затронуто объектов</p>
              </div>
              <div class="bg-[var(--ui-bg-muted)] rounded-lg p-3">
                <p class="text-2xl font-bold">{{ impactResult.affected_statements ?? 0 }}</p>
                <p class="text-sm text-[var(--ui-text-muted)]">Затронуто утверждений</p>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-[var(--ui-text-muted)]">Breaking change:</span>
              <UBadge :color="impactResult.breaking ? 'error' : 'success'" variant="soft">
                {{ impactResult.breaking ? 'Да' : 'Нет' }}
              </UBadge>
            </div>
            <div v-if="impactResult.details" class="space-y-1 text-sm">
              <p>Новых полей: {{ impactResult.details.new_fields ?? 0 }}</p>
              <p>Удалённых полей: {{ impactResult.details.removed_fields ?? 0 }}</p>
              <p>Изменённых полей: {{ impactResult.details.changed_fields ?? 0 }}</p>
              <p>Миграция требуется: {{ impactResult.details.migration_required ? 'Да' : 'Нет' }}</p>
            </div>
          </div>
          <div class="flex justify-end">
            <UButton @click="showImpactResult = false">{{ t('common.close', 'Закрыть') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Draft edit dialog -->
    <UModal v-model:open="showDraftEdit">
      <template #content>
        <div class="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
          <h2 class="text-lg font-bold">{{ t('schema.edit_draft', 'Редактировать черновик') }}</h2>
          <UFormField :label="t('schema.draft_name', 'Название черновика')">
            <UInput v-model="editForm.display_name" class="w-full" />
          </UFormField>
          <UFormField :label="t('schema.types', 'Типы (JSON)')">
            <UTextarea v-model="editForm.types" :rows="4" class="w-full font-mono text-sm" />
          </UFormField>
          <UFormField :label="t('schema.predicates', 'Предикаты (JSON)')">
            <UTextarea v-model="editForm.predicates" :rows="4" class="w-full font-mono text-sm" />
          </UFormField>
          <UFormField :label="t('schema.changes', 'Изменения (JSON)')">
            <UTextarea v-model="editForm.changes" :rows="3" class="w-full font-mono text-sm" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showDraftEdit = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton :loading="savingDraft" @click="saveDraft">
              {{ t('common.save') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Draft detail dialog -->
    <UModal v-model:open="showDraftDetail">
      <template #content>
        <div class="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
          <h2 class="text-lg font-bold">{{ draftDetail?.display_name ?? 'Черновик' }}</h2>
          <div v-if="draftDetail" class="space-y-3">
            <div class="grid grid-cols-2 gap-3 text-sm">
              <div>
                <p class="text-[var(--ui-text-muted)]">ID:</p>
                <p class="font-medium">{{ draftDetail.revision_id }}</p>
              </div>
              <div>
                <p class="text-[var(--ui-text-muted)]">Статус:</p>
                <UBadge variant="soft" size="sm">{{ draftDetail.status }}</UBadge>
              </div>
              <div>
                <p class="text-[var(--ui-text-muted)]">Тип:</p>
                <p class="font-medium">{{ draftDetail.type_id }}</p>
              </div>
              <div>
                <p class="text-[var(--ui-text-muted)]">Версия:</p>
                <p class="font-medium">{{ draftDetail.version }}</p>
              </div>
              <div>
                <p class="text-[var(--ui-text-muted)]">Создан:</p>
                <p class="font-medium">{{ draftDetail.created_at }}</p>
              </div>
            </div>
            <div v-if="draftDetail.types?.length" class="space-y-1">
              <p class="font-medium">Типы ({{ draftDetail.types.length }}):</p>
              <div class="flex flex-wrap gap-1">
                <UBadge v-for="tp in draftDetail.types" :key="tp" variant="subtle" size="sm">{{ tp }}</UBadge>
              </div>
            </div>
            <div v-if="draftDetail.predicates?.length" class="space-y-1">
              <p class="font-medium">Предикаты ({{ draftDetail.predicates.length }}):</p>
              <div class="flex flex-wrap gap-1">
                <UBadge v-for="p in draftDetail.predicates" :key="p" variant="subtle" size="sm">{{ p }}</UBadge>
              </div>
            </div>
            <div v-if="draftDetail.changes?.length" class="space-y-1">
              <p class="font-medium">Изменения:</p>
              <div class="space-y-1">
                <div v-for="(c, i) in draftDetail.changes" :key="i" class="text-sm bg-[var(--ui-bg-muted)] rounded p-2">
                  {{ c }}
                </div>
              </div>
            </div>
          </div>
          <div v-else class="text-center py-4 text-[var(--ui-text-muted)]">
            {{ t('common.loading') }}
          </div>
          <div class="flex justify-end">
            <UButton @click="showDraftDetail = false">{{ t('common.close', 'Закрыть') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';

definePageMeta({
  middleware: ['auth', capability('admin.view')],
});

const { t } = useI18n();
const toast = useToast();
const api = useApi();

const activeTab = ref('drafts');
const tabs = [
  { value: 'drafts', label: 'Черновики' },
  { value: 'published', label: 'Опубликованные' },
];

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

// Action loading states
const validatingId = ref<string | null>(null);
const impactingId = ref<string | null>(null);
const publishingId = ref<string | null>(null);
const retiringId = ref<string | null>(null);

// Result dialogs
const showValidateResult = ref(false);
const validateResult = ref<any>(null);
const showImpactResult = ref(false);
const impactResult = ref<any>(null);
const showDraftDetail = ref(false);
const draftDetail = ref<any>(null);
const showDraftEdit = ref(false);
const savingDraft = ref(false);
const editDraftId = ref<string | null>(null);
const editForm = ref({
  display_name: '',
  types: '[]',
  predicates: '[]',
  changes: '[]',
});

async function fetchDrafts() {
  draftsLoading.value = true;
  try {
    const res = await api.schemas.listDrafts();
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
    await api.schemas.createDraft({ name: newDraftName.value });
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
  validatingId.value = id;
  try {
    const res = await api.schemas.validateDraft(id);
    validateResult.value = res.data;
    showValidateResult.value = true;
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    validatingId.value = null;
  }
}

async function impactDraft(id: string) {
  impactingId.value = id;
  try {
    const res = await api.schemas.impactDraft(id);
    impactResult.value = res.data;
    showImpactResult.value = true;
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    impactingId.value = null;
  }
}

async function publishDraft(id: string) {
  publishingId.value = id;
  try {
    await api.schemas.publishDraft(id);
    toast.add({ title: t('schema.publish'), color: 'success' });
    await fetchDrafts();
    await fetchPublished();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    publishingId.value = null;
  }
}

async function deleteDraft(id: string) {
  try {
    await api.schemas.deleteDraft(id);
    toast.add({ title: t('common.deleted', 'Удалено'), color: 'success' });
    await fetchDrafts();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

async function viewDraft(id: string) {
  showDraftDetail.value = true;
  draftDetail.value = null;
  try {
    const res = await api.schemas.getDraft(id);
    draftDetail.value = res.data;
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

async function editDraft(id: string) {
  editDraftId.value = id;
  showDraftEdit.value = true;
  try {
    const res = await api.schemas.getDraft(id);
    const d = res.data as any;
    editForm.value.display_name = d.display_name ?? '';
    editForm.value.types = JSON.stringify(d.types ?? [], null, 2);
    editForm.value.predicates = JSON.stringify(d.predicates ?? [], null, 2);
    editForm.value.changes = JSON.stringify(d.changes ?? [], null, 2);
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

async function saveDraft() {
  if (!editDraftId.value) return;
  savingDraft.value = true;
  try {
    const payload = {
      display_name: editForm.value.display_name,
      types: JSON.parse(editForm.value.types),
      predicates: JSON.parse(editForm.value.predicates),
      changes: JSON.parse(editForm.value.changes),
    };
    await api.schemas.updateDraft(editDraftId.value, payload);
    toast.add({ title: t('common.saved', 'Сохранено'), color: 'success' });
    showDraftEdit.value = false;
    await fetchDrafts();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    savingDraft.value = false;
  }
}

// Published
const published = ref<any[]>([]);
const publishedLoading = ref(false);

async function fetchPublished() {
  publishedLoading.value = true;
  try {
    const res = await api.schemas.listPublished();
    published.value = res.data ?? [];
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
  retiringId.value = id;
  try {
    await api.schemas.retireDraft(id);
    toast.add({ title: t('schema.retire'), color: 'success' });
    await fetchPublished();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    retiringId.value = null;
  }
}

onMounted(() => {
  fetchDrafts();
  fetchPublished();
});
</script>
