<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div>
      <h1 class="text-2xl font-bold">{{ t('nav.home') }}</h1>
      <p class="text-[var(--ui-text-muted)] mt-1">{{ t('app.tagline') }}</p>
    </div>

    <!-- Dashboard cards (docs/17 §2) -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <StatCard
        :label="t('dashboard.my_tasks')"
        :value="stats.my_tasks"
        icon="i-lucide-clipboard-list"
        to="/requests"
      />
      <StatCard
        :label="t('dashboard.overdue_tasks')"
        :value="stats.overdue_tasks"
        icon="i-lucide-alert-octagon"
        to="/requests"
      />
      <StatCard
        :label="t('nav.requests')"
        :value="stats.new_requests"
        icon="i-lucide-inbox"
        to="/requests"
      />
      <StatCard
        :label="t('dashboard.checks_in_progress')"
        :value="stats.checks_in_progress"
        icon="i-lucide-shield-check"
        to="/checks"
      />
      <StatCard
        :label="t('dashboard.critical_factors')"
        :value="stats.critical_factors"
        icon="i-lucide-alert-triangle"
        to="/risks"
      />
      <StatCard
        :label="t('dashboard.monitors_due')"
        :value="stats.monitors_due"
        icon="i-lucide-eye"
        to="/monitoring"
      />
      <StatCard
        :label="t('dashboard.ready_reports')"
        :value="stats.ready_reports"
        icon="i-lucide-file-text"
        to="/reports"
      />
      <StatCard
        :label="t('dashboard.checks_errors')"
        :value="stats.checks_errors"
        icon="i-lucide-x-circle"
        to="/checks"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import StatCard from '~/components/common/StatCard.vue';

definePageMeta({ middleware: 'auth' });

const { t } = useI18n();
const toast = useToast();
const api = useApi();

const stats = reactive({
  my_tasks: 0,
  overdue_tasks: 0,
  new_requests: 0,
  checks_in_progress: 0,
  checks_errors: 0,
  critical_factors: 0,
  monitors_due: 0,
  ready_reports: 0,
});

async function fetchStats() {
  try {
    const res = await api.dashboard.stats();
    Object.assign(stats, res.data);
  } catch (err) {
    // Silent fail — dashboard stats are non-critical
    console.error('Failed to fetch dashboard stats:', err);
  }
}

onMounted(fetchStats);
</script>
