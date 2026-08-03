<template>
  <div class="p-6 space-y-6">
    <!-- Page header -->
    <div>
      <h1 class="text-2xl font-bold">{{ t('nav.home') }}</h1>
      <p class="text-[var(--ui-text-muted)] mt-1">{{ t('app.tagline') }}</p>
    </div>

    <!-- Dashboard cards (docs/17 §2) -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <StatCard
        :label="t('nav.requests')"
        :value="stats.newRequests"
        icon="i-lucide-clipboard-list"
        to="/requests"
      />
      <StatCard
        :label="t('nav.checks')"
        :value="stats.activeChecks"
        icon="i-lucide-shield-check"
        to="/checks"
      />
      <StatCard
        :label="t('nav.risks')"
        :value="stats.criticalFactors"
        icon="i-lucide-alert-triangle"
        to="/risks"
      />
      <StatCard
        :label="t('nav.monitoring')"
        :value="stats.monitorsDue"
        icon="i-lucide-eye"
        to="/monitoring"
      />
      <StatCard
        :label="t('nav.reports')"
        :value="stats.readyReports"
        icon="i-lucide-file-text"
        to="/reports"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
definePageMeta({
  middleware: 'auth',
});

const { t } = useI18n();

// Placeholder stats — will be fetched from API
const stats = reactive({
  newRequests: 0,
  activeChecks: 0,
  criticalFactors: 0,
  monitorsDue: 0,
  readyReports: 0,
});
</script>
