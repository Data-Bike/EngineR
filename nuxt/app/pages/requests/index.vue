<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('nav.requests') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('requests.subtitle') }}</p>
      </div>
      <UButton icon="i-lucide-plus" @click="showNewDialog = true">
        {{ t('requests.new') }}
      </UButton>
    </div>

    <!-- Summary cards -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <StatCard icon="i-lucide-clock" :value="stats.pending" :label="t('requests.status.pending')" color="neutral" />
      <StatCard icon="i-lucide-loader" :value="stats.inProgress" :label="t('requests.status.in_progress')" color="info" />
      <StatCard icon="i-lucide-check-circle" :value="stats.completed" :label="t('requests.status.completed')" color="success" />
      <StatCard icon="i-lucide-alert-circle" :value="stats.overdue" :label="t('requests.overdue')" color="error" />
    </div>

    <!-- Filters -->
    <UCard>
      <div class="flex flex-wrap gap-3 items-end">
        <UFormField :label="t('requests.filter.status')">
          <USelect v-model="filters.status" :items="statusOptions" value-key="value" class="w-full sm:w-48" />
        </UFormField>
        <UButton @click="fetchRequests" icon="i-lucide-search">{{ t('common.search') }}</UButton>
        <UButton @click="resetFilters" variant="ghost" icon="i-lucide-rotate-ccw">{{ t('common.reset') }}</UButton>
      </div>
    </UCard>

    <!-- Requests table -->
    <UCard>
      <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">{{ t('common.loading') }}</div>
      <div v-else-if="requests.length === 0" class="py-8 text-center text-[var(--ui-text-muted)]">{{ t('entity.empty') }}</div>
      <div v-else class="overflow-x-auto"><table class="w-full text-sm">
        <thead>
          <tr class="border-b border-[var(--ui-border)] text-left">
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">ID</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('requests.col.target') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('requests.col.product') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('requests.col.initiator') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('requests.col.status') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('requests.col.urgency') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('requests.col.due_date') }}</th>
            <th class="py-2 px-3 font-medium text-[var(--ui-text-muted)]">{{ t('requests.col.created') }}</th>
            <th class="py-2 px-3"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="req in filteredRequests" :key="req.request_id" class="border-b border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)]">
            <td class="py-2 px-3 font-mono text-xs">{{ req.request_id }}</td>
            <td class="py-2 px-3">
              <NuxtLink :to="`/entities/${req.target_entity_id}`" class="hover:underline">{{ req.target_display_name }}</NuxtLink>
            </td>
            <td class="py-2 px-3">{{ req.requested_product }}</td>
            <td class="py-2 px-3 font-mono text-xs">{{ req.initiator }}</td>
            <td class="py-2 px-3">
              <UBadge :color="statusColor(req.status)" size="sm">{{ t(`requests.status.${req.status}`) }}</UBadge>
            </td>
            <td class="py-2 px-3">
              <UBadge :color="urgencyColor(req.urgency)" size="sm">{{ enumLabel(t, te, 'requests.urgency', req.urgency) }}</UBadge>
            </td>
            <td class="py-2 px-3 text-[var(--ui-text-muted)]">{{ req.due_date ? formatDate(req.due_date) : '—' }}</td>
            <td class="py-2 px-3 text-[var(--ui-text-muted)]">{{ formatDate(req.created_at) }}</td>
            <td class="py-2 px-3">
              <UButton size="xs" variant="outline" :to="`/entities/${req.target_entity_id}`">{{ t('common.view') }}</UButton>
            </td>
          </tr>
        </tbody>
      </table></div>
      <div v-if="nextCursor" class="flex justify-center pt-3">
        <UButton variant="outline" :loading="loadingMore" @click="loadMore">{{ t('entity.load_more') }}</UButton>
      </div>
    </UCard>

    <!-- New request dialog -->
    <UModal v-model:open="showNewDialog">
      <template #content>
        <UCard>
          <template #header>
            <h3 class="text-lg font-semibold">{{ t('requests.new') }}</h3>
          </template>
          <div class="space-y-4">
            <UFormField :label="t('requests.form.target')">
              <EntityAutocomplete
                v-model="newRequest.target_entity_id"
                :placeholder="t('entity.search_placeholder')"
              />
            </UFormField>
            <UFormField :label="t('requests.form.product')">
              <UInput v-model="newRequest.requested_product" class="w-full" />
            </UFormField>
            <UFormField :label="t('requests.form.urgency')">
              <USelect v-model="newRequest.urgency" :items="urgencyOptions" value-key="value" class="w-full" />
            </UFormField>
            <UFormField :label="t('requests.form.due_date')">
              <UInput v-model="newRequest.due_date" type="date" class="w-full" />
            </UFormField>
          </div>
          <template #footer>
            <div class="flex justify-end gap-2">
              <UButton variant="outline" @click="showNewDialog = false">{{ t('common.cancel') }}</UButton>
              <UButton
                :disabled="!canSubmitRequest"
                :loading="submitting"
                @click="submitNewRequest"
              >
                {{ t('common.submit') }}
              </UButton>
            </div>
          </template>
        </UCard>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import StatCard from '~/components/common/StatCard.vue';
import { nextCursorOf } from '~/utils/pagination';

definePageMeta({ middleware: ['auth', capability('entity.read')] });

const { t, te } = useI18n();
const toast = useToast();
const api = useApi();

interface RequestRow {
  request_id: string;
  target_entity_id: string;
  target_display_name: string;
  initiator: string;
  status: string;
  requested_product: string;
  requested_limit: number | null;
  urgency: string;
  due_date: string | null;
  created_at: string;
}

const requests = ref<RequestRow[]>([]);
const loading = ref(false);
const showNewDialog = ref(false);
const submitting = ref(false);

const filters = reactive({ status: 'all' as string });
const newRequest = reactive({
  target_entity_id: '',
  requested_product: '',
  urgency: 'normal',
  due_date: '',
});

const canSubmitRequest = computed(() => (
  newRequest.target_entity_id.trim().length > 0
  && newRequest.requested_product.trim().length > 0
));

const statusOptions = [
  { label: t('requests.filter.all_statuses'), value: 'all' },
  { label: t('requests.status.pending'), value: 'pending' },
  { label: t('requests.status.in_progress'), value: 'in_progress' },
  { label: t('requests.status.completed'), value: 'completed' },
];

const urgencyOptions = [
  { label: t('requests.urgency.normal'), value: 'normal' },
  { label: t('requests.urgency.high'), value: 'high' },
  { label: t('requests.urgency.urgent'), value: 'urgent' },
];

const stats = computed(() => ({
  pending: requests.value.filter((r) => r.status === 'pending').length,
  inProgress: requests.value.filter((r) => r.status === 'in_progress').length,
  completed: requests.value.filter((r) => r.status === 'completed').length,
  overdue: requests.value.filter((r) => r.due_date && new Date(r.due_date) < new Date() && r.status !== 'completed').length,
}));

const filteredRequests = computed(() => {
  if (filters.status === 'all') return requests.value;
  return requests.value.filter((r) => r.status === filters.status);
});

type BadgeColor = 'success' | 'info' | 'warning' | 'error' | 'neutral';

function statusColor(status: string): BadgeColor {
  switch (status) {
    case 'completed': return 'success';
    case 'in_progress': return 'info';
    case 'pending': return 'neutral';
    default: return 'neutral';
  }
}

function urgencyColor(urgency: string): BadgeColor {
  switch (urgency) {
    case 'urgent': return 'error';
    case 'high': return 'warning';
    default: return 'neutral';
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

function resetFilters() {
  filters.status = 'all';
  fetchRequests();
}

const nextCursor = ref<string | null>(null);
const loadingMore = ref(false);

function listQuery(cursor?: string) {
  return { status: filters.status === 'all' ? undefined : filters.status, cursor };
}

async function fetchRequests() {
  loading.value = true;
  try {
    const res = await api.requests.list(listQuery());
    requests.value = res.data as RequestRow[];
    nextCursor.value = nextCursorOf(res.meta);
  } catch (err) {
    toast.add({ title: t('common.error'), description: displayError(err, t, te), color: 'error' });
  } finally {
    loading.value = false;
  }
}

async function loadMore() {
  if (!nextCursor.value) return;
  loadingMore.value = true;
  try {
    const res = await api.requests.list(listQuery(nextCursor.value));
    requests.value.push(...(res.data as RequestRow[]));
    nextCursor.value = nextCursorOf(res.meta);
  } catch (err) {
    toast.add({ title: t('common.error'), description: displayError(err, t, te), color: 'error' });
  } finally {
    loadingMore.value = false;
  }
}

async function submitNewRequest() {
  if (!canSubmitRequest.value) return;
  submitting.value = true;
  try {
    await api.requests.create(newRequest);
    showNewDialog.value = false;
    newRequest.target_entity_id = '';
    newRequest.requested_product = '';
    newRequest.urgency = 'normal';
    newRequest.due_date = '';
    toast.add({ title: t('requests.new_success'), color: 'success' });
    await fetchRequests();
  } catch (err) {
    toast.add({ title: t('common.error'), description: displayError(err, t, te), color: 'error' });
  } finally {
    submitting.value = false;
  }
}

onMounted(fetchRequests);
</script>
