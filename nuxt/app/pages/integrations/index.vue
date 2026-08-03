<template>
  <div class="p-6 space-y-6">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('nav.integrations') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('integrations.subtitle') }}</p>
      </div>
      <UButton icon="i-lucide-plus" @click="showNewDialog = true">
        {{ t('integrations.new') }}
      </UButton>
    </div>

    <!-- Summary cards -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <StatCard
        :label="t('integrations.total')"
        :value="connectors.length"
        icon="i-lucide-plug"
      />
      <StatCard
        :label="t('integrations.active')"
        :value="activeCount"
        icon="i-lucide-check-circle"
      />
      <StatCard
        :label="t('integrations.disabled')"
        :value="disabledCount"
        icon="i-lucide-pause-circle"
      />
      <StatCard
        :label="t('integrations.healthy')"
        :value="healthyCount"
        icon="i-lucide-heart-pulse"
      />
    </div>

    <!-- Connectors table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('common.loading') }}
      </div>
      <div v-else-if="connectors.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">
        {{ t('entity.empty') }}
      </div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('integrations.col.id') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('integrations.col.type') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('integrations.col.environment') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('integrations.col.enabled') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('integrations.col.rate_limit') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('integrations.col.trust') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('entity.actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="conn in connectors"
            :key="conn.instance_id"
            class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)]"
          >
            <td class="py-2 px-3 font-mono text-xs">{{ conn.instance_id.slice(0, 8) }}</td>
            <td class="py-2 px-3">
              <UBadge variant="subtle" size="sm">{{ conn.connector_type_id }}</UBadge>
            </td>
            <td class="py-2 px-3">{{ conn.environment }}</td>
            <td class="py-2 px-3">
              <UBadge :color="conn.enabled ? 'success' : 'neutral'" size="sm">
                {{ conn.enabled ? t('common.enabled') : t('common.disabled') }}
              </UBadge>
            </td>
            <td class="py-2 px-3">{{ conn.rate_limit_rps }} rps</td>
            <td class="py-2 px-3">
              <div class="flex items-center gap-2">
                <div class="w-16 h-2 bg-[var(--ui-bg-muted)] rounded-full overflow-hidden">
                  <div
                    class="h-full rounded-full"
                    :class="trustColor(conn.trust_rank)"
                    :style="{ width: `${conn.trust_rank}%` }"
                  />
                </div>
                <span class="text-xs text-[var(--ui-text-muted)]">{{ conn.trust_rank }}</span>
              </div>
            </td>
            <td class="py-2 px-3">
              <div class="flex gap-1">
                <UButton
                  size="xs"
                  variant="outline"
                  icon="i-lucide-heart-pulse"
                  @click="runHealthCheck(conn.instance_id)"
                  :loading="healthChecking === conn.instance_id"
                />
                <UButton
                  v-if="conn.enabled"
                  size="xs"
                  variant="outline"
                  icon="i-lucide-pause"
                  @click="toggleConnector(conn, false)"
                />
                <UButton
                  v-else
                  size="xs"
                  variant="outline"
                  icon="i-lucide-play"
                  @click="toggleConnector(conn, true)"
                />
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </UCard>

    <!-- New connector dialog -->
    <UModal v-model:open="showNewDialog">
      <template #content>
        <UCard>
          <template #header>
            <h3 class="text-lg font-semibold">{{ t('integrations.new') }}</h3>
          </template>
          <div class="space-y-4">
            <UFormField :label="t('integrations.form.type')">
              <UInput v-model="newConnector.connector_type_id" placeholder="fns_egru" class="w-full" />
            </UFormField>
            <UFormField :label="t('integrations.form.environment')">
              <USelect
                v-model="newConnector.environment"
                :items="envOptions"
                class="w-full"
              />
            </UFormField>
            <UFormField :label="t('integrations.form.rate_limit')">
              <UInput v-model.number="newConnector.rate_limit_rps" type="number" class="w-full" />
            </UFormField>
            <UFormField :label="t('integrations.form.trust_rank')">
              <USelect
                v-model="newConnector.trust_rank"
                :items="trustOptions"
                value-key="value"
                option-label="label"
                class="w-full"
              />
            </UFormField>
          </div>
          <template #footer>
            <div class="flex justify-end gap-2">
              <UButton variant="outline" @click="showNewDialog = false">{{ t('common.cancel') }}</UButton>
              <UButton @click="submitNew" :loading="submitting">{{ t('common.submit') }}</UButton>
            </div>
          </template>
        </UCard>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useApi } from '~/composables/useApi';
import type { ConnectorInstance } from '@enginer/api-client';

definePageMeta({
  middleware: 'auth',
});

const { t } = useI18n();
const api = useApi();
const toast = useToast();

const connectors = ref<ConnectorInstance[]>([]);
const loading = ref(false);
const showNewDialog = ref(false);
const submitting = ref(false);
const healthChecking = ref<string | null>(null);

const newConnector = reactive({
  connector_type_id: '',
  environment: 'prod',
  rate_limit_rps: 10,
  trust_rank: 75,
});

const envOptions = ['dev', 'staging', 'prod'];
const trustOptions = [
  { label: t('integrations.trust.untrusted'), value: 0 },
  { label: t('integrations.trust.low'), value: 25 },
  { label: t('integrations.trust.medium'), value: 50 },
  { label: t('integrations.trust.high'), value: 75 },
  { label: t('integrations.trust.authoritative'), value: 100 },
];

const activeCount = computed(() => connectors.value.filter((c) => c.enabled).length);
const disabledCount = computed(() => connectors.value.filter((c) => !c.enabled).length);
const healthyCount = computed(() => connectors.value.filter((c) => c.enabled).length);

function trustColor(rank: number): string {
  if (rank >= 75) return 'bg-green-500';
  if (rank >= 50) return 'bg-blue-500';
  if (rank >= 25) return 'bg-yellow-500';
  return 'bg-red-500';
}

async function fetchConnectors() {
  loading.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: ConnectorInstance[] }>(`${baseUrl.replace(/\/$/, '')}/connectors`, {
      method: 'GET',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    connectors.value = res.data;
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

async function toggleConnector(conn: ConnectorInstance, enable: boolean) {
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const endpoint = enable ? 'enable' : 'disable';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/connectors/${conn.instance_id}/${endpoint}`, {
      method: 'POST',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({
      title: enable ? t('integrations.enabled_ok') : t('integrations.disabled_ok'),
      color: 'success',
    });
    await fetchConnectors();
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  }
}

async function runHealthCheck(id: string) {
  healthChecking.value = id;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const res = await $fetch<{ data: { healthy: boolean; latency_ms: number; message: string } }>(
      `${baseUrl.replace(/\/$/, '')}/connectors/${id}/health`,
      {
        method: 'POST',
        credentials: 'include',
        headers: { Accept: 'application/json' },
      },
    );
    toast.add({
      title: res.data.healthy ? t('integrations.healthy_ok') : t('integrations.healthy_fail'),
      description: `${res.data.latency_ms}ms — ${res.data.message}`,
      color: res.data.healthy ? 'success' : 'error',
    });
  } catch (err) {
    toast.add({
      title: t('common.error'),
      description: err instanceof Error ? err.message : String(err),
      color: 'error',
    });
  } finally {
    healthChecking.value = null;
  }
}

async function submitNew() {
  submitting.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/connectors`, {
      method: 'POST',
      body: {
        connector_type_id: newConnector.connector_type_id,
        environment: newConnector.environment,
        rate_limit_rps: newConnector.rate_limit_rps,
        trust_rank: newConnector.trust_rank,
      },
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    showNewDialog.value = false;
    toast.add({ title: t('integrations.created_ok'), color: 'success' });
    await fetchConnectors();
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

onMounted(fetchConnectors);
</script>
