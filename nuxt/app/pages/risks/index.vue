<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('nav.risks') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('risks.subtitle') }}</p>
      </div>
    </div>

    <!-- Summary cards -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <StatCard icon="i-lucide-shield-check" :value="stats.minimal" :label="t('risks.level.minimal')" color="success" />
      <StatCard icon="i-lucide-info" :value="stats.low" :label="t('risks.level.low')" color="info" />
      <StatCard icon="i-lucide-alert-triangle" :value="stats.high" :label="t('risks.level.high')" color="warning" />
      <StatCard icon="i-lucide-octagon-alert" :value="stats.critical" :label="t('risks.level.critical')" color="error" />
    </div>

    <!-- Filters -->
    <UCard>
      <div class="flex flex-wrap gap-3 items-end">
        <UFormField :label="t('risks.filter.status')">
          <USelect v-model="filters.status" :items="statusOptions" value-key="value" class="w-full sm:w-48" />
        </UFormField>
        <UButton @click="fetchAssessments" icon="i-lucide-search">{{ t('common.search') }}</UButton>
        <UButton @click="resetFilters" variant="ghost" icon="i-lucide-rotate-ccw">{{ t('common.reset') }}</UButton>
      </div>
    </UCard>

    <!-- Assessments table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">{{ t('common.loading') }}</div>
      <div v-else-if="assessments.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">{{ t('entity.empty') }}</div>
      <div v-else class="overflow-x-auto"><table class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">ID</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('risks.col.target') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('risks.col.score') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('risks.col.level') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('risks.col.decision') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('risks.col.status') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('risks.col.factors') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('risks.col.created') }}</th>
            <th class="py-2 px-3"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="asmt in filteredAssessments" :key="asmt.assessment_id" class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)]">
            <td class="py-2 px-3 font-mono text-xs">{{ asmt.assessment_id }}</td>
            <td class="py-2 px-3">
              <NuxtLink :to="`/entities/${asmt.target_entity_id}`" class="hover:underline">{{ asmt.target_display_name }}</NuxtLink>
            </td>
            <td class="py-2 px-3 font-mono">{{ asmt.score.toFixed(1) }}</td>
            <td class="py-2 px-3">
              <RiskLevelBadge :level="asmt.risk_level" />
            </td>
            <td class="py-2 px-3">
              <UBadge :color="decisionColor(asmt.decision)" size="sm">{{ t(`risks.decision.${asmt.decision}`) }}</UBadge>
            </td>
            <td class="py-2 px-3">
              <UBadge :color="asmt.status === 'approved' ? 'success' : 'warning'" size="sm">{{ t(`risks.status.${asmt.status}`) }}</UBadge>
            </td>
            <td class="py-2 px-3">{{ asmt.factors.length }}</td>
            <td class="py-2 px-3 text-[var(--ui-text-muted)]">{{ formatDate(asmt.created_at) }}</td>
            <td class="py-2 px-3">
              <UButton size="xs" variant="outline" :to="`/entities/${asmt.target_entity_id}`">{{ t('common.view') }}</UButton>
            </td>
          </tr>
        </tbody>
      </table></div>
    </UCard>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import StatCard from '~/components/common/StatCard.vue';
import RiskLevelBadge from '~/components/entity/RiskLevelBadge.vue';

definePageMeta({ middleware: 'auth' });

const { t } = useI18n();
const toast = useToast();
const api = useApi();

interface FactorRow {
  factor_id: string;
  criterion: string;
  severity: string;
  status: string;
  explanation: string;
  detected_at: string;
}

interface AssessmentRow {
  assessment_id: string;
  target_entity_id: string;
  target_display_name: string;
  score: number;
  risk_level: string;
  decision: string;
  status: string;
  created_at: string;
  factors: FactorRow[];
}

const assessments = ref<AssessmentRow[]>([]);
const loading = ref(false);
const filters = reactive({ status: 'all' as string });

const statusOptions = [
  { label: t('risks.filter.all_statuses'), value: 'all' },
  { label: t('risks.status.approved'), value: 'approved' },
  { label: t('risks.status.pending_approval'), value: 'pending_approval' },
];

const stats = computed(() => ({
  minimal: assessments.value.filter((a) => a.risk_level === 'minimal').length,
  low: assessments.value.filter((a) => a.risk_level === 'low').length,
  high: assessments.value.filter((a) => a.risk_level === 'high').length,
  critical: assessments.value.filter((a) => a.risk_level === 'critical').length,
}));

const filteredAssessments = computed(() => {
  if (filters.status === 'all') return assessments.value;
  return assessments.value.filter((a) => a.status === filters.status);
});

function decisionColor(decision: string): string {
  switch (decision) {
    case 'approve': return 'success';
    case 'reject': return 'error';
    case 'escalate': return 'warning';
    case 'defer': return 'neutral';
    default: return 'neutral';
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

function resetFilters() {
  filters.status = 'all';
  fetchAssessments();
}

async function fetchAssessments() {
  loading.value = true;
  try {
    const res = await api.checks.listAssessments();
    assessments.value = res.data as AssessmentRow[];
  } catch (err) {
    toast.add({ title: t('common.error'), description: err instanceof Error ? err.message : String(err), color: 'error' });
  } finally {
    loading.value = false;
  }
}

onMounted(fetchAssessments);
</script>
