<template>
  <aside
    :class="[
      'border-r border-[var(--ui-border)] overflow-y-auto transition-all shrink-0',
      collapsed ? 'w-16' : 'w-64',
    ]"
  >
    <nav
      class="p-2"
      role="navigation"
      aria-label="Main navigation"
    >
      <ul class="space-y-1">
        <li v-for="item in menuItems" :key="item.to">
          <NuxtLink
            :to="item.to"
            class="flex items-center gap-3 rounded-md px-3 py-2 text-sm hover:bg-[var(--ui-bg-muted)] transition-colors"
            active-class="bg-[var(--ui-bg-muted)] font-medium"
          >
            <UIcon :name="item.icon" class="shrink-0" />
            <span v-if="!collapsed" class="truncate">{{ t(item.label) }}</span>
          </NuxtLink>
        </li>
      </ul>
    </nav>
  </aside>
</template>

<script setup lang="ts">
const { t } = useI18n();
const authStore = useAuthStore();
const { capabilities } = useAuth();

defineProps<{ collapsed?: boolean }>();

interface MenuItem {
  to: string;
  label: string;
  icon: string;
  capability?: string;
}

const allItems: MenuItem[] = [
  { to: '/', label: 'nav.home', icon: 'i-lucide-home' },
  { to: '/objects', label: 'nav.objects', icon: 'i-lucide-database', capability: 'entity.read' },
  { to: '/requests', label: 'nav.requests', icon: 'i-lucide-clipboard-list', capability: 'entity.read' },
  { to: '/checks', label: 'nav.checks', icon: 'i-lucide-shield-check', capability: 'check.view' },
  { to: '/monitoring', label: 'nav.monitoring', icon: 'i-lucide-eye', capability: 'monitor.manage' },
  { to: '/risks', label: 'nav.risks', icon: 'i-lucide-alert-triangle', capability: 'risk.view' },
  { to: '/reports', label: 'nav.reports', icon: 'i-lucide-file-text', capability: 'report.view' },
  { to: '/docs', label: 'nav.docs', icon: 'i-lucide-book-open' },
  { to: '/admin', label: 'nav.admin', icon: 'i-lucide-settings', capability: 'admin.pages.access' },
  { to: '/integrations', label: 'nav.integrations', icon: 'i-lucide-plug', capability: 'connector.manage' },
  { to: '/audit', label: 'nav.audit', icon: 'i-lucide-scroll-text', capability: 'audit.read' },
];

// Menu entries are hidden (not rendered at all) when the capability or the
// URL rule denies access — the server re-checks on every API call.
const menuItems = computed(() =>
  allItems.filter(
    (item) =>
      (!item.capability || capabilities.value.includes(item.capability)) &&
      authStore.canUrl(item.to),
  ),
);
</script>
