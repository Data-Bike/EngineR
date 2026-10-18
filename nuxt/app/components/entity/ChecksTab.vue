<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between flex-wrap gap-2">
      <h3 class="font-medium">{{ t('entity.tabs.checks') }}</h3>
      <div class="flex items-center gap-2">
        <USelect
          v-model="selectedKind"
          :items="checkKinds"
          size="sm"
          class="w-40"
        />
        <UButton size="sm" icon="i-lucide-play" @click="runCheck" :loading="running">
          {{ t('entity.checks.run') }}
        </UButton>
      </div>
    </div>

    <!-- Check instances -->
    <div v-if="checks.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>
    <div v-else class="space-y-2">
      <div
        v-for="check in checks"
        :key="check.check_id"
        class="rounded-md border border-[var(--ui-border)] p-3"
      >
        <div class="flex items-start justify-between">
          <div class="flex items-start gap-3 flex-1">
            <!-- Status icon (not color-only, docs/17 §7) -->
            <UIcon :name="checkStatusIcon(check.status)" class="text-xl shrink-0" :class="checkStatusClass(check.status)" />
            <div class="flex-1">
              <!-- Kind + status -->
              <div class="flex items-center gap-2">
                <p class="font-medium">{{ t(`entity.checks.kind.${check.kind}`) }}</p>
                <UBadge :color="checkStatusColor(check.status)" size="sm">
                  {{ t(`entity.checks.status.${check.status}`) }}
                </UBadge>
              </div>
              <!-- Timestamps -->
              <p class="text-xs text-[var(--ui-text-muted)] mt-1">
                <span v-if="check.started_at">{{ t('entity.checks.started') }}: {{ check.started_at }}</span>
                <span v-if="check.completed_at"> · {{ t('entity.checks.completed') }}: {{ check.completed_at }}</span>
              </p>
              <!-- Connector -->
              <p v-if="check.connector_id" class="text-xs text-[var(--ui-text-muted)] mt-1">
                {{ t('entity.checks.connector') }}: {{ check.connector_id }}
              </p>
              <!-- Error -->
              <p v-if="check.error" class="text-sm text-red-500 mt-1">{{ check.error }}</p>

              <!-- Step-up required (docs/16 §5) -->
              <div v-if="check.step_up_required" class="mt-2 rounded-md bg-yellow-50 border border-yellow-300 p-2">
                <p class="text-sm text-yellow-800">{{ t('entity.checks.step_up_required') }}</p>
                <UButton size="xs" variant="outline" @click="stepUp(check.check_id)">
                  {{ t('auth.step_up_required') }}
                </UButton>
              </div>

              <!-- Data denied (docs/16 §5) -->
              <div v-if="check.data_denied && !check.step_up_required" class="mt-2 text-sm text-[var(--ui-text-muted)]">
                {{ t('entity.checks.data_denied') }}
              </div>

              <!-- Evidence items (docs/04 §4) -->
              <div v-if="!check.step_up_required && !check.data_denied && check.evidence.length > 0" class="mt-2 space-y-1">
                <p class="text-xs font-medium text-[var(--ui-text-muted)]">{{ t('entity.checks.evidence') }} ({{ check.evidence.length }})</p>
                <div
                  v-for="ev in check.evidence"
                  :key="ev.evidence_id"
                  class="text-sm rounded-md bg-[var(--ui-bg-muted)] p-2"
                >
                  <div class="flex items-start justify-between">
                    <div class="flex-1">
                      <p>{{ ev.summary }}</p>
                      <p class="text-xs text-[var(--ui-text-muted)] mt-1">
                        {{ ev.source }} · {{ ev.retrieved_at }}
                        <UBadge v-if="ev.adverse" color="error" size="sm" class="ml-1">{{ t('entity.checks.adverse') }}</UBadge>
                      </p>
                      <NuxtLink v-if="ev.source_url" :to="ev.source_url" target="_blank" class="text-xs text-blue-500 hover:underline">
                        {{ t('entity.checks.source_link') }}
                      </NuxtLink>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CheckInstance, CheckStatus } from '@enginer/api-client';

const props = defineProps<{
  entityId: string;
}>();

const { t } = useI18n();
const toast = useToast();
const api = useApi();

const checks = ref<CheckInstance[]>([]);
const running = ref(false);
const selectedKind = ref('kyc');
const checkKinds = [
  { label: 'KYC', value: 'kyc' },
  { label: 'Sanctions', value: 'sanctions' },
  { label: 'Financial Health', value: 'financial_health' },
  { label: 'Legal Status', value: 'legal_status' },
  { label: 'Ownership', value: 'ownership' },
];

function checkStatusIcon(status: CheckStatus): string {
  const icons: Record<CheckStatus, string> = {
    pending: 'i-lucide-clock',
    running: 'i-lucide-loader',
    completed: 'i-lucide-check-circle',
    failed: 'i-lucide-x-circle',
    skipped: 'i-lucide-skip-forward',
    cancelled: 'i-lucide-ban',
  };
  return icons[status] ?? 'i-lucide-circle';
}

function checkStatusClass(status: CheckStatus): string {
  const classes: Record<CheckStatus, string> = {
    pending: 'text-[var(--ui-text-muted)]',
    running: 'text-blue-500 animate-spin',
    completed: 'text-green-500',
    failed: 'text-red-500',
    skipped: 'text-[var(--ui-text-muted)]',
    cancelled: 'text-[var(--ui-text-muted)]',
  };
  return classes[status] ?? '';
}

type BadgeColor = 'primary' | 'secondary' | 'success' | 'info' | 'warning' | 'error' | 'neutral';

function checkStatusColor(status: CheckStatus): BadgeColor {
  const colors: Record<CheckStatus, BadgeColor> = {
    pending: 'neutral',
    running: 'primary',
    completed: 'success',
    failed: 'error',
    skipped: 'neutral',
    cancelled: 'neutral',
  };
  return colors[status] ?? 'neutral';
}

async function fetchChecks() {
  try {
    const res = await api.checks.getEntityChecks(props.entityId);
    checks.value = (res.data ?? []).map((check) => ({
      ...check,
      entity_public_id: check.target_entity_id,
      kind: check.kind as CheckInstance['kind'],
      status: check.status as CheckInstance['status'],
      completed_at: check.completed_at ?? undefined,
      evidence: (check.evidence ?? []) as unknown as CheckInstance['evidence'],
    }));
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
    checks.value = [];
  }
}

async function runCheck() {
  running.value = true;
  try {
    const kind = selectedKind.value;
    await api.checks.requestCheck({ target_entity_id: props.entityId, check_kind: kind, connector_instance_id: undefined, parameters: {} });
    toast.add({ title: t('entity.checks.run'), color: 'success' });
    await fetchChecks();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    running.value = false;
  }
}

async function stepUp(_checkId: string) {
  const route = useRoute();
  await navigateTo({ path: '/login', query: { redirect: route.fullPath, step_up: '1' } });
}

onMounted(fetchChecks);
</script>
