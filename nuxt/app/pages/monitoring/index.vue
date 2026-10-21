<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div>
      <h1 class="text-2xl font-bold">{{ t('nav.monitoring') }}</h1>
      <p class="text-[var(--ui-text-muted)] mt-1">{{ t('entity.monitoring.subtitle') }}</p>
    </div>

    <!-- Summary cards -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <StatCard
        :label="t('entity.monitoring.active')"
        :value="activeCount"
        icon="i-lucide-eye"
      />
      <StatCard
        :label="t('entity.monitoring.due')"
        :value="dueCount"
        icon="i-lucide-clock"
      />
      <StatCard
        :label="t('entity.monitoring.escalations')"
        :value="escalationCount"
        icon="i-lucide-alert-triangle"
      />
    </div>

    <!-- Monitors table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('common.loading') }}
      </div>
      <div v-else-if="monitors.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('entity.empty') }}
      </div>
      <div v-else class="overflow-x-auto"><table class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.monitoring.entity') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.monitoring.frequency') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.monitoring.next_run') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.monitoring.last_run') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.monitoring.last_result') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="mon in monitors"
            :key="mon.monitor_id"
            class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)]"
          >
            <td class="py-2 px-3">
              <NuxtLink :to="`/entities/${mon.entity_public_id}`" class="hover:underline">
                {{ mon.entity_public_id }}
              </NuxtLink>
            </td>
            <td class="py-2 px-3">
              <UBadge variant="subtle" size="sm">{{ enumLabel(t, te, 'entity.monitoring.freq', mon.frequency) }}</UBadge>
            </td>
            <td class="py-2 px-3" :class="{ 'text-red-500': isOverdue(mon.next_run) }">
              {{ formatDateTime(mon.next_run) }}
            </td>
            <td class="py-2 px-3">{{ formatDateTime(mon.last_run) }}</td>
            <td class="py-2 px-3">
              <div v-if="mon.last_result" class="flex items-center gap-2">
                <span>{{ mon.last_result.checks_completed }}/{{ mon.last_result.checks_completed + mon.last_result.checks_failed }}</span>
                <UBadge v-if="mon.last_result.new_factors > 0" color="error" size="sm">
                  +{{ mon.last_result.new_factors }}
                </UBadge>
                <UBadge v-if="mon.last_result.escalate" color="warning" size="sm">
                  {{ t('entity.monitoring.escalate') }}
                </UBadge>
              </div>
              <span v-else class="text-[var(--ui-text-muted)]">—</span>
            </td>
            <td class="py-2 px-3">
              <UButton size="xs" variant="outline" @click="runNow(mon.monitor_id)" :loading="running === mon.monitor_id">
                {{ t('entity.monitoring.run_now') }}
              </UButton>
            </td>
          </tr>
        </tbody>
      </table></div>
    </UCard>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MonitorSchedule } from '@enginer/api-client';

definePageMeta({
  middleware: ['auth', capability('monitor.manage')],
});

const { t, te } = useI18n();
const toast = useToast();
const api = useApi();

const monitors = ref<MonitorSchedule[]>([]);
const loading = ref(false);
const running = ref<string | null>(null);

const activeCount = computed(() => monitors.value.filter((m) => m.active).length);
const dueCount = computed(() => monitors.value.filter((m) => isOverdue(m.next_run)).length);
const escalationCount = computed(() => monitors.value.filter((m) => m.last_result?.escalate).length);

function isOverdue(date: string): boolean {
  return new Date(date) < new Date();
}

async function fetchMonitors() {
  loading.value = true;
  try {
    const res = await api.monitors.list();
    monitors.value = res.data;
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: displayError(err, t, te),
      color: 'error',
    });
  } finally {
    loading.value = false;
  }
}

async function runNow(monitorId: string) {
  running.value = monitorId;
  try {
    await api.monitors.run(monitorId);
    toast.add({ title: t('entity.monitoring.run_now'), color: 'success' });
    await fetchMonitors();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: displayError(err, t, te),
      color: 'error',
    });
  } finally {
    running.value = null;
  }
}

onMounted(fetchMonitors);
</script>
