<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('users.title') }}</h1>
        <p class="text-[var(--ui-text-muted)] mt-1">{{ t('users.desc') }}</p>
      </div>
      <UButton v-if="canCreate" icon="i-lucide-user-plus" data-testid="create-user" @click="openCreate">
        {{ t('users.create') }}
      </UButton>
    </div>

    <!-- Filters -->
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
      <UFormField :label="t('users.filter_text')">
        <UInput v-model="filters.q" icon="i-lucide-search" class="w-full" data-testid="filter-q"
                :placeholder="t('users.filter_text_hint')" @keyup.enter="load" />
      </UFormField>
      <UFormField :label="t('users.filter_role')">
        <USelect v-model="filters.group_id" :items="roleOptions" class="w-full" data-testid="filter-role" />
      </UFormField>
      <UFormField :label="t('users.filter_ip')">
        <UInput v-model="filters.ip" class="w-full" placeholder="192.168." @keyup.enter="load" />
      </UFormField>
      <UFormField :label="t('users.filter_status')">
        <USelect v-model="filters.status" :items="statusOptions" class="w-full" />
      </UFormField>
    </div>
    <div class="flex items-center gap-4">
      <UCheckbox v-model="filters.inherited" data-testid="filter-inherited"
                 :label="t('users.inherited_roles')" />
      <UButton icon="i-lucide-filter" size="sm" data-testid="apply-filters" @click="load">
        {{ t('users.apply') }}
      </UButton>
    </div>

    <!-- User table -->
    <div v-if="loading" class="text-center py-8 text-[var(--ui-text-muted)]">{{ t('common.loading') }}</div>
    <div v-else class="border border-[var(--ui-border)] rounded-md overflow-x-auto">
      <table class="w-full text-sm" data-testid="users-table">
        <thead class="bg-[var(--ui-bg-muted)]">
          <tr>
            <th class="text-start p-3 font-medium">{{ t('users.col_name') }}</th>
            <th class="text-start p-3 font-medium">{{ t('users.col_login') }}</th>
            <th class="text-start p-3 font-medium">{{ t('users.col_email') }}</th>
            <th class="text-start p-3 font-medium">{{ t('users.col_roles') }}</th>
            <th class="text-start p-3 font-medium">{{ t('users.col_status') }}</th>
            <th class="text-start p-3 font-medium">{{ t('users.col_last_seen') }}</th>
            <th class="text-start p-3 font-medium">{{ t('users.col_ip') }}</th>
            <th class="p-3"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="u in users" :key="u.principal_id"
              class="border-t border-[var(--ui-border)] hover:bg-[var(--ui-bg-muted)] cursor-pointer"
              tabindex="0"
              role="button"
              @click="openDetail(u.principal_id)"
              @keydown.enter.prevent="openDetail(u.principal_id)"
              @keydown.space.prevent="openDetail(u.principal_id)">
            <td class="p-3 font-medium">{{ u.display_name }}</td>
            <td class="p-3">{{ u.login }}</td>
            <td class="p-3 text-[var(--ui-text-muted)]">{{ u.email ?? '—' }}</td>
            <td class="p-3">
              <UBadge v-for="r in u.roles" :key="r" variant="soft" size="sm" class="me-1">{{ r }}</UBadge>
            </td>
            <td class="p-3">
              <UBadge :color="u.status === 'active' ? 'success' : 'error'" variant="subtle" size="sm">
                {{ u.status }}
              </UBadge>
            </td>
            <td class="p-3 text-[var(--ui-text-muted)]">{{ fmtDate(u.last_seen_at) }}</td>
            <td class="p-3 text-[var(--ui-text-muted)]">{{ u.last_ip ?? '—' }}</td>
            <td class="p-3" @click.stop>
              <UButton v-if="canDelete" size="xs" variant="ghost" color="error" icon="i-lucide-user-x"
                       :title="t('users.disable')" @click="disableUser(u)" />
            </td>
          </tr>
          <tr v-if="!users.length">
            <td colspan="8" class="p-6 text-center text-[var(--ui-text-muted)]">{{ t('users.empty') }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Create user dialog -->
    <UModal v-model:open="showCreate">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('users.create') }}</h2>
          <UFormField :label="t('users.col_name')">
            <UInput v-model="createForm.display_name" class="w-full" data-testid="user-name" />
          </UFormField>
          <UFormField :label="t('users.col_login')">
            <UInput v-model="createForm.login" class="w-full" data-testid="user-login" />
          </UFormField>
          <UFormField :label="t('users.col_email')">
            <UInput v-model="createForm.email" type="email" class="w-full" data-testid="user-email" />
          </UFormField>
          <UCheckbox v-model="createForm.is_service_account" :label="t('users.service_account')"
                     data-testid="user-service" />
          <UFormField :label="t('users.password')"
                      :hint="createForm.is_service_account ? t('users.password_optional') : ''">
            <UInput v-model="createForm.password" type="password" class="w-full" data-testid="user-password" />
          </UFormField>
          <UFormField :label="t('users.col_roles')">
            <USelectMenu v-model="createForm.groups" :items="roleSelectItems" multiple value-key="value"
                         class="w-full" data-testid="user-roles" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showCreate = false">{{ t('common.cancel') }}</UButton>
            <UButton :loading="saving" data-testid="save-user" @click="saveUser">{{ t('common.save') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>

    <!-- Detail slide-over -->
    <USlideover v-model:open="showDetail" :title="detail?.display_name ?? ''" class="max-w-2xl w-full">
      <template #body>
        <div v-if="detail" class="space-y-6">
          <div class="grid grid-cols-2 gap-3 text-sm">
            <div><p class="text-[var(--ui-text-muted)]">{{ t('users.col_login') }}</p><p class="font-medium">{{ detail.login }}</p></div>
            <div><p class="text-[var(--ui-text-muted)]">{{ t('users.col_email') }}</p><p class="font-medium">{{ detail.email ?? '—' }}</p></div>
            <div><p class="text-[var(--ui-text-muted)]">{{ t('users.col_status') }}</p>
              <UBadge :color="detail.status === 'active' ? 'success' : 'error'" variant="subtle" size="sm">{{ detail.status }}</UBadge></div>
            <div><p class="text-[var(--ui-text-muted)]">{{ t('users.col_last_seen') }}</p><p>{{ fmtDate(detail.last_seen_at) }}</p></div>
            <div><p class="text-[var(--ui-text-muted)]">{{ t('users.created_at') }}</p><p>{{ fmtDate(detail.created_at) }}</p></div>
          </div>

          <div>
            <p class="font-medium mb-1">{{ t('users.col_roles') }} ({{ t('users.direct') }})</p>
            <div class="flex flex-wrap gap-1">
              <UBadge v-for="r in detail.roles" :key="r.id" variant="soft" size="sm">{{ r.name }}</UBadge>
            </div>
            <p class="font-medium mb-1 mt-3">{{ t('users.effective_roles') }}</p>
            <div class="flex flex-wrap gap-1">
              <UBadge v-for="r in detail.effective_roles" :key="r" variant="outline" size="sm">{{ r }}</UBadge>
            </div>
          </div>

          <!-- Edit (user.edit / users.manage) -->
          <div v-if="canEdit" class="space-y-3 border-t border-[var(--ui-border)] pt-4">
            <p class="font-medium">{{ t('users.edit') }}</p>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <UFormField :label="t('users.col_name')">
                <UInput v-model="editForm.display_name" class="w-full" data-testid="edit-name" />
              </UFormField>
              <UFormField :label="t('users.col_email')">
                <UInput v-model="editForm.email" type="email" class="w-full" data-testid="edit-email" />
              </UFormField>
              <UFormField :label="t('users.col_status')">
                <USelect v-model="editForm.status" :items="statusEditOptions" class="w-full" data-testid="edit-status" />
              </UFormField>
              <UFormField :label="t('users.new_password')" :hint="t('users.new_password_hint')">
                <UInput v-model="editForm.password" type="password" class="w-full" data-testid="edit-password" />
              </UFormField>
            </div>
            <UFormField :label="t('users.col_roles')">
              <USelectMenu v-model="editForm.groups" :items="roleSelectItems" multiple value-key="value"
                           class="w-full" data-testid="edit-roles" />
            </UFormField>
            <div class="flex justify-end">
              <UButton :loading="saving" data-testid="save-edit" @click="saveEdit">{{ t('common.save') }}</UButton>
            </div>
          </div>

          <div>
            <p class="font-medium mb-1">{{ t('users.sessions') }}</p>
            <div class="space-y-1 text-sm max-h-40 overflow-auto">
              <div v-for="s in detail.sessions" :key="s.session_id"
                   class="flex justify-between border border-[var(--ui-border)] rounded px-2 py-1">
                <span>{{ s.ip_address ?? '—' }}</span>
                <span class="text-[var(--ui-text-muted)]">{{ fmtDate(s.started_at) }}</span>
                <UBadge :color="s.ended_at ? 'neutral' : 'success'" variant="subtle" size="sm">
                  {{ s.ended_at ? t('users.session_ended') : t('users.session_active') }}
                </UBadge>
              </div>
            </div>
          </div>

          <!-- API tokens (user.edit / users.manage) -->
          <div v-if="canEdit" class="space-y-2">
            <div class="flex items-center justify-between">
              <p class="font-medium">{{ t('users.api_tokens') }}</p>
              <UButton size="xs" variant="outline" data-testid="refresh-tokens" @click="loadTokens">
                {{ t('common.refresh') }}
              </UButton>
            </div>
            <div class="space-y-1 text-sm">
              <div v-for="tok in tokens" :key="tok.token_id"
                   class="flex items-center justify-between gap-2 border border-[var(--ui-border)] rounded px-2 py-1">
                <span class="truncate font-medium">{{ tok.name }}</span>
                <span class="text-[var(--ui-text-muted)] text-xs">
                  {{ t('users.token_last_used') }}: {{ fmtDate(tok.last_used_at) }}
                  · {{ t('users.token_expires') }}: {{ tok.expires_at ? fmtDate(tok.expires_at) : '∞' }}
                </span>
                <UBadge v-if="tok.revoked_at" color="neutral" variant="subtle" size="sm">
                  {{ t('users.token_revoked') }}
                </UBadge>
                <UButton v-else size="xs" color="error" variant="ghost"
                         :data-testid="`revoke-token-${tok.token_id}`"
                         @click="revokeToken(tok.token_id)">
                  {{ t('users.token_revoke') }}
                </UButton>
              </div>
              <p v-if="!tokens.length" class="text-[var(--ui-text-muted)]">
                {{ t('users.tokens_empty') }}
              </p>
            </div>
            <div class="flex items-center gap-2">
              <UInput v-model="tokenForm.name" size="sm" class="flex-1"
                      :placeholder="t('users.token_name')" data-testid="token-name" />
              <UInput v-model="tokenForm.days" size="sm" type="number" min="1" class="w-28"
                      :placeholder="t('users.token_days')" data-testid="token-days" />
              <UButton size="sm" :loading="tokenSaving" data-testid="create-token" @click="createToken">
                {{ t('users.token_create') }}
              </UButton>
            </div>
            <UAlert v-if="newToken" color="warning" variant="subtle" icon="i-lucide-key"
                    :title="t('users.token_created_once')"
                    :description="newToken" data-testid="new-token" />
          </div>

          <!-- Activity -->
          <div>
            <div class="flex items-center gap-2 mb-2">
              <p class="font-medium">{{ t('users.activity') }}</p>
              <UInput v-model="activityQuery" size="sm" icon="i-lucide-search" class="flex-1"
                      :placeholder="t('users.activity_search')" data-testid="activity-search"
                      @keyup.enter="loadActivity" />
              <UButton size="xs" variant="outline" @click="loadActivity">{{ t('users.apply') }}</UButton>
            </div>
            <UTabs v-model="activityTab" :items="activityTabs" />
            <div class="mt-2 space-y-1 max-h-72 overflow-auto text-sm">
              <div v-for="(row, i) in activityRows" :key="i"
                   class="flex justify-between gap-2 border border-[var(--ui-border)] rounded px-2 py-1">
                <NuxtLink v-if="row.public_id" :to="`/entities/${row.public_id}`"
                          class="text-[var(--ui-primary)] hover:underline truncate font-mono text-xs">
                  {{ row.public_id }}
                </NuxtLink>
                <span v-else class="truncate">{{ row.action }}</span>
                <span class="text-[var(--ui-text-muted)] shrink-0">{{ row.type ?? row.outcome ?? '' }}</span>
                <span class="text-[var(--ui-text-muted)] shrink-0">{{ fmtDate(row.at) }}</span>
              </div>
              <p v-if="!activityRows.length" class="text-center text-[var(--ui-text-muted)] py-4">
                {{ t('users.activity_empty') }}
              </p>
            </div>
          </div>
        </div>
      </template>
    </USlideover>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';

definePageMeta({
  middleware: ['auth', capability(['users.manage', 'user.read'])],
});

const { t, te } = useI18n();
const toast = useToast();
const api = useApi();
const authStore = useAuthStore();

// Granular capabilities (umbrella `users.manage` covers all of them).
const canCreate = computed(() => authStore.hasCapability('user.create') || authStore.hasCapability('users.manage'));
const canEdit = computed(() => authStore.hasCapability('user.edit') || authStore.hasCapability('users.manage'));
const canDelete = computed(() => authStore.hasCapability('user.delete') || authStore.hasCapability('users.manage'));

interface UserRow {
  principal_id: string;
  display_name: string;
  login: string;
  email?: string;
  status: string;
  last_seen_at?: string;
  last_ip?: string;
  roles: string[];
}

interface RoleGroup { id: string; name: string; class: string; parent_id?: string }

interface UserDetail {
  principal_id: string;
  display_name: string;
  login: string;
  email?: string;
  status: string;
  created_at?: string;
  last_seen_at?: string;
  roles: { id: string; name: string; class: string; parent_id?: string }[];
  effective_roles: string[];
  sessions: { session_id: string; ip_address?: string; user_agent?: string;
    started_at?: string; ended_at?: string }[];
}

interface ApiToken {
  token_id: string;
  name: string;
  created_at?: string;
  expires_at?: string | null;
  revoked_at?: string | null;
  last_used_at?: string | null;
}

const loading = ref(false);
const users = ref<UserRow[]>([]);
const roleGroups = ref<RoleGroup[]>([]);
const filters = ref<{ q: string; group_id: string | null; inherited: boolean; ip: string; status: string | null }>({
  q: '', group_id: null, inherited: true, ip: '', status: null,
});

const statusOptions = computed(() => [
  { label: t('users.status_all'), value: null },
  { label: t('users.status_active'), value: 'active' },
  { label: t('users.status_disabled'), value: 'disabled' },
]);

const roleOptions = computed(() => [
  { label: t('users.role_all'), value: null },
  ...roleGroups.value.map((g) => ({ label: g.name, value: g.id })),
]);

const roleSelectItems = computed(() =>
  roleGroups.value.map((g) => ({ label: `${g.name} (${g.class})`, value: g.id })),
);

async function loadRoles() {
  try {
    const res = await api.users.listRoleGroups();
    roleGroups.value = (res.data ?? []) as RoleGroup[];
  } catch { /* non-fatal: role filter stays empty */ }
}

async function load() {
  loading.value = true;
  try {
    const res = await api.users.list({
      q: filters.value.q || undefined,
      group_id: filters.value.group_id || undefined,
      inherited: filters.value.inherited,
      ip: filters.value.ip || undefined,
      status: filters.value.status || undefined,
    });
    users.value = (res.data ?? []) as UserRow[];
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    loading.value = false;
  }
}

function fmtDate(v?: string | null) {
  return v ? new Date(v).toLocaleString() : '—';
}

// --- detail / activity ---
const showDetail = ref(false);
const detail = ref<UserDetail | null>(null);
const activityTab = ref('created');
const activityQuery = ref('');
const activity = ref<{ created: any[]; changed: any[]; viewed: any[]; log: any[] }>({
  created: [], changed: [], viewed: [], log: [],
});

const activityTabs = computed(() => [
  { label: t('users.act_created'), value: 'created' },
  { label: t('users.act_changed'), value: 'changed' },
  { label: t('users.act_viewed'), value: 'viewed' },
  { label: t('users.act_log'), value: 'log' },
]);

const activityRows = computed(() => {
  const key = activityTab.value as 'created' | 'changed' | 'viewed' | 'log';
  return activity.value[key] ?? [];
});

const statusEditOptions = computed(() => [
  { label: t('users.status_one_active'), value: 'active' },
  { label: t('users.status_one_disabled'), value: 'disabled' },
  { label: t('users.status_one_locked'), value: 'locked' },
]);

const editForm = ref({ display_name: '', email: '', status: 'active', password: '', groups: [] as string[] });

async function saveEdit() {
  if (!detail.value) return;
  saving.value = true;
  try {
    await api.users.update(detail.value.principal_id, {
      display_name: editForm.value.display_name,
      email: editForm.value.email || null,
      status: editForm.value.status,
      password: editForm.value.password || undefined,
      groups: editForm.value.groups,
    });
    toast.add({ title: t('users.saved'), color: 'success' });
    await openDetail(detail.value.principal_id);
    await load();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    saving.value = false;
  }
}

async function openDetail(id: string) {
  try {
    const res = await api.users.get(id);
    detail.value = res.data as UserDetail;
    editForm.value = {
      display_name: detail.value.display_name,
      email: detail.value.email ?? '',
      status: detail.value.status,
      password: '',
      groups: detail.value.roles.map((r) => r.id),
    };
    activityTab.value = 'created';
    activityQuery.value = '';
    newToken.value = '';
    tokens.value = [];
    tokenForm.value = { name: '', days: '' };
    showDetail.value = true;
    await loadActivity();
    await loadTokens();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

async function loadActivity() {
  if (!detail.value) return;
  try {
    const res = await api.users.activity(detail.value.principal_id, activityQuery.value || undefined);
    activity.value = res.data as typeof activity.value;
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

// --- API tokens ---
const tokens = ref<ApiToken[]>([]);
const tokenForm = ref({ name: '', days: '' as string });
const tokenSaving = ref(false);
const newToken = ref('');

async function loadTokens() {
  if (!detail.value) return;
  try {
    const res = await api.users.listTokens(detail.value.principal_id);
    tokens.value = (res.data ?? []) as ApiToken[];
  } catch { /* non-fatal */ }
}

async function createToken() {
  if (!detail.value || !tokenForm.value.name.trim()) return;
  tokenSaving.value = true;
  newToken.value = '';
  try {
    const days = Number(tokenForm.value.days);
    const res = await api.users.createToken(detail.value.principal_id, {
      name: tokenForm.value.name.trim(),
      expires_in_days: Number.isFinite(days) && days > 0 ? days : undefined,
    });
    newToken.value = (res.data as { token?: string })?.token ?? '';
    tokenForm.value = { name: '', days: '' };
    await loadTokens();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    tokenSaving.value = false;
  }
}

async function revokeToken(tokenId: string) {
  if (!detail.value) return;
  try {
    await api.users.revokeToken(detail.value.principal_id, tokenId);
    await loadTokens();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

// --- create / disable ---
const showCreate = ref(false);
const saving = ref(false);
const createForm = ref({ login: '', display_name: '', email: '', password: '', is_service_account: false, groups: [] as string[] });

function openCreate() {
  createForm.value = { login: '', display_name: '', email: '', password: '', is_service_account: false, groups: [] };
  showCreate.value = true;
}

async function saveUser() {
  saving.value = true;
  try {
    await api.users.create({
      login: createForm.value.login,
      display_name: createForm.value.display_name,
      email: createForm.value.email || undefined,
      password: createForm.value.password || undefined,
      is_service_account: createForm.value.is_service_account || undefined,
      groups: createForm.value.groups,
    });
    showCreate.value = false;
    toast.add({ title: t('users.saved'), color: 'success' });
    await load();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    saving.value = false;
  }
}

async function disableUser(u: UserRow) {
  try {
    await api.users.remove(u.principal_id);
    toast.add({ title: t('users.disabled_ok'), color: 'success' });
    await load();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  }
}

onMounted(() => {
  loadRoles();
  load();
});
</script>
