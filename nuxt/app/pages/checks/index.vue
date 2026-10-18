<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('nav.checks') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('checks.subtitle') }}</p>
      </div>
      <UButton
        icon="i-lucide-plus"
        @click="showNewCheckDialog = true"
      >
        {{ t('checks.new') }}
      </UButton>
    </div>

    <!-- Summary cards -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <StatCard
        :label="t('checks.pending')"
        :value="pendingCount"
        icon="i-lucide-clock"
      />
      <StatCard
        :label="t('checks.running')"
        :value="runningCount"
        icon="i-lucide-loader"
      />
      <StatCard
        :label="t('checks.completed')"
        :value="completedCount"
        icon="i-lucide-check-circle"
      />
      <StatCard
        :label="t('checks.failed')"
        :value="failedCount"
        icon="i-lucide-x-circle"
      />
    </div>

    <!-- Filters -->
    <UCard>
      <div class="flex flex-wrap gap-3 items-end">
        <UFormField :label="t('checks.filter.kind')">
          <USelect
            v-model="filters.kind"
            :items="kindOptions"
            value-key="value"
           
            class="w-full sm:w-48"
          />
        </UFormField>
        <UFormField :label="t('checks.filter.status')">
          <USelect
            v-model="filters.status"
            :items="statusOptions"
            value-key="value"
           
            class="w-full sm:w-48"
          />
        </UFormField>
        <UFormField :label="t('checks.filter.entity')">
          <UInput
            v-model="filters.entityId"
            :placeholder="t('checks.filter.entity_placeholder')"
            class="w-64"
          />
        </UFormField>
        <UButton @click="fetchChecks" icon="i-lucide-search">
          {{ t('common.search') }}
        </UButton>
      </div>
    </UCard>

    <!-- Checks table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('common.loading') }}
      </div>
      <div v-else-if="filteredChecks.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('entity.empty') }}
      </div>
      <div v-else class="overflow-x-auto"><table class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('checks.col.id') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('checks.col.entity') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('checks.col.kind') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('checks.col.status') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('checks.col.created') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('checks.col.score') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('checks.col.level') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="chk in filteredChecks"
            :key="chk.check_id"
            class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)]"
          >
            <td class="py-2 px-3 font-mono text-xs">{{ chk.check_id.slice(0, 8) }}</td>
            <td class="py-2 px-3">
              <NuxtLink :to="`/entities/${chk.target_entity_id}`" class="hover:underline">
                {{ chk.target_entity_id.slice(0, 8) }}
              </NuxtLink>
            </td>
            <td class="py-2 px-3">
              <UBadge variant="subtle" size="sm">{{ t(`checks.kind.${chk.check_kind}`) }}</UBadge>
            </td>
            <td class="py-2 px-3">
              <UBadge :color="statusColor(chk.status)" size="sm">
                {{ t(`checks.status.${chk.status}`) }}
              </UBadge>
            </td>
            <td class="py-2 px-3 text-[var(--ui-text-muted)]">{{ formatDate(chk.created_at) }}</td>
            <td class="py-2 px-3">{{ chk.final_score?.toFixed(1) ?? '—' }}</td>
            <td class="py-2 px-3">
              <RiskLevelBadge v-if="chk.risk_level" :level="chk.risk_level" />
              <span v-else class="text-[var(--ui-text-muted)]">—</span>
            </td>
            <td class="py-2 px-3">
              <UButton size="xs" variant="outline" :to="`/checks/${chk.check_id}`">
                {{ t('common.view') }}
              </UButton>
            </td>
          </tr>
        </tbody>
      </table></div>
    </UCard>

    <!-- New check dialog -->
    <UModal v-model:open="showNewCheckDialog">
      <template #content>
        <UCard>
          <template #header>
            <h3 class="text-lg font-semibold">{{ t('checks.new') }}</h3>
          </template>
          <div class="space-y-4">
            <UFormField :label="t('checks.form.entity')">
              <EntityAutocomplete
                v-model="newCheck.target_entity_id"
                :placeholder="t('entity.search_placeholder')"
              />
            </UFormField>
            <UFormField :label="t('checks.form.kind')">
              <USelect
                v-model="newCheck.check_kind"
                :items="kindOptions"
                value-key="value"
               
                class="w-full"
              />
            </UFormField>
            <UFormField :label="t('checks.form.connector')">
              <UInput v-model="newCheck.connector_instance_id" :placeholder="t('checks.form.connector_placeholder')" class="w-full" />
            </UFormField>
          </div>
          <template #footer>
            <div class="flex justify-end gap-2">
              <UButton variant="outline" @click="showNewCheckDialog = false">{{ t('common.cancel') }}</UButton>
              <UButton
                @click="submitNewCheck"
                :loading="submitting"
                :disabled="!newCheck.target_entity_id"
              >{{ t('common.submit') }}</UButton>
            </div>
          </template>
        </UCard>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ref, computed, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useApi } from '~/composables/useApi';
import type { CheckResponse, RiskLevel } from '@enginer/api-client';

definePageMeta({
  middleware: ['auth', capability('check.view')],
});

const { t } = useI18n();
const api = useApi();
const toast = useToast();

interface CheckRow extends Omit<CheckResponse, 'created_at' | 'risk_level'> {
  target_entity_id: string;
  check_kind: string;
  created_at: string;
  final_score?: number;
  risk_level?: RiskLevel;
}

const checks = ref<CheckRow[]>([]);
const loading = ref(false);
const showNewCheckDialog = ref(false);
const submitting = ref(false);

const filters = reactive({
  kind: 'all' as string,
  status: 'all' as string,
  entityId: '' as string,
});

const newCheck = reactive({
  target_entity_id: '',
  check_kind: 'kyc' as string,
  connector_instance_id: '',
});

const kindOptions = [
  { label: t('checks.filter.all_kinds'), value: 'all' },
  { label: t('checks.kind.kyc'), value: 'kyc' },
  { label: t('checks.kind.sanctions'), value: 'sanctions' },
  { label: t('checks.kind.financial_health'), value: 'financial_health' },
  { label: t('checks.kind.legal_status'), value: 'legal_status' },
  { label: t('checks.kind.ownership'), value: 'ownership' },
];

const statusOptions = [
  { label: t('checks.filter.all_statuses'), value: 'all' },
  { label: t('checks.status.pending'), value: 'pending' },
  { label: t('checks.status.running'), value: 'running' },
  { label: t('checks.status.completed'), value: 'completed' },
  { label: t('checks.status.failed'), value: 'failed' },
  { label: t('checks.status.cancelled'), value: 'cancelled' },
];

const filteredChecks = computed(() => {
  return checks.value.filter((c) => {
    if (filters.kind !== 'all' && c.check_kind !== filters.kind) return false;
    if (filters.status !== 'all' && c.status !== filters.status) return false;
    if (filters.entityId && !c.target_entity_id.includes(filters.entityId)) return false;
    return true;
  });
});

const pendingCount = computed(() => checks.value.filter((c) => c.status === 'pending').length);
const runningCount = computed(() => checks.value.filter((c) => c.status === 'running').length);
const completedCount = computed(() => checks.value.filter((c) => c.status === 'completed').length);
const failedCount = computed(() => checks.value.filter((c) => c.status === 'failed').length);

type BadgeColor = 'success' | 'info' | 'error' | 'neutral';

function statusColor(status: string): BadgeColor {
  switch (status) {
    case 'completed': return 'success';
    case 'running': return 'info';
    case 'pending': return 'neutral';
    case 'failed': return 'error';
    case 'cancelled': return 'neutral';
    default: return 'neutral';
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

async function fetchChecks() {
  loading.value = true;
  try {
    const res = await api.checks.listChecks();
    checks.value = (res.data || []).map((c: any) => ({
      ...c,
      check_id: c.check_id,
      target_entity_id: c.target_entity_id,
      check_kind: c.kind ?? c.check_kind,
      final_score: c.final_score ?? c.score ?? null,
      risk_level: c.risk_level,
      status: c.status,
      created_at: c.started_at ?? c.created_at,
    }));
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

async function submitNewCheck() {
  submitting.value = true;
  try {
    await api.checks.requestCheck({
      target_entity_id: newCheck.target_entity_id,
      check_kind: newCheck.check_kind,
      connector_instance_id: newCheck.connector_instance_id || undefined,
      parameters: {},
    });
    showNewCheckDialog.value = false;
    toast.add({ title: t('checks.new_success'), color: 'success' });
    await fetchChecks();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    submitting.value = false;
  }
}

onMounted(fetchChecks);
</script>
