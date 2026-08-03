<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('admin.permissions_title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('admin.permissions_desc') }}</p>
      </div>
      <UButton icon="i-lucide-plus" @click="openCreateDialog">
        {{ t('admin.create_permission') }}
      </UButton>
    </div>

    <!-- Info banner -->
    <div class="bg-[var(--ui-bg-muted)] border border-[var(--ui-border)] rounded-md p-4 text-sm">
      <p class="font-medium mb-1">{{ t('admin.permissions_info_title') }}</p>
      <p class="text-[var(--ui-text-muted)]">{{ t('admin.permissions_info_text') }}</p>
    </div>

    <!-- Permission groups -->
    <div v-if="loading" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>
    <div v-else class="space-y-3">
      <UCard v-for="group in permissionGroups" :key="group.id">
        <div class="flex items-start justify-between">
          <div class="flex-1">
            <div class="flex items-center gap-2">
              <p class="font-medium">{{ group.name }}</p>
              <UBadge variant="subtle" size="sm">{{ group.principals.length }} {{ t('admin.principals') }}</UBadge>
            </div>
            <p class="text-sm text-[var(--ui-text-muted)] mt-1">{{ group.description }}</p>

            <!-- Principals -->
            <div class="mt-2">
              <p class="text-xs font-medium text-[var(--ui-text-muted)] mb-1">{{ t('admin.principals') }}:</p>
              <div class="flex flex-wrap gap-1">
                <UBadge v-for="p in group.principals" :key="p" variant="soft" size="sm">{{ p }}</UBadge>
              </div>
            </div>

            <!-- Rules -->
            <div class="mt-3 space-y-2">
              <p class="text-xs font-medium text-[var(--ui-text-muted)]">{{ t('admin.rules') }}:</p>
              <div v-for="(rule, idx) in group.rules" :key="idx" class="border border-[var(--ui-border)] rounded p-2 text-sm">
                <div class="flex items-center gap-2 flex-wrap">
                  <UBadge :color="ruleColor(rule.effect)" variant="soft" size="sm">{{ rule.effect }}</UBadge>
                  <span v-if="rule.resource_type" class="text-[var(--ui-text-muted)]">{{ t('admin.resource_type') }}: <strong>{{ rule.resource_type }}</strong></span>
                  <span v-if="rule.predicate" class="text-[var(--ui-text-muted)]">{{ t('admin.predicate') }}: <strong>{{ rule.predicate }}</strong></span>
                  <span v-if="rule.direction" class="text-[var(--ui-text-muted)]">{{ t('admin.direction') }}: <strong>{{ rule.direction }}</strong></span>
                  <span v-if="rule.field" class="text-[var(--ui-text-muted)]">{{ t('admin.field') }}: <strong>{{ rule.field }}</strong></span>
                  <span v-if="rule.action" class="text-[var(--ui-text-muted)]">{{ t('admin.action') }}: <strong>{{ rule.action }}</strong></span>
                </div>
                <p v-if="rule.mask" class="text-xs text-[var(--ui-text-muted)] mt-1">{{ t('admin.mask') }}: {{ rule.mask }}</p>
              </div>
            </div>
          </div>
          <div class="flex gap-2">
            <UButton size="sm" variant="outline" @click="editGroup(group)">
              {{ t('common.edit') }}
            </UButton>
            <UButton size="sm" variant="ghost" color="error" icon="i-lucide-trash" @click="deleteGroup(group)" />
          </div>
        </div>
      </UCard>
    </div>

    <!-- Create/Edit Dialog -->
    <UModal v-model:open="showDialog">
      <template #content>
        <div class="p-6 space-y-4 max-h-[80vh] overflow-auto">
          <h2 class="text-lg font-bold">{{ editing ? t('common.edit') : t('admin.create_permission') }}</h2>

          <UFormField :label="t('admin.group_name')">
            <UInput v-model="formData.name" class="w-full" />
          </UFormField>

          <UFormField :label="t('admin.group_description')">
            <UInput v-model="formData.description" class="w-full" />
          </UFormField>

          <UFormField :label="t('admin.principals')" :hint="t('admin.principals_hint')">
            <UInput v-model="principalInput" class="w-full" :placeholder="t('admin.principals_placeholder')" @keyup.enter="addPrincipal" />
            <UButton size="xs" variant="outline" class="mt-1" @click="addPrincipal">{{ t('admin.add_principal') }}</UButton>
            <div class="flex flex-wrap gap-1 mt-2">
              <UBadge v-for="(p, idx) in formData.principals" :key="idx" variant="soft" size="sm" class="cursor-pointer" @click="removePrincipal(idx)">
                {{ p }} ×
              </UBadge>
            </div>
          </UFormField>

          <!-- Rules -->
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <p class="font-medium">{{ t('admin.rules') }}</p>
              <UButton size="xs" variant="outline" icon="i-lucide-plus" @click="addRule">
                {{ t('admin.add_rule') }}
              </UButton>
            </div>
            <div v-for="(rule, idx) in formData.rules" :key="idx" class="border border-[var(--ui-border)] rounded-md p-3 space-y-2">
              <div class="flex items-center justify-between">
                <p class="text-sm font-medium">{{ t('admin.rule_n', { n: idx + 1 }) }}</p>
                <UButton size="xs" variant="ghost" color="error" icon="i-lucide-x" @click="removeRule(idx)" />
              </div>
              <UFormField :label="t('admin.effect')">
                <USelect v-model="rule.effect" :items="effectOptions" class="w-full" />
              </UFormField>
              <UFormField :label="t('admin.resource_type')">
                <USelect v-model="rule.resource_type" :items="resourceTypeOptions" class="w-full" />
              </UFormField>
              <div v-if="rule.resource_type === 'predicate'" class="grid grid-cols-2 gap-2">
                <UFormField :label="t('admin.predicate')">
                  <UInput v-model="rule.predicate" class="w-full" placeholder="managed_by" />
                </UFormField>
                <UFormField :label="t('admin.direction')">
                  <USelect v-model="rule.direction" :items="ruleDirectionOptions" class="w-full" />
                </UFormField>
              </div>
              <div v-if="rule.resource_type === 'field'" class="grid grid-cols-2 gap-2">
                <UFormField :label="t('admin.field')">
                  <UInput v-model="rule.field" class="w-full" placeholder="inn" />
                </UFormField>
                <UFormField :label="t('admin.mask')">
                  <USelect v-model="rule.mask" :items="maskOptions" class="w-full" />
                </UFormField>
              </div>
              <UFormField :label="t('admin.action')">
                <USelect v-model="rule.action" :items="actionOptions" class="w-full" />
              </UFormField>
            </div>
          </div>

          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showDialog = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton @click="saveGroup" :loading="saving">
              {{ t('common.save') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

definePageMeta({
  middleware: ['auth'],
});

const { t } = useI18n();
const { requireCapability } = useCapability();
requireCapability('admin.view');
const toast = useToast();

interface PermissionRule {
  effect: string;
  resource_type: string;
  predicate: string;
  direction: string;
  field: string;
  action: string;
  mask: string;
}

interface PermissionGroup {
  id: string;
  name: string;
  description: string;
  principals: string[];
  rules: PermissionRule[];
}

const loading = ref(false);
const permissionGroups = ref<PermissionGroup[]>([
  {
    id: 'pg-001',
    name: t('admin.default_compliance_officers', 'Риск-менеджеры'),
    description: t('admin.default_compliance_desc', 'Доступ к проверкам, рискам и решениям'),
    principals: ['risk_analyst', 'compliance_officer'],
    rules: [
      { effect: 'allow', resource_type: 'type', predicate: '', direction: '', field: '', action: 'read', mask: '' },
      { effect: 'allow', resource_type: 'predicate', predicate: '*', direction: 'both', field: '', action: 'read', mask: '' },
      { effect: 'allow', resource_type: 'type', predicate: '', direction: '', field: '', action: 'create', mask: '' },
      { effect: 'allow', resource_type: 'field', predicate: '', direction: '', field: 'inn', action: 'read', mask: 'allow' },
    ],
  },
  {
    id: 'pg-002',
    name: t('admin.default_managers', 'Менеджеры'),
    description: t('admin.default_managers_desc', 'Создание заявок и просмотр объектов'),
    principals: ['manager'],
    rules: [
      { effect: 'allow', resource_type: 'type', predicate: '', direction: '', field: '', action: 'read', mask: '' },
      { effect: 'allow', resource_type: 'type', predicate: '', direction: '', field: '', action: 'create', mask: '' },
      { effect: 'deny', resource_type: 'field', predicate: '', direction: '', field: 'passport', action: 'read', mask: 'omit' },
    ],
  },
]);

const showDialog = ref(false);
const editing = ref(false);
const saving = ref(false);
const principalInput = ref('');

const effectOptions = [
  { label: 'Allow', value: 'allow' },
  { label: 'Deny', value: 'deny' },
];

const resourceTypeOptions = [
  { label: t('admin.resource_type', 'Тип ресурса'), value: '' },
  { label: t('admin.res_type', 'Тип объекта'), value: 'type' },
  { label: t('admin.res_predicate', 'Связь (предикат)'), value: 'predicate' },
  { label: t('admin.res_field', 'Поле'), value: 'field' },
  { label: t('admin.res_entity', 'Конкретный объект'), value: 'entity' },
];

const ruleDirectionOptions = [
  { label: t('admin.dir_both', 'Оба направления'), value: 'both' },
  { label: t('admin.dir_outgoing', 'Исходящие'), value: 'outgoing' },
  { label: t('admin.dir_incoming', 'Входящие'), value: 'incoming' },
];

const actionOptions = [
  { label: t('admin.act_read', 'Чтение'), value: 'read' },
  { label: t('admin.act_create', 'Создание'), value: 'create' },
  { label: t('admin.act_update', 'Изменение'), value: 'update' },
  { label: t('admin.act_delete', 'Удаление'), value: 'delete' },
];

const maskOptions = [
  { label: t('admin.mask_allow', 'Показать полностью'), value: 'allow' },
  { label: t('admin.mask_omit', 'Скрыть'), value: 'omit' },
  { label: t('admin.mask_partial', 'Частично скрыть'), value: 'partial-mask' },
  { label: t('admin.mask_null', 'Заменить на null'), value: 'null-mask' },
  { label: t('admin.mask_tokenize', 'Токенизировать'), value: 'tokenize' },
];

const formData = ref<PermissionGroup>({
  id: '',
  name: '',
  description: '',
  principals: [],
  rules: [],
});

function ruleColor(effect: string): string {
  return effect === 'allow' ? 'success' : 'error';
}

function openCreateDialog() {
  editing.value = false;
  formData.value = {
    id: '',
    name: '',
    description: '',
    principals: [],
    rules: [{ effect: 'allow', resource_type: 'type', predicate: '', direction: '', field: '', action: 'read', mask: '' }],
  };
  showDialog.value = true;
}

function editGroup(group: PermissionGroup) {
  editing.value = true;
  formData.value = JSON.parse(JSON.stringify(group));
  showDialog.value = true;
}

function addPrincipal() {
  if (principalInput.value.trim()) {
    formData.value.principals.push(principalInput.value.trim());
    principalInput.value = '';
  }
}

function removePrincipal(idx: number) {
  formData.value.principals.splice(idx, 1);
}

function addRule() {
  formData.value.rules.push({ effect: 'allow', resource_type: 'type', predicate: '', direction: '', field: '', action: 'read', mask: '' });
}

function removeRule(idx: number) {
  formData.value.rules.splice(idx, 1);
}

async function saveGroup() {
  saving.value = true;
  try {
    if (editing.value) {
      const idx = permissionGroups.value.findIndex(g => g.id === formData.value.id);
      if (idx >= 0) permissionGroups.value[idx] = { ...formData.value };
    } else {
      formData.value.id = `pg-${Date.now()}`;
      permissionGroups.value.push({ ...formData.value });
    }
    showDialog.value = false;
    toast.add({ title: t('admin.permission_saved'), color: 'success' });
  } finally {
    saving.value = false;
  }
}

async function deleteGroup(group: PermissionGroup) {
  const idx = permissionGroups.value.findIndex(g => g.id === group.id);
  if (idx >= 0) {
    permissionGroups.value.splice(idx, 1);
    toast.add({ title: t('admin.permission_deleted'), color: 'success' });
  }
}
</script>
