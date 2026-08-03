<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h3 class="font-medium">{{ t('entity.tabs.risks') }}</h3>
      <UButton size="sm" icon="i-lucide-clipboard-check" @click="showAssessment = true">
        {{ t('entity.risks.new_assessment') }}
      </UButton>
    </div>

    <!-- Current assessment summary (docs/04 §5) -->
    <UCard v-if="currentAssessment">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm text-[var(--ui-text-muted)]">{{ t('entity.risks.overall_level') }}</p>
          <div class="flex items-center gap-2 mt-1">
            <RiskLevelBadge :level="currentAssessment.overall_level" />
            <UBadge v-if="currentAssessment.approval_pending" color="warning" size="sm">
              {{ t('entity.risks.approval_pending') }}
            </UBadge>
          </div>
        </div>
        <div class="text-right">
          <p class="text-sm text-[var(--ui-text-muted)]">{{ t('entity.risks.assessed_by') }}</p>
          <p class="font-medium">{{ currentAssessment.assessor }}</p>
          <p class="text-xs text-[var(--ui-text-muted)]">{{ currentAssessment.assessed_at }}</p>
        </div>
      </div>
      <!-- Decision -->
      <div class="mt-3 pt-3 border-t border-[var(--ui-border)]">
        <p class="text-sm">
          <span class="text-[var(--ui-text-muted)]">{{ t('entity.risks.decision_label') }}:</span>
          <UBadge :color="decisionColor(currentAssessment.decision)" size="sm" class="ml-1">
            {{ t(`entity.risks.decision.${currentAssessment.decision}`) }}
          </UBadge>
        </p>
        <p v-if="currentAssessment.rationale" class="text-sm text-[var(--ui-text-muted)] mt-1">
          {{ currentAssessment.rationale }}
        </p>
      </div>
    </UCard>

    <!-- Risk factors (docs/04 §5, docs/05) -->
    <div v-if="factors.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>
    <div v-else class="space-y-2">
      <p class="text-sm font-medium">{{ t('entity.risks.factors') }} ({{ factors.length }})</p>
      <div
        v-for="factor in factors"
        :key="factor.factor_id"
        class="rounded-md border border-[var(--ui-border)] p-3"
      >
        <div class="flex items-start justify-between">
          <div class="flex-1">
            <div class="flex items-center gap-2">
              <p class="font-medium">{{ factor.label }}</p>
              <RiskLevelBadge :level="factor.level" />
              <UBadge v-if="factor.resolved" color="success" size="sm">
                {{ t('entity.risks.resolved') }}
              </UBadge>
            </div>
            <p class="text-sm text-[var(--ui-text-muted)] mt-1">{{ factor.explanation }}</p>
            <p class="text-xs text-[var(--ui-text-muted)] mt-1">
              {{ t('entity.risks.detected') }}: {{ factor.detected_at }}
              <span v-if="factor.resolved_at"> · {{ t('entity.risks.resolved_at') }}: {{ factor.resolved_at }}</span>
            </p>
            <!-- Source checks -->
            <div v-if="factor.source_checks.length > 0" class="flex flex-wrap gap-1 mt-1">
              <UBadge
                v-for="checkId in factor.source_checks"
                :key="checkId"
                variant="subtle"
                size="sm"
              >
                {{ checkId.slice(0, 8) }}
              </UBadge>
            </div>
          </div>
          <!-- Resolve action -->
          <UButton
            v-if="!factor.resolved && canResolve"
            size="xs"
            variant="outline"
            @click="resolveFactor(factor.factor_id)"
          >
            {{ t('entity.risks.resolve') }}
          </UButton>
        </div>
      </div>
    </div>

    <!-- New assessment dialog -->
    <UModal v-model:open="showAssessment">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.risks.new_assessment') }}</h2>
          <UFormField :label="t('entity.risks.overall_level')">
            <USelect v-model="newLevel" :items="levelOptions" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.risks.decision_label')">
            <USelect v-model="newDecision" :items="decisionOptions" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.risks.rationale')">
            <UTextarea v-model="newRationale" :rows="3" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showAssessment = false">{{ t('common.cancel') }}</UButton>
            <UButton @click="submitAssessment" :loading="submitting">{{ t('common.save') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { RiskFactor, RiskAssessment, RiskLevel, RiskDecision } from '@enginer/api-client';
import RiskLevelBadge from './RiskLevelBadge.vue';

const props = defineProps<{
  entityId: string;
}>();

const { t } = useI18n();
const { hasCapability } = useCapability();
const toast = useToast();

const factors = ref<RiskFactor[]>([]);
const currentAssessment = ref<RiskAssessment | null>(null);
const showAssessment = ref(false);
const submitting = ref(false);
const newLevel = ref<RiskLevel>('low');
const newDecision = ref<RiskDecision>('review');
const newRationale = ref('');

const canResolve = computed(() => hasCapability('risk.resolve'));

const levelOptions = computed(() => [
  { label: t('entity.risks.level.low'), value: 'low' },
  { label: t('entity.risks.level.medium'), value: 'medium' },
  { label: t('entity.risks.level.high'), value: 'high' },
  { label: t('entity.risks.level.critical'), value: 'critical' },
]);

const decisionOptions = computed(() => [
  { label: t('entity.risks.decision.accept'), value: 'accept' },
  { label: t('entity.risks.decision.reject'), value: 'reject' },
  { label: t('entity.risks.decision.escalate'), value: 'escalate' },
  { label: t('entity.risks.decision.review'), value: 'review' },
]);

type BadgeColor = 'primary' | 'secondary' | 'success' | 'info' | 'warning' | 'error' | 'neutral';

function decisionColor(decision: RiskDecision): BadgeColor {
  const colors: Record<RiskDecision, BadgeColor> = {
    accept: 'success',
    reject: 'error',
    escalate: 'warning',
    review: 'neutral',
  };
  return colors[decision];
}

interface AssessmentResponse {
  factors: RiskFactor[];
  score?: number;
  risk_level?: string;
  decision?: string;
  status?: string;
  assessor?: string;
  assessed_at?: string;
  rationale?: string;
  approval_pending?: boolean;
}

async function fetchRisks() {
  const config = useRuntimeConfig();
  const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
  try {
    const res = await $fetch<{ data: AssessmentResponse | null }>(`${baseUrl.replace(/\/$/, '')}/entities/${props.entityId}/assessment`, {
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    if (res.data) {
      factors.value = res.data.factors ?? [];
      currentAssessment.value = {
        assessment_id: '',
        entity_public_id: props.entityId,
        overall_level: (res.data.risk_level ?? 'low') as RiskAssessment['overall_level'],
        factors: res.data.factors ?? [],
        assessor: res.data.assessor ?? '',
        assessed_at: res.data.assessed_at ?? '',
        decision: (res.data.decision ?? 'review') as RiskAssessment['decision'],
        rationale: res.data.rationale,
        approval_pending: res.data.approval_pending ?? false,
      };
    } else {
      factors.value = [];
      currentAssessment.value = null;
    }
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
    factors.value = [];
    currentAssessment.value = null;
  }
}

async function resolveFactor(factorId: string) {
  const config = useRuntimeConfig();
  const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
  try {
    await $fetch(`${baseUrl.replace(/\/$/, '')}/entities/${props.entityId}/factors/${factorId}/resolve`, {
      method: 'POST',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({ title: t('entity.risks.resolved'), color: 'success' });
    await fetchRisks();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  }
}

async function submitAssessment() {
  submitting.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/entities/${props.entityId}/assessment`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: {
        overall_level: newLevel.value,
        decision: newDecision.value,
        rationale: newRationale.value,
      },
    });
    toast.add({ title: t('entity.risks.new_assessment'), color: 'success' });
    showAssessment.value = false;
    newRationale.value = '';
    await fetchRisks();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    submitting.value = false;
  }
}

onMounted(fetchRisks);
</script>
