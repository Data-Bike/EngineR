<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('nav.audit') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('audit.subtitle') }}</p>
      </div>
      <div class="flex gap-2">
        <UButton
          icon="i-lucide-shield-check"
          variant="outline"
          @click="verifyChain"
          :loading="verifying"
        >
          {{ t('audit.verify_chain') }}
        </UButton>
        <UButton
          icon="i-lucide-bookmark"
          variant="outline"
          @click="createCheckpoint"
          :loading="checkpointing"
        >
          {{ t('audit.create_checkpoint') }}
        </UButton>
      </div>
    </div>

    <!-- Chain verification banner -->
    <UAlert
      v-if="chainVerification"
      :color="chainVerification.valid ? 'success' : 'error'"
      :icon="chainVerification.valid ? 'i-lucide-check-circle' : 'i-lucide-alert-octagon'"
      :title="chainVerification.valid ? t('audit.chain_valid') : t('audit.chain_broken')"
      :description="t('audit.chain_entries', { count: chainVerification.entry_count, last_seq: chainVerification.last_sequence })"
    />

    <!-- Filters -->
    <UCard>
      <div class="flex flex-wrap gap-3 items-end">
        <UFormField :label="t('audit.filter.kind')">
          <USelect
            v-model="filters.kind"
            :items="kindOptions"
            value-key="value"
           
            class="w-full sm:w-48"
          />
        </UFormField>
        <UFormField :label="t('audit.filter.actor')">
          <UInput v-model="filters.actor" class="w-full sm:w-48" />
        </UFormField>
        <UFormField :label="t('audit.filter.action')">
          <UInput v-model="filters.action" class="w-full sm:w-48" />
        </UFormField>
        <UFormField :label="t('audit.filter.from')">
          <UInput v-model="filters.from" type="datetime-local" class="w-full sm:w-48" />
        </UFormField>
        <UFormField :label="t('audit.filter.to')">
          <UInput v-model="filters.to" type="datetime-local" class="w-full sm:w-48" />
        </UFormField>
        <UButton @click="fetchAudit" icon="i-lucide-search">
          {{ t('common.search') }}
        </UButton>
        <UButton @click="resetFilters" variant="ghost" icon="i-lucide-rotate-ccw">
          {{ t('common.reset') }}
        </UButton>
      </div>
    </UCard>

    <!-- Audit entries table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('common.loading') }}
      </div>
      <div v-else-if="entries.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('entity.empty') }}
      </div>
      <div v-else class="overflow-x-auto"><table class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">#</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('audit.col.timestamp') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('audit.col.kind') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('audit.col.actor') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('audit.col.action') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('audit.col.resource') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('audit.col.effect') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('audit.col.hash') }}</th>
            <th class="py-2 px-3"></th>
          </tr>
        </thead>
        <tbody>
          <template v-for="entry in entries" :key="entry.audit_id">
            <tr
              class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)] cursor-pointer"
              @click="toggleExpand(entry.audit_id)"
            >
              <td class="py-2 px-3 text-[var(--ui-text-muted)]">{{ entry.sequence }}</td>
              <td class="py-2 px-3">{{ formatDate(entry.occurred_at) }}</td>
              <td class="py-2 px-3">
                <UBadge :color="kindColor(entry.kind)" size="sm">
                  {{ t(`audit.kind.${entry.kind}`) }}
                </UBadge>
              </td>
              <td class="py-2 px-3 font-mono text-xs">{{ entry.actor }}</td>
              <td class="py-2 px-3 font-mono text-xs">{{ entry.action }}</td>
              <td class="py-2 px-3 font-mono text-xs">
                {{ entry.resource_kind }}:{{ entry.resource_id?.slice(0, 8) ?? '—' }}
              </td>
              <td class="py-2 px-3">
                <UBadge v-if="entry.effect" :color="effectColor(entry.effect)" size="sm">
                  {{ entry.effect }}
                </UBadge>
                <span v-else class="text-[var(--ui-text-muted)]">—</span>
              </td>
              <td class="py-2 px-3 font-mono text-xs text-[var(--ui-text-muted)]">
                {{ entry.entry_hash.slice(0, 12) }}…
              </td>
              <td class="py-2 px-3">
                <UIcon
                  :name="expandedId === entry.audit_id ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
                  class="text-[var(--ui-text-muted)]"
                />
              </td>
            </tr>
            <!-- Expanded detail row -->
            <tr v-if="expandedId === entry.audit_id">
              <td colspan="9" class="py-4 px-6 bg-[var(--ui-bg-muted)]">
                <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.entry_id') }}</div>
                    <div class="font-mono text-xs">{{ entry.audit_id }}</div>
                  </div>
                  <div>
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.sequence') }}</div>
                    <div class="font-mono">{{ entry.sequence }}</div>
                  </div>
                  <div>
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.previous_hash') }}</div>
                    <div class="font-mono text-xs break-all">{{ entry.previous_hash }}</div>
                  </div>
                  <div>
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.entry_hash') }}</div>
                    <div class="font-mono text-xs break-all">{{ entry.entry_hash }}</div>
                  </div>
                  <div v-if="entry.effective_actor">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.effective_actor') }}</div>
                    <div class="font-mono text-xs">{{ entry.effective_actor }}</div>
                  </div>
                  <div v-if="entry.request_id">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.request_id') }}</div>
                    <div class="font-mono text-xs">{{ entry.request_id }}</div>
                  </div>
                  <div v-if="entry.correlation_id">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.correlation_id') }}</div>
                    <div class="font-mono text-xs">{{ entry.correlation_id }}</div>
                  </div>
                  <div v-if="entry.policy_revision !== null">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.policy_revision') }}</div>
                    <div class="font-mono">{{ entry.policy_revision }}</div>
                  </div>
                  <div v-if="entry.query_fingerprint">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.query_fingerprint') }}</div>
                    <div class="font-mono text-xs">{{ entry.query_fingerprint }}</div>
                  </div>
                  <div v-if="entry.row_count !== null">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.row_count') }}</div>
                    <div>{{ entry.row_count }}</div>
                  </div>
                  <div v-if="entry.bytes !== null">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.bytes') }}</div>
                    <div>{{ formatBytes(entry.bytes) }}</div>
                  </div>
                  <div v-if="entry.client">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.client') }}</div>
                    <div class="font-mono text-xs">{{ entry.client }}</div>
                  </div>
                  <div v-if="entry.service">
                    <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.service') }}</div>
                    <div class="font-mono text-xs">{{ entry.service }}</div>
                  </div>
                </div>
                <!-- Reason IDs -->
                <div v-if="entry.reason_ids.length > 0" class="mt-4">
                  <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.reason_ids') }}</div>
                  <div class="flex flex-wrap gap-1">
                    <UBadge v-for="rid in entry.reason_ids" :key="rid" variant="subtle" size="sm">
                      {{ rid.slice(0, 8) }}
                    </UBadge>
                  </div>
                </div>
                <!-- Metadata -->
                <div v-if="entry.metadata && Object.keys(entry.metadata).length > 0" class="mt-4">
                  <div class="text-[var(--ui-text-muted)] mb-1">{{ t('audit.detail.metadata') }}</div>
                  <pre class="bg-[var(--ui-bg)] rounded p-3 text-xs overflow-x-auto">{{ JSON.stringify(entry.metadata, null, 2) }}</pre>
                </div>
              </td>
            </tr>
          </template>
        </tbody>
      </table></div>

      <!-- Pagination -->
      <div v-if="entries.length > 0" class="flex justify-between items-center mt-4 pt-4 border-t border-[var(--ui-border)]">
        <span class="text-sm text-[var(--ui-text-muted)]">
          {{ t('audit.showing', { count: entries.length }) }}
        </span>
        <UButton
          v-if="entries.length >= 100"
          size="sm"
          variant="outline"
          @click="loadMore"
          :loading="loadingMore"
        >
          {{ t('common.load_more') }}
        </UButton>
      </div>
    </UCard>

    <!-- Latest checkpoint + Retention policy -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- Latest checkpoint -->
      <UCard v-if="latestCheckpoint">
        <template #header>
          <h3 class="font-semibold">{{ t('audit.latest_checkpoint') }}</h3>
        </template>
        <div class="space-y-3 text-sm">
          <div>
            <span class="text-[var(--ui-text-muted)]">{{ t('audit.checkpoint.sequence') }}: </span>
            <span class="font-mono">{{ latestCheckpoint.sequence }}</span>
          </div>
          <div>
            <span class="text-[var(--ui-text-muted)]">{{ t('audit.checkpoint.hash') }}: </span>
            <span class="font-mono text-xs break-all">{{ latestCheckpoint.entry_hash }}</span>
          </div>
          <div>
            <span class="text-[var(--ui-text-muted)]">{{ t('audit.checkpoint.created') }}: </span>
            <span>{{ formatDate(latestCheckpoint.created_at) }}</span>
          </div>
        </div>
      </UCard>

      <!-- Retention policy -->
      <UCard>
        <template #header>
          <div class="flex items-center justify-between">
            <h3 class="font-semibold">{{ t('audit.retention.title') }}</h3>
            <UButton
              size="xs"
              variant="outline"
              icon="i-lucide-edit"
              @click="showRetentionDialog = true"
            >
              {{ t('common.edit') }}
            </UButton>
          </div>
        </template>
        <div v-if="retentionPolicy" class="space-y-3 text-sm">
          <div>
            <span class="text-[var(--ui-text-muted)]">{{ t('audit.retention.days') }}: </span>
            <span class="font-medium">{{ retentionPolicy.retention_days }}</span>
          </div>
          <div>
            <span class="text-[var(--ui-text-muted)]">{{ t('audit.retention.legal_hold') }}: </span>
            <UBadge :color="retentionPolicy.legal_hold ? 'error' : 'neutral'" size="sm">
              {{ retentionPolicy.legal_hold ? t('common.yes') : t('common.no') }}
            </UBadge>
          </div>
          <div>
            <span class="text-[var(--ui-text-muted)]">{{ t('audit.retention.archive') }}: </span>
            <UBadge :color="retentionPolicy.archive_before_delete ? 'success' : 'neutral'" size="sm">
              {{ retentionPolicy.archive_before_delete ? t('common.enabled') : t('common.disabled') }}
            </UBadge>
          </div>
          <div v-if="retentionPolicy.archive_url">
            <span class="text-[var(--ui-text-muted)]">{{ t('audit.retention.archive_url') }}: </span>
            <span class="font-mono text-xs">{{ retentionPolicy.archive_url }}</span>
          </div>
        </div>
        <div v-else class="text-[var(--ui-text-muted)] text-sm">
          {{ t('common.loading') }}
        </div>
      </UCard>
    </div>

    <!-- Edit retention dialog -->
    <UModal v-model:open="showRetentionDialog">
      <template #content>
        <UCard>
          <template #header>
            <h3 class="text-lg font-semibold">{{ t('audit.retention.edit_title') }}</h3>
          </template>
          <div class="space-y-4">
            <UFormField :label="t('audit.retention.days')">
              <UInput v-model.number="retentionForm.retention_days" type="number" class="w-full" />
            </UFormField>
            <UFormField :label="t('audit.retention.legal_hold')">
              <USwitch v-model="retentionForm.legal_hold" />
            </UFormField>
            <UFormField :label="t('audit.retention.archive')">
              <USwitch v-model="retentionForm.archive_before_delete" />
            </UFormField>
            <UFormField :label="t('audit.retention.archive_url')">
              <UInput v-model="retentionForm.archive_url" placeholder="s3://archive-bucket" class="w-full" />
            </UFormField>
          </div>
          <template #footer>
            <div class="flex justify-end gap-2">
              <UButton variant="outline" @click="showRetentionDialog = false">{{ t('common.cancel') }}</UButton>
              <UButton @click="saveRetention" :loading="savingRetention">{{ t('common.save') }}</UButton>
            </div>
          </template>
        </UCard>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useApi } from '~/composables/useApi';
import type { AuditKind, AuditRecord, ChainCheckpoint, ChainVerificationResult, RetentionPolicy } from '@enginer/api-client';

definePageMeta({
  middleware: ['auth', capability('audit.read')],
});

const { t } = useI18n();
const api = useApi();
const toast = useToast();

const entries = ref<AuditRecord[]>([]);
const loading = ref(false);
const loadingMore = ref(false);
const verifying = ref(false);
const checkpointing = ref(false);
const savingRetention = ref(false);
const expandedId = ref<number | null>(null);
const latestCheckpoint = ref<ChainCheckpoint | null>(null);
const chainVerification = ref<ChainVerificationResult | null>(null);
const retentionPolicy = ref<RetentionPolicy | null>(null);
const showRetentionDialog = ref(false);

const filters = reactive({
  kind: 'all' as string,
  actor: '' as string,
  action: '' as string,
  from: '' as string,
  to: '' as string,
});

const retentionForm = reactive({
  retention_days: 365,
  legal_hold: false,
  archive_before_delete: true,
  archive_url: '' as string,
});

const kindOptions = [
  { label: t('audit.filter.all_kinds'), value: 'all' },
  { label: t('audit.kind.authentication'), value: 'authentication' },
  { label: t('audit.kind.decision'), value: 'decision' },
  { label: t('audit.kind.denied_request'), value: 'denied_request' },
  { label: t('audit.kind.policy_change'), value: 'policy_change' },
  { label: t('audit.kind.export'), value: 'export' },
  { label: t('audit.kind.blob_read'), value: 'blob_read' },
  { label: t('audit.kind.bulk_action'), value: 'bulk_action' },
  { label: t('audit.kind.break_glass'), value: 'break_glass' },
  { label: t('audit.kind.impersonation'), value: 'impersonation' },
];

type BadgeColor = 'info' | 'neutral' | 'warning' | 'error' | 'success';

function kindColor(kind: string): BadgeColor {
  switch (kind) {
    case 'authentication': return 'info';
    case 'decision': return 'neutral';
    case 'denied_request': return 'warning';
    case 'policy_change': return 'error';
    case 'export': return 'warning';
    case 'blob_read': return 'info';
    case 'bulk_action': return 'warning';
    case 'break_glass': return 'error';
    case 'impersonation': return 'error';
    default: return 'neutral';
  }
}

function effectColor(effect: string): BadgeColor {
  switch (effect) {
    case 'allow': return 'success';
    case 'deny': return 'error';
    case 'escalate': return 'warning';
    case 'defer': return 'neutral';
    case 'allow_with_obligations': return 'warning';
    default: return 'neutral';
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function toggleExpand(id: number) {
  expandedId.value = expandedId.value === id ? null : id;
}

function resetFilters() {
  filters.kind = 'all';
  filters.actor = '';
  filters.action = '';
  filters.from = '';
  filters.to = '';
  fetchAudit();
}

async function fetchAudit() {
  loading.value = true;
  try {
    const res = await api.audit.query({
      limit: 100,
      kind: filters.kind && filters.kind !== 'all' ? filters.kind as AuditKind : undefined,
      actor: filters.actor || undefined,
      action: filters.action || undefined,
      from: filters.from ? new Date(filters.from).toISOString() : undefined,
      to: filters.to ? new Date(filters.to).toISOString() : undefined,
    });
    entries.value = res.data;
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

async function loadMore() {
  const lastSeq = entries.value[entries.value.length - 1]?.sequence;
  if (lastSeq === undefined) return;
  loadingMore.value = true;
  try {
    const res = await api.audit.query({
      limit: 100,
      cursor: lastSeq,
      kind: filters.kind && filters.kind !== 'all' ? filters.kind as AuditKind : undefined,
      actor: filters.actor || undefined,
      action: filters.action || undefined,
      from: filters.from ? new Date(filters.from).toISOString() : undefined,
      to: filters.to ? new Date(filters.to).toISOString() : undefined,
    });
    entries.value.push(...res.data);
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    loadingMore.value = false;
  }
}

async function verifyChain() {
  verifying.value = true;
  try {
    const res = await api.audit.verifyChain();
    chainVerification.value = res.data;
    toast.add({
      title: res.data.valid ? t('audit.chain_valid') : t('audit.chain_broken'),
      color: res.data.valid ? 'success' : 'error',
    });
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    verifying.value = false;
  }
}

async function createCheckpoint() {
  checkpointing.value = true;
  try {
    await api.audit.createCheckpoint();
    await fetchCheckpoint();
    toast.add({
      title: t('audit.checkpoint_created'),
      color: 'success',
    });
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    checkpointing.value = false;
  }
}

async function fetchCheckpoint() {
  try {
    const res = await api.audit.latestCheckpoint();
    latestCheckpoint.value = res.data;
  } catch {
    // Non-fatal — checkpoint may not exist yet
  }
}

async function fetchRetention() {
  try {
    const res = await api.audit.getRetentionPolicy();
    retentionPolicy.value = res.data;
    retentionForm.retention_days = res.data.retention_days;
    retentionForm.legal_hold = res.data.legal_hold;
    retentionForm.archive_before_delete = res.data.archive_before_delete;
    retentionForm.archive_url = res.data.archive_url ?? '';
  } catch {
    // Non-fatal
  }
}

async function saveRetention() {
  savingRetention.value = true;
  try {
    const res = await api.audit.setRetentionPolicy({
      tenant_id: retentionPolicy.value?.tenant_id ?? '',
      retention_days: retentionForm.retention_days,
      legal_hold: retentionForm.legal_hold,
      archive_before_delete: retentionForm.archive_before_delete,
      archive_url: retentionForm.archive_url || null,
    });
    retentionPolicy.value = res.data;
    showRetentionDialog.value = false;
    toast.add({
      title: t('audit.retention.saved'),
      color: 'success',
    });
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    savingRetention.value = false;
  }
}

onMounted(() => {
  fetchAudit();
  fetchCheckpoint();
  fetchRetention();
});
</script>
