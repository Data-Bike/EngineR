<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('admin.permissions_title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('admin.permissions_desc') }}</p>
      </div>
      <UButton v-if="canMutate" icon="i-lucide-plus" data-testid="create-role" @click="openCreateDialog">
        {{ t('admin.create_permission') }}
      </UButton>
    </div>

    <div class="bg-[var(--ui-bg-muted)] border border-[var(--ui-border)] rounded-md p-4 text-sm">
      <p class="font-medium mb-1">{{ t('admin.permissions_info_title') }}</p>
      <p class="text-[var(--ui-text-muted)]">{{ t('admin.permissions_info_text') }}</p>
    </div>

    <!-- Role tree -->
    <div v-if="loading" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('common.loading') }}
    </div>
    <div v-else class="space-y-2">
      <UCard v-for="node in roleTree" :key="node.id" :style="{ marginInlineStart: `${node.depth * 1.5}rem` }">
        <div class="flex items-start justify-between">
          <div class="flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <UIcon v-if="node.depth > 0" name="i-lucide-corner-down-right" class="text-[var(--ui-text-muted)]" />
              <p class="font-medium">{{ node.name }}</p>
              <UBadge :color="node.class === 'data' ? 'info' : 'neutral'" variant="subtle" size="sm">
                {{ t(`admin.class_${node.class}`) }}
              </UBadge>
              <UBadge v-if="node.special" color="warning" variant="subtle" size="sm">{{ node.name }}</UBadge>
              <UBadge v-if="node.parentName" variant="outline" size="sm">
                {{ t('admin.inherits') }}: {{ node.parentName }}
              </UBadge>
              <UBadge variant="subtle" size="sm">{{ node.principals.length }} {{ t('admin.principals') }}</UBadge>
            </div>
            <p class="text-sm text-[var(--ui-text-muted)] mt-1">{{ node.description }}</p>

            <div class="mt-2">
              <div class="flex flex-wrap gap-1">
                <UBadge v-for="p in node.principals" :key="p" variant="soft" size="sm">{{ p }}</UBadge>
              </div>
            </div>

            <div class="mt-3 space-y-2">
              <div v-for="(rule, idx) in node.rules" :key="idx" class="border border-[var(--ui-border)] rounded p-2 text-sm">
                <div class="flex items-center gap-2 flex-wrap">
                  <UBadge :color="ruleColor(rule.effect)" variant="soft" size="sm">{{ rule.effect }}</UBadge>
                  <span class="text-[var(--ui-text-muted)]">
                    {{ t(`admin.res_${rule.resource_type === 'predicate' ? 'relation' : rule.resource_type}`, rule.resource_type) }}
                    <strong>{{ resourceLabel(rule) }}</strong>
                  </span>
                  <span class="text-[var(--ui-text-muted)]">{{ t('admin.action') }}: <strong>{{ rule.action }}</strong></span>
                </div>
                <p v-if="rule.mask" class="text-xs text-[var(--ui-text-muted)] mt-1">{{ t('admin.mask') }}: {{ rule.mask }}</p>
              </div>
            </div>
          </div>
          <div v-if="canMutate && !node.special" class="flex gap-2">
            <UButton size="sm" variant="outline" data-testid="edit-role" @click="editGroup(node)">
              {{ t('common.edit') }}
            </UButton>
            <UButton size="sm" variant="ghost" color="error" icon="i-lucide-trash" data-testid="delete-role" @click="deleteGroup(node)" />
          </div>
        </div>
      </UCard>
    </div>

    <!-- Create/Edit Dialog -->
    <UModal v-model:open="showDialog">
      <template #content>
        <div class="p-6 space-y-4 max-h-[80vh] overflow-auto">
          <h2 class="text-lg font-bold">{{ editing ? t('common.edit') : t('admin.create_permission') }}</h2>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <UFormField :label="t('admin.group_name')">
              <UInput v-model="formData.name" class="w-full" data-testid="role-name" />
            </UFormField>
            <UFormField :label="t('admin.group_class')">
              <USelect v-model="formData.class" :items="classOptions" class="w-full" data-testid="role-class" />
            </UFormField>
          </div>

          <UFormField :label="t('admin.group_description')">
            <UInput v-model="formData.description" class="w-full" />
          </UFormField>

          <UFormField :label="t('admin.parent_role')" :hint="t('admin.parent_role_hint')">
            <USelect v-model="formData.parent_id" :items="parentOptions" class="w-full" data-testid="role-parent" />
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
              <UButton size="xs" variant="outline" icon="i-lucide-plus" data-testid="add-rule" @click="addRule">
                {{ t('admin.add_rule') }}
              </UButton>
            </div>
            <div v-for="(rule, idx) in formData.rules" :key="idx" class="border border-[var(--ui-border)] rounded-md p-3 space-y-2">
              <div class="flex items-center justify-between">
                <p class="text-sm font-medium">{{ t('admin.rule_n', { n: idx + 1 }) }}</p>
                <UButton size="xs" variant="ghost" color="error" icon="i-lucide-x" @click="removeRule(idx)" />
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <UFormField :label="t('admin.effect')">
                  <USelect v-model="rule.effect" :items="effectOptions" class="w-full" />
                </UFormField>
                <UFormField :label="t('admin.resource_type')">
                  <USelect v-model="rule.resource_type" :items="resourceTypeOptions" class="w-full" />
                </UFormField>
              </div>

              <UFormField v-if="rule.resource_type === 'type' || rule.resource_type === 'field'" :label="t('admin.res_type')">
                <UInput v-model="rule.type_alias" class="w-full" placeholder="person" />
              </UFormField>
              <UFormField v-if="rule.resource_type === 'field'" :label="t('admin.field')">
                <UInput v-model="rule.field" class="w-full" placeholder="inn ('*' = все поля)" />
              </UFormField>
              <UFormField v-if="rule.resource_type === 'entity'" :label="t('admin.res_entity')">
                <UInput v-model="rule.entity" class="w-full" placeholder="public_id" />
              </UFormField>
              <div v-if="rule.resource_type === 'relation' || rule.resource_type === 'predicate'" class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <UFormField :label="t('admin.predicate')">
                  <UInput v-model="rule.predicate" class="w-full" placeholder="works_for ('*' = все)" />
                </UFormField>
                <UFormField :label="t('admin.direction')">
                  <USelect v-model="rule.direction" :items="ruleDirectionOptions" class="w-full" />
                </UFormField>
              </div>
              <UFormField v-if="rule.resource_type === 'relation_instance'" :label="t('admin.res_relation_instance')">
                <UInput v-model="rule.relation" class="w-full" placeholder="statement_id" />
              </UFormField>
              <UFormField v-if="rule.resource_type === 'url'" :label="t('admin.res_url')">
                <UInput v-model="rule.path" class="w-full" placeholder="/admin или /api/v1/..." />
              </UFormField>
              <UFormField v-if="rule.resource_type === 'capability'" :label="t('admin.capability')">
                <UInput v-model="rule.action" class="w-full" placeholder="entity.read" />
              </UFormField>

              <UFormField v-if="rule.resource_type !== 'capability'" :label="t('admin.action')">
                <USelect v-model="rule.action" :items="actionOptions" class="w-full" />
              </UFormField>
              <UFormField v-if="rule.resource_type === 'field'" :label="t('admin.mask')">
                <USelect v-model="rule.mask" :items="maskOptions" class="w-full" />
              </UFormField>
            </div>
          </div>

          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showDialog = false">
              {{ t('common.cancel') }}
            </UButton>
            <UButton :loading="saving" data-testid="save-role" @click="saveGroup">
              {{ t('common.save') }}
            </UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';

definePageMeta({
  middleware: ['auth', capability('admin.pages.access')],
});

const { t } = useI18n();
const toast = useToast();
const api = useApi();
const authStore = useAuthStore();

// Viewing the page needs admin.pages.access; mutating roles/rules needs
// security.manage — mirror the backend capability split.
const canMutate = computed(() => authStore.hasCapability('security.manage'));

interface PermissionRule {
  effect: string;
  resource_type: string;
  type_alias: string;
  predicate: string;
  direction: string;
  field: string;
  entity: string;
  relation: string;
  path: string;
  action: string;
  mask: string | null;
}

interface PermissionGroup {
  id: string;
  name: string;
  description: string;
  class: string;
  parent_id: string | null;
  special: boolean;
  principals: string[];
  rules: PermissionRule[];
}

interface TreeNode extends PermissionGroup {
  depth: number;
  parentName: string;
}

const emptyRule = (): PermissionRule => ({
  effect: 'allow', resource_type: 'type', type_alias: '', predicate: '',
  direction: 'both', field: '', entity: '', relation: '', path: '',
  action: 'read', mask: null as string | null,
});

const loading = ref(false);
const permissionGroups = ref<PermissionGroup[]>([]);

async function fetchPermissionGroups() {
  loading.value = true;
  try {
    const res = await api.security.listGroups();
    permissionGroups.value = (res.data ?? []).map((g: any) => ({
      id: g.id ?? '',
      name: g.name ?? '',
      description: g.description ?? '',
      class: g.class ?? 'capability',
      parent_id: g.parent_id ?? null,
      special: !!g.special,
      principals: g.principals ?? [],
      rules: (g.rules ?? []).map((r: any) => ({
        ...emptyRule(),
        effect: r.effect ?? 'allow',
        resource_type: r.resource_type ?? 'type',
        type_alias: r.type_alias ?? '',
        predicate: r.predicate ?? '',
        direction: r.direction ?? 'both',
        field: r.field ?? '',
        entity: r.entity ?? '',
        relation: r.relation ?? '',
        path: r.path ?? '',
        action: r.action ?? 'read',
        mask: r.mask ?? null,
      })),
    }));
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    loading.value = false;
  }
}

/** Flatten the role hierarchy parents-first with a depth marker. */
const roleTree = computed<TreeNode[]>(() => {
  const byId = new Map(permissionGroups.value.map((g) => [g.id, g]));
  const childrenOf = new Map<string, PermissionGroup[]>();
  const roots: PermissionGroup[] = [];
  for (const g of permissionGroups.value) {
    if (g.parent_id && byId.has(g.parent_id) && g.parent_id !== g.id) {
      const arr = childrenOf.get(g.parent_id) ?? [];
      arr.push(g);
      childrenOf.set(g.parent_id, arr);
    } else {
      roots.push(g);
    }
  }
  const out: TreeNode[] = [];
  const visit = (g: PermissionGroup, depth: number, seen: Set<string>) => {
    if (seen.has(g.id)) return;
    seen.add(g.id);
    out.push({ ...g, depth, parentName: byId.get(g.parent_id)?.name ?? '' });
    for (const c of childrenOf.get(g.id) ?? []) visit(c, depth + 1, seen);
  };
  for (const g of roots) visit(g, 0, new Set());
  for (const g of permissionGroups.value) {
    if (!out.some((n) => n.id === g.id)) out.push({ ...g, depth: 0, parentName: '' });
  }
  return out;
});

const showDialog = ref(false);
const editing = ref(false);
const saving = ref(false);
const principalInput = ref('');

const classOptions = computed(() => [
  { label: t('admin.class_capability'), value: 'capability' },
  { label: t('admin.class_data'), value: 'data' },
]);

const parentOptions = computed(() => [
  { label: t('admin.no_parent'), value: null },
  ...permissionGroups.value
    .filter((g) => g.id !== formData.value.id)
    .map((g) => ({ label: g.name, value: g.id })),
]);

const effectOptions = computed(() => [
  { label: t('admin.effect_allow'), value: 'allow' },
  { label: t('admin.effect_deny'), value: 'deny' },
]);

const resourceTypeOptions = computed(() => [
  { label: t('admin.res_all'), value: 'all' },
  { label: t('admin.res_type'), value: 'type' },
  { label: t('admin.res_field'), value: 'field' },
  { label: t('admin.res_entity'), value: 'entity' },
  { label: t('admin.res_relation'), value: 'relation' },
  { label: t('admin.res_relation_instance'), value: 'relation_instance' },
  { label: t('admin.res_url'), value: 'url' },
  { label: t('admin.res_capability'), value: 'capability' },
]);

const ruleDirectionOptions = computed(() => [
  { label: t('admin.dir_both'), value: 'both' },
  { label: t('admin.dir_outgoing'), value: 'outgoing' },
  { label: t('admin.dir_incoming'), value: 'incoming' },
]);

const actionOptions = computed(() => [
  { label: t('admin.act_read'), value: 'read' },
  { label: t('admin.act_create'), value: 'create' },
  { label: t('admin.act_update'), value: 'update' },
  { label: t('admin.act_delete'), value: 'delete' },
  { label: t('admin.act_all'), value: '*' },
]);

const maskOptions = computed(() => [
  { label: t('admin.mask_allow'), value: null },
  { label: t('admin.mask_omit'), value: 'omit' },
  { label: t('admin.mask_partial'), value: 'partial-mask' },
  { label: t('admin.mask_null'), value: 'null-mask' },
  { label: t('admin.mask_tokenize'), value: 'tokenize' },
]);

const formData = ref<PermissionGroup>({
  id: '', name: '', description: '', class: 'data', parent_id: null,
  special: false, principals: [], rules: [],
});

type BadgeColor = 'success' | 'error';

function ruleColor(effect: string): BadgeColor {
  return effect === 'allow' ? 'success' : 'error';
}

function resourceLabel(rule: PermissionRule): string {
  switch (rule.resource_type) {
    case 'type': return rule.type_alias || '*';
    case 'field': return `${rule.type_alias || '*'}.${rule.field || '*'}`;
    case 'entity': return rule.entity;
    case 'relation': return rule.predicate || '*';
    case 'relation_instance': return rule.relation;
    case 'url': return rule.path || '/';
    default: return '';
  }
}

function openCreateDialog() {
  editing.value = false;
  formData.value = {
    id: '', name: '', description: '', class: 'data', parent_id: null,
    special: false, principals: [], rules: [emptyRule()],
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
  formData.value.rules.push(emptyRule());
}

function removeRule(idx: number) {
  formData.value.rules.splice(idx, 1);
}

async function saveGroup() {
  saving.value = true;
  try {
    const payload = {
      name: formData.value.name,
      description: formData.value.description,
      class: formData.value.class,
      parent_id: formData.value.parent_id || null,
      principals: formData.value.principals,
      rules: formData.value.rules,
    };
    if (editing.value) {
      await api.security.updateGroup(formData.value.id, payload);
    } else {
      await api.security.createGroup(payload);
    }
    showDialog.value = false;
    toast.add({ title: t('admin.permission_saved'), color: 'success' });
    await fetchPermissionGroups();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    saving.value = false;
  }
}

async function deleteGroup(group: PermissionGroup) {
  try {
    await api.security.deleteGroup(group.id);
    toast.add({ title: t('admin.permission_deleted'), color: 'success' });
    await fetchPermissionGroups();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  }
}

onMounted(fetchPermissionGroups);
</script>
