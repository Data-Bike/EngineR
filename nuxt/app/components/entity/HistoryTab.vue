<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h3 class="font-medium">{{ t('entity.tabs.history') }}</h3>
      <UButton
        v-if="revisions.length > 0"
        size="sm"
        variant="outline"
        icon="i-lucide-git-compare"
        @click="showCompare = !showCompare"
      >
        {{ t('entity.history.compare') }}
      </UButton>
    </div>

    <!-- Revision timeline -->
    <div v-if="revisions.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>
    <div v-else class="space-y-3">
      <div
        v-for="rev in revisions"
        :key="rev.revision"
        class="relative pl-6 border-l-2 border-[var(--ui-border)] pb-4"
      >
        <!-- Timeline dot -->
        <div
          class="absolute -left-[5px] top-1 w-2 h-2 rounded-full"
          :class="rev.revision === currentRevision ? 'bg-blue-500' : 'bg-[var(--ui-border-muted)]'"
        />

        <!-- Revision card -->
        <div class="rounded-md border border-[var(--ui-border)] p-3">
          <div class="flex items-center justify-between">
            <div>
              <p class="font-medium">
                {{ t('entity.history.revision') }} {{ rev.revision }}
                <UBadge v-if="rev.revision === currentRevision" color="primary" size="sm" class="ml-1">
                  {{ t('entity.history.current') }}
                </UBadge>
              </p>
              <p class="text-sm text-[var(--ui-text-muted)]">
                {{ rev.timestamp }} · {{ rev.actor }}
              </p>
            </div>
            <div class="flex items-center gap-2">
              <UBadge variant="subtle" size="sm">{{ rev.operation }}</UBadge>
              <UButton
                v-if="showCompare && selectedRev !== rev.revision"
                size="xs"
                variant="ghost"
                @click="selectForCompare(rev.revision)"
              >
                {{ t('entity.history.select_compare') }}
              </UButton>
            </div>
          </div>

          <!-- Changed fields diff (docs/17 §8) -->
          <div v-if="rev.changes && rev.changes.length > 0" class="mt-2 space-y-1">
            <div
              v-for="change in rev.changes"
              :key="change.predicate_id"
              class="text-sm"
            >
              <span class="text-[var(--ui-text-muted)]">{{ change.alias }}:</span>
              <span class="line-through text-red-500 ml-1">{{ change.old_value ?? '—' }}</span>
              <UIcon name="i-lucide-arrow-right" class="inline text-xs mx-1" />
              <span class="text-green-600">{{ change.new_value ?? '—' }}</span>
            </div>
          </div>

          <!-- Reason -->
          <p v-if="rev.reason" class="text-xs text-[var(--ui-text-muted)] mt-2">
            {{ t('entity.history.reason') }}: {{ rev.reason }}
          </p>
        </div>
      </div>
    </div>

    <!-- Compare dialog -->
    <UModal v-model:open="compareDialogOpen">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.history.compare') }}</h2>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <p class="text-sm font-medium mb-1">{{ t('entity.history.revision') }} {{ compareFrom }}</p>
              <div class="rounded-md border border-[var(--ui-border)] p-2 text-sm space-y-1">
                <div v-for="field in compareFromFields" :key="field.alias">
                  <span class="text-[var(--ui-text-muted)]">{{ field.alias }}:</span>
                  <span class="ml-1">{{ field.value ?? '—' }}</span>
                </div>
              </div>
            </div>
            <div>
              <p class="text-sm font-medium mb-1">{{ t('entity.history.revision') }} {{ compareTo }}</p>
              <div class="rounded-md border border-[var(--ui-border)] p-2 text-sm space-y-1">
                <div v-for="field in compareToFields" :key="field.alias">
                  <span class="text-[var(--ui-text-muted)]">{{ field.alias }}:</span>
                  <span class="ml-1">{{ field.value ?? '—' }}</span>
                </div>
              </div>
            </div>
          </div>
          <div class="flex justify-end">
            <UButton variant="outline" @click="compareDialogOpen = false">{{ t('common.close') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps<{
  entityId: string;
  currentRevision: number;
}>();

const { t } = useI18n();
const toast = useToast();
const api = useApi();

interface FieldChange {
  predicate_id: string;
  alias: string;
  old_value?: string;
  new_value?: string;
}

interface RevisionEntry {
  revision: number;
  timestamp: string;
  actor: string;
  operation: 'create' | 'update' | 'retire' | 'merge' | 'relation';
  changes?: FieldChange[];
  reason?: string;
}

const revisions = ref<RevisionEntry[]>([]);
const showCompare = ref(false);
const selectedRev = ref<number | null>(null);
const compareDialogOpen = ref(false);

const compareFrom = ref<number | null>(null);
const compareTo = ref<number | null>(null);
const compareFromFields = ref<{ alias: string; value?: string }[]>([]);
const compareToFields = ref<{ alias: string; value?: string }[]>([]);

function selectForCompare(rev: number) {
  if (compareFrom.value === null) {
    compareFrom.value = rev;
    selectedRev.value = rev;
  } else {
    compareTo.value = rev;
    compareFromFields.value = fieldsAtRevision(compareFrom.value);
    compareToFields.value = fieldsAtRevision(rev);
    compareDialogOpen.value = true;
  }
}

function fieldsAtRevision(revision: number) {
  const values = new Map<string, string | undefined>();
  for (const entry of revisions.value
    .filter((item) => item.revision <= revision)
    .sort((left, right) => left.revision - right.revision)) {
    for (const change of entry.changes ?? []) {
      values.set(change.alias, change.new_value);
    }
  }
  return [...values.entries()].map(([alias, value]) => ({ alias, value }));
}

async function fetchHistory() {
  try {
    const res = await api.entities.getHistory(props.entityId);
    revisions.value = (res.data ?? []) as RevisionEntry[];
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
    revisions.value = [];
  }
}

onMounted(fetchHistory);
</script>
