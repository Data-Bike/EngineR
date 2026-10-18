<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('nav.reports') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('entity.reports.subtitle') }}</p>
      </div>
      <UButton icon="i-lucide-file-plus" @click="showGenerate = true">
        {{ t('entity.reports.generate') }}
      </UButton>
    </div>

    <!-- Summary cards -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <StatCard
        :label="t('entity.reports.ready')"
        :value="readyCount"
        icon="i-lucide-file-check"
      />
      <StatCard
        :label="t('entity.reports.generating')"
        :value="generatingCount"
        icon="i-lucide-loader"
      />
      <StatCard
        :label="t('entity.reports.expired_label')"
        :value="expiredCount"
        icon="i-lucide-file-x"
      />
    </div>

    <!-- Reports table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('common.loading') }}
      </div>
      <div v-else-if="reports.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('entity.empty') }}
      </div>
      <div v-else class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-[var(--ui-border)] text-left">
              <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">Title</th>
              <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">Entity</th>
              <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">Format</th>
              <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">Status</th>
              <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">Generated</th>
              <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">Size</th>
              <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="report in reports" :key="report.report_id" class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)]">
              <td class="py-2 px-3 font-medium">{{ report.title ?? report.report_id }}</td>
              <td class="py-2 px-3">
                <span v-if="report.entity_public_id">{{ report.entity_public_id }}</span>
                <span v-else class="text-[var(--ui-text-muted)]">—</span>
              </td>
              <td class="py-2 px-3"><UBadge variant="subtle" size="sm">{{ String(report.format ?? '').toUpperCase() }}</UBadge></td>
              <td class="py-2 px-3"><UBadge variant="subtle" size="sm">{{ t(`entity.reports.status.${report.status}`, report.status) }}</UBadge></td>
              <td class="py-2 px-3">{{ report.generated_at ?? '—' }}</td>
              <td class="py-2 px-3">{{ report.size_bytes ? formatFileSize(report.size_bytes as number) : '—' }}</td>
              <td class="py-2 px-3">
                <UButton
                  v-if="report.status === 'ready' && report.download_url && !isExpired(report)"
                  size="xs"
                  variant="outline"
                  icon="i-lucide-download"
                  @click="downloadReport(report)"
                >
                  {{ t('entity.reports.download') }}
                </UButton>
                <span v-else-if="isExpired(report)" class="text-xs text-[var(--ui-text-muted)]">
                  {{ t('entity.reports.expired_label') }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </UCard>

    <!-- Generate dialog -->
    <UModal v-model:open="showGenerate">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.reports.generate') }}</h2>
          <UFormField :label="t('entity.reports.entity')">
            <EntityAutocomplete
              v-model="newEntityId"
              :placeholder="t('entity.search_placeholder')"
            />
          </UFormField>
          <UFormField :label="t('entity.reports.template')">
            <USelect v-model="newTemplate" :items="templateOptions" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.reports.format')">
            <USelect v-model="newFormat" :items="formatOptions" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showGenerate = false">{{ t('common.cancel') }}</UButton>
            <UButton
              @click="generateReport"
              :loading="generating"
              :disabled="!newEntityId"
            >{{ t('entity.reports.generate') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ReportInstance, ReportFormat } from '@enginer/api-client';

definePageMeta({
  middleware: ['auth', capability('report.view')],
});

const { t } = useI18n();
const toast = useToast();
const api = useApi();

const reports = ref<ReportInstance[]>([]);
const loading = ref(false);
const showGenerate = ref(false);
const generating = ref(false);
const newEntityId = ref('');
const newTemplate = ref('standard');
const newFormat = ref<ReportFormat>('pdf');

const readyCount = computed(() => reports.value.filter((r) => r.status === 'ready').length);
const generatingCount = computed(() => reports.value.filter((r) => r.status === 'generating').length);
const expiredCount = computed(() => reports.value.filter((r) => isExpired(r)).length);

const templateOptions = computed(() => [
  { label: t('entity.reports.template.standard'), value: 'standard' },
  { label: t('entity.reports.template.due_diligence'), value: 'due_diligence' },
  { label: t('entity.reports.template.compliance'), value: 'compliance' },
]);

const formatOptions = computed(() => [
  { label: 'PDF', value: 'pdf' },
  { label: 'Excel', value: 'xlsx' },
  { label: 'Word', value: 'docx' },
  { label: 'JSON', value: 'json' },
  { label: 'CSV', value: 'csv' },
]);

function isExpired(report: ReportInstance): boolean {
  if (!report.expires_at) return false;
  return new Date(report.expires_at) < new Date();
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function downloadReport(report: ReportInstance) {
  if (report.download_url) {
    window.open(report.download_url, '_blank');
    toast.add({ title: t('entity.reports.download'), color: 'success' });
  }
}

async function fetchReports() {
  loading.value = true;
  try {
    const res = await api.reports.list();
    reports.value = res.data as ReportInstance[];
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    loading.value = false;
  }
}

async function generateReport() {
  generating.value = true;
  try {
    await api.reports.create({
      entity_id: newEntityId.value,
      template: newTemplate.value,
      format: newFormat.value,
    });
    showGenerate.value = false;
    newEntityId.value = '';
    toast.add({ title: t('entity.reports.generate'), color: 'success' });
    await fetchReports();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    generating.value = false;
  }
}

onMounted(fetchReports);
</script>
