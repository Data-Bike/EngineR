<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-5xl">
    <div class="flex items-center justify-between gap-3">
      <div>
        <h1 class="text-2xl font-bold">{{ checkTitle }}</h1>
        <p class="font-mono text-sm text-[var(--ui-text-muted)]">{{ checkId }}</p>
      </div>
      <UButton to="/checks" variant="outline" icon="i-lucide-arrow-left">
        {{ t('nav.checks') }}
      </UButton>
    </div>

    <div v-if="loading" class="py-12 text-center text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>
    <UAlert v-else-if="error" color="error" :title="t('common.error')" :description="error" />

    <template v-else-if="check">
      <UCard>
        <dl class="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-3 text-sm">
          <div>
            <dt class="text-[var(--ui-text-muted)]">{{ t('checks.col.entity') }}</dt>
            <dd>
              <NuxtLink :to="`/entities/${check.target_entity_id}`" class="hover:underline">
                {{ check.target_display_name || check.target_entity_id }}
              </NuxtLink>
            </dd>
          </div>
          <div>
            <dt class="text-[var(--ui-text-muted)]">{{ t('checks.col.kind') }}</dt>
            <dd>{{ enumLabel(t, te, 'checks.kind', check.kind) }}</dd>
          </div>
          <div>
            <dt class="text-[var(--ui-text-muted)]">{{ t('checks.col.status') }}</dt>
            <dd>{{ enumLabel(t, te, 'checks.status', check.status) }}</dd>
          </div>
          <div>
            <dt class="text-[var(--ui-text-muted)]">{{ t('entity.checks.started') }}</dt>
            <dd>{{ formatDate(check.started_at) }}</dd>
          </div>
        </dl>
      </UCard>

      <UCard>
        <template #header>
          <h2 class="font-semibold">{{ t('entity.checks.evidence') }}</h2>
        </template>
        <div v-if="evidence.length === 0" class="py-6 text-center text-[var(--ui-text-muted)]">
          {{ t('entity.empty') }}
        </div>
        <div v-else class="space-y-3">
          <pre
            v-for="(item, index) in evidence"
            :key="index"
            class="overflow-auto rounded-md bg-[var(--ui-bg-muted)] p-3 text-xs"
          >{{ JSON.stringify(item, null, 2) }}</pre>
        </div>
      </UCard>
    </template>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import type { CheckResponse } from '@enginer/api-client';

definePageMeta({ middleware: ['auth', capability('check.view')] });

const { t, te } = useI18n();
const route = useRoute();
const api = useApi();
const checkId = computed(() => String(route.params.id ?? ''));
const check = ref<CheckResponse | null>(null);
const evidence = ref<Array<Record<string, unknown>>>([]);
const loading = ref(false);
const error = ref<string | null>(null);

// Page title + breadcrumb label: "kind — target" instead of the raw uuid.
const checkTitle = computed(() =>
  check.value
    ? `${enumLabel(t, te, 'checks.kind', check.value.kind)} — ${check.value.target_display_name || checkId.value}`
    : t('nav.checks'),
);
const { setBreadcrumbLabel } = useBreadcrumbLabels();
watch(
  () => check.value?.target_display_name,
  () => setBreadcrumbLabel(checkId.value, check.value ? checkTitle.value : null),
);
onUnmounted(() => setBreadcrumbLabel(checkId.value, null));

function formatDate(value: string): string {
  return new Date(value).toLocaleString();
}

async function fetchCheck() {
  loading.value = true;
  error.value = null;
  try {
    const [checkResponse, evidenceResponse] = await Promise.all([
      api.checks.getCheck(checkId.value),
      api.checks.getCheckEvidence(checkId.value),
    ]);
    check.value = checkResponse.data;
    evidence.value = evidenceResponse.data.evidence ?? [];
  } catch (cause: unknown) {
    error.value = displayError(cause, t, te);
  } finally {
    loading.value = false;
  }
}

onMounted(fetchCheck);
watch(checkId, fetchCheck);
</script>
