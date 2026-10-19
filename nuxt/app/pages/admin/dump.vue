<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('dump.title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('dump.desc') }}</p>
      </div>
      <UButton icon="i-lucide-database-backup" :loading="starting" data-testid="run-dump"
               @click="runDump">
        {{ t('dump.run') }}
      </UButton>
    </div>

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
            <td class="p-3">
              <UButton v-if="d.status !== 'running'" size="xs" variant="ghost" color="error"
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

const dumps = ref<DumpInfo[]>([]);
const loading = ref(true);
const starting = ref(false);
const error = ref('');
let pollTimer: ReturnType<typeof setInterval> | null = null;

async function load() {
  try {
    const res = await api.dumps.list();
    dumps.value = res.data ?? [];
    error.value = '';
    // Auto-refresh while any dump is running.
    const running = dumps.value.some((d) => d.status === 'running');
    if (running && !pollTimer) {
      pollTimer = setInterval(load, 3000);
    } else if (!running && pollTimer) {
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

async function removeDump(id: string) {
  try {
    await api.dumps.remove(id);
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function statusColor(status: string): 'success' | 'error' | 'info' | 'neutral' {
  if (status === 'done') return 'success';
  if (status === 'failed') return 'error';
  if (status === 'running') return 'info';
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
