<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h3 class="font-medium">{{ t('entity.tabs.tasks') }}</h3>
      <UButton size="sm" icon="i-lucide-plus" @click="showCreate = true">
        {{ t('entity.tasks.create') }}
      </UButton>
    </div>

    <!-- Task list -->
    <div v-if="tasks.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>
    <div v-else class="space-y-2">
      <div
        v-for="task in sortedTasks"
        :key="task.task_id"
        class="rounded-md border border-[var(--ui-border)] p-3 hover:border-[var(--ui-border-muted)]"
      >
        <div class="flex items-start justify-between">
          <div class="flex items-start gap-3 flex-1">
            <!-- Status icon -->
            <UIcon :name="taskStatusIcon(task.status)" class="text-xl shrink-0" :class="taskStatusClass(task.status)" />
            <div class="flex-1">
              <div class="flex items-center gap-2">
                <p class="font-medium">{{ task.title }}</p>
                <UBadge v-if="task.blocking" color="error" size="sm">{{ t('entity.tasks.blocking') }}</UBadge>
              </div>
              <p v-if="task.description" class="text-sm text-[var(--ui-text-muted)] mt-1">{{ task.description }}</p>
              <!-- Meta -->
              <div class="flex flex-wrap items-center gap-3 mt-2 text-xs text-[var(--ui-text-muted)]">
                <UBadge variant="subtle" size="sm">{{ enumLabel(t, te, 'entity.tasks.kind', task.kind) }}</UBadge>
                <UBadge :color="taskStatusColor(task.status)" size="sm">
                  {{ enumLabel(t, te, 'entity.tasks.status', task.status) }}
                </UBadge>
                <span v-if="task.assignee">{{ t('entity.tasks.assignee') }}: {{ task.assignee }}</span>
                <span v-if="task.due_date" :class="{ 'text-red-500': isOverdue(task.due_date) }">
                  {{ t('entity.tasks.due_date') }}: {{ formatDateTime(task.due_date) }}
                </span>
                <span>{{ t('entity.tasks.created') }}: {{ formatDateTime(task.created_at) }}</span>
              </div>
              <!-- Sub-tasks -->
              <div v-if="task.sub_tasks && task.sub_tasks.length > 0" class="mt-2 text-xs text-[var(--ui-text-muted)]">
                {{ t('entity.tasks.sub_tasks') }}: {{ task.sub_tasks.length }}
              </div>
            </div>
          </div>
          <!-- Actions -->
          <div class="flex gap-1">
            <UButton
              v-if="task.status === 'pending' || task.status === 'assigned' || task.status === 'overdue'"
              size="xs"
              variant="outline"
              @click="startTask(task.task_id)"
            >
              {{ t('entity.tasks.start') }}
            </UButton>
            <UButton
              v-if="task.status === 'in_progress'"
              size="xs"
              variant="outline"
              @click="completeTask(task.task_id)"
            >
              {{ t('entity.tasks.complete') }}
            </UButton>
            <UButton
              v-if="canCancel(task.status)"
              size="xs"
              variant="ghost"
              color="error"
              @click="cancelTask(task.task_id)"
            >
              {{ t('common.cancel') }}
            </UButton>
          </div>
        </div>
      </div>
    </div>

    <!-- Create task dialog -->
    <UModal v-model:open="showCreate">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.tasks.create') }}</h2>
          <UFormField :label="t('entity.tasks.title')">
            <UInput v-model="newTitle" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.tasks.kind_label')">
            <USelect v-model="newKind" :items="kindOptions" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.tasks.due_date')">
            <UInput v-model="newDueDate" type="date" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.tasks.description')">
            <UTextarea v-model="newDescription" :rows="3" class="w-full" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showCreate = false">{{ t('common.cancel') }}</UButton>
            <UButton @click="createTask" :loading="creating">{{ t('common.save') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { WorkflowTask, TaskStatus, TaskKind } from '@enginer/api-client';

const props = defineProps<{
  entityId?: string;
  requestId?: string;
}>();

const { t, te } = useI18n();
const toast = useToast();
const api = useApi();

const tasks = ref<WorkflowTask[]>([]);
const showCreate = ref(false);
const creating = ref(false);
const newTitle = ref('');
const newKind = ref<TaskKind>('check_execution');
const newDueDate = ref('');
const newDescription = ref('');

const sortedTasks = computed(() =>
  [...tasks.value].sort((a, b) => a.priority - b.priority || (a.created_at ?? '').localeCompare(b.created_at ?? '')),
);

const kindOptions = computed(() => [
  { label: t('entity.tasks.kind.request_processing'), value: 'request_processing' },
  { label: t('entity.tasks.kind.check_execution'), value: 'check_execution' },
  { label: t('entity.tasks.kind.risk_assessment'), value: 'risk_assessment' },
  { label: t('entity.tasks.kind.monitoring_review'), value: 'monitoring_review' },
  { label: t('entity.tasks.kind.report_generation'), value: 'report_generation' },
  { label: t('entity.tasks.kind.approval'), value: 'approval' },
  { label: t('entity.tasks.kind.notification'), value: 'notification' },
]);

function taskStatusIcon(status: TaskStatus): string {
  const icons: Record<TaskStatus, string> = {
    pending: 'i-lucide-circle',
    assigned: 'i-lucide-user-check',
    in_progress: 'i-lucide-play-circle',
    completed: 'i-lucide-check-circle',
    cancelled: 'i-lucide-x-circle',
    overdue: 'i-lucide-clock-alert',
    escalated: 'i-lucide-alert-octagon',
  };
  return icons[status] ?? 'i-lucide-circle';
}

function taskStatusClass(status: TaskStatus): string {
  const classes: Record<TaskStatus, string> = {
    pending: 'text-[var(--ui-text-muted)]',
    assigned: 'text-blue-500',
    in_progress: 'text-blue-500',
    completed: 'text-green-500',
    cancelled: 'text-[var(--ui-text-muted)]',
    overdue: 'text-red-500',
    escalated: 'text-orange-500',
  };
  return classes[status] ?? '';
}

type BadgeColor = 'primary' | 'secondary' | 'success' | 'info' | 'warning' | 'error' | 'neutral';

function taskStatusColor(status: TaskStatus): BadgeColor {
  const colors: Record<TaskStatus, BadgeColor> = {
    pending: 'neutral',
    assigned: 'info',
    in_progress: 'primary',
    completed: 'success',
    cancelled: 'neutral',
    overdue: 'error',
    escalated: 'warning',
  };
  return colors[status] ?? 'neutral';
}

function isOverdue(dueDate: string): boolean {
  return new Date(dueDate) < new Date();
}

function canCancel(status: TaskStatus): boolean {
  return ['pending', 'assigned', 'in_progress', 'overdue', 'escalated'].includes(status);
}

async function fetchTasks() {
  if (!props.entityId) {
    tasks.value = [];
    return;
  }
  try {
    const res = await api.checks.getEntityTasks(props.entityId);
    tasks.value = (res.data ?? []).map((task) => ({
      ...task,
      description: task.description ?? undefined,
      assignee: task.assignee ?? undefined,
      due_date: task.due_date ?? undefined,
    }));
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
    tasks.value = [];
  }
}

async function startTask(taskId: string) {
  try {
    await api.checks.startTask(taskId);
    toast.add({ title: t('entity.tasks.start'), color: 'success' });
    await fetchTasks();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

async function completeTask(taskId: string) {
  try {
    await api.checks.completeTask(taskId, { result: {} });
    toast.add({ title: t('entity.tasks.complete'), color: 'success' });
    await fetchTasks();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

async function cancelTask(taskId: string) {
  try {
    await api.checks.cancelTask(taskId, { reason: 'cancelled' });
    toast.add({ title: t('common.cancel'), color: 'success' });
    await fetchTasks();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

async function createTask() {
  if (!props.entityId) return;
  creating.value = true;
  try {
    await api.checks.createEntityTask(props.entityId, {
      title: newTitle.value,
      kind: newKind.value,
      due_date: newDueDate.value || undefined,
      description: newDescription.value || undefined,
    });
    toast.add({ title: t('entity.tasks.create'), color: 'success' });
    showCreate.value = false;
    newTitle.value = '';
    newDescription.value = '';
    newDueDate.value = '';
    await fetchTasks();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    creating.value = false;
  }
}

onMounted(fetchTasks);
</script>
