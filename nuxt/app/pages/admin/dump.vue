<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <div>
        <h1 class="text-2xl font-bold">{{ t('dump.title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('dump.desc') }}</p>
      </div>
      <div class="flex gap-2">
        <UButton icon="i-lucide-upload" variant="outline" :loading="uploading"
                 data-testid="upload-dump" @click="fileInput?.click()">
          {{ t('dump.upload') }}
        </UButton>
        <UButton icon="i-lucide-database-backup" :loading="starting" data-testid="run-dump"
                 @click="runDump">
          {{ t('dump.run') }}
        </UButton>
      </div>
    </div>
    <input ref="fileInput" type="file" accept=".xz,.ndjson" class="hidden" @change="onUpload" />

    <UAlert color="warning" variant="subtle" icon="i-lucide-shield-alert"
            :title="t('dump.warning_title')" :description="t('dump.warning_desc')" />

    <div v-if="loading" class="text-center py-8 text-[var(--ui-text-muted)]">{{ t('common.loading') }}</div>
    <div v-else class="border border-[var(--ui-border)] rounded-md overflow-x-auto">
      <table class="w-full text-sm" data-testid="dumps-table">
        <thead class="bg-[var(--ui-bg-muted)]">
          <tr>
            <th class="text-start p-3 font-medium">{{ t('dump.col_id') }}</th>
            <th class="text-start p-3 font-medium">{{ t('dump.col_status') }}</th>
            <th class="text-start p-3 font-medium">{{ t('dump.col_finished') }}</th>
            <th class="text-start p-3 font-medium">{{ t('dump.col_files') }}</th>
            <th class="p-3"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="d in dumps" :key="d.dump_id"
              class="border-t border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)]">
            <td class="p-3 font-medium font-mono">{{ d.dump_id }}</td>
            <td class="p-3">
              <UBadge :color="statusColor(d.status)" variant="subtle" size="sm">{{ d.status }}</UBadge>
            </td>
            <td class="p-3 text-[var(--ui-text-muted)]">{{ fmtDate(d.finished_at) }}</td>
            <td class="p-3">
              <div class="flex flex-wrap gap-2">
                <a v-for="f in d.files" :key="f.name" :href="f.download_url" download
                   class="text-[var(--ui-primary)] hover:underline font-mono text-xs">
                  {{ f.name }} ({{ fmtSize(f.size) }})
                </a>
                <span v-if="!d.files.length" class="text-[var(--ui-text-muted)]">—</span>
              </div>
            </td>
            <td class="p-3 whitespace-nowrap">
              <UButton v-if="isFinished(d.status) && hasPayload(d)" size="xs" variant="ghost"
                       color="warning" icon="i-lucide-history" :title="t('dump.restore')"
                       :data-testid="`restore-${d.dump_id}`" @click="askRestore(d.dump_id)" />
              <UButton v-if="isFinished(d.status)" size="xs" variant="ghost" color="error"
                       icon="i-lucide-trash-2" :title="t('dump.delete')"
                       :data-testid="`delete-${d.dump_id}`" @click="removeDump(d.dump_id)" />
            </td>
          </tr>
          <tr v-if="!dumps.length">
            <td colspan="5" class="p-6 text-center text-[var(--ui-text-muted)]">{{ t('dump.empty') }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <UAlert v-if="error" color="error" variant="subtle" :title="error" icon="i-lucide-alert-circle" />

    <UModal v-model:open="restoreDialog" :title="t('dump.restore_title')">
      <template #body>
        <UAlert color="error" variant="subtle" icon="i-lucide-alert-triangle"
                :description="t('dump.restore_desc')" />
        <p class="mt-3 text-sm font-mono">{{ restoreTarget }}</p>
      </template>
      <template #footer>
        <div class="flex justify-end gap-2">
          <UButton variant="ghost" @click="restoreDialog = false">{{ t('common.cancel') }}</UButton>
          <UButton color="error" icon="i-lucide-history" :loading="restoring"
                   data-testid="confirm-restore" @click="doRestore">
            {{ t('dump.restore') }}
          </UButton>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import type { DumpInfo } from '@enginer/api-client';
import { capability } from '~/middleware/capability';

definePageMeta({
  middleware: ['auth', capability(['security.manage'])],
});

const { t } = useI18n();
const api = useApi();
const toast = useToast();

const dumps = ref<DumpInfo[]>([]);
const loading = ref(true);
const starting = ref(false);
const uploading = ref(false);
const error = ref('');
const fileInput = ref<HTMLInputElement | null>(null);
const restoreDialog = ref(false);
const restoreTarget = ref('');
const restoring = ref(false);
let pollTimer: ReturnType<typeof setInterval> | null = null;

const BUSY = new Set(['running', 'restoring']);

function isFinished(status: string): boolean {
  return !BUSY.has(status);
}

function hasPayload(d: DumpInfo): boolean {
  return d.files.some((f) => f.name.endsWith('.ndjson') || f.name.endsWith('.ndjson.xz'));
}

async function load() {
  try {
    const res = await api.dumps.list();
    dumps.value = res.data ?? [];
    error.value = '';
    // Auto-refresh while any dump is running or restoring.
    const busy = dumps.value.some((d) => BUSY.has(d.status));
    if (busy && !pollTimer) {
      pollTimer = setInterval(load, 3000);
    } else if (!busy && pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function runDump() {
  starting.value = true;
  error.value = '';
  try {
    await api.dumps.run();
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    starting.value = false;
  }
}

async function onUpload(ev: Event) {
  const input = ev.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file) return;
  uploading.value = true;
  error.value = '';
  try {
    await api.dumps.upload(file);
    toast.add({ title: t('dump.uploaded'), color: 'success' });
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    uploading.value = false;
  }
}

function askRestore(id: string) {
  restoreTarget.value = id;
  restoreDialog.value = true;
}

async function doRestore() {
  restoring.value = true;
  error.value = '';
  try {
    await api.dumps.restore(restoreTarget.value);
    restoreDialog.value = false;
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    restoring.value = false;
  }
}

async function removeDump(id: string) {
  try {
    await api.dumps.remove(id);
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function statusColor(status: string): 'success' | 'error' | 'info' | 'neutral' {
  if (status === 'done' || status === 'restored') return 'success';
  if (status === 'failed') return 'error';
  if (BUSY.has(status)) return 'info';
  return 'neutral';
}

function fmtDate(v?: string | null): string {
  return v ? new Date(v).toLocaleString() : '—';
}

function fmtSize(bytes: number): string {
  if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`;
  if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

onMounted(load);
onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
});
</script>
