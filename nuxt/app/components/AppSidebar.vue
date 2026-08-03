<template>
  <aside
    :class="[
      'border-r border-[var(--ui-border)] overflow-y-auto transition-all',
      collapsed ? 'w-16' : 'w-64',
    ]"
    role="navigation"
    aria-label="Main navigation"
  >
    <nav class="p-2">
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
  { to: '/objects', label: 'nav.objects', icon: 'i-lucide-database' },
  { to: '/requests', label: 'nav.requests', icon: 'i-lucide-clipboard-list' },
  { to: '/checks', label: 'nav.checks', icon: 'i-lucide-shield-check' },
  { to: '/monitoring', label: 'nav.monitoring', icon: 'i-lucide-eye' },
  { to: '/risks', label: 'nav.risks', icon: 'i-lucide-alert-triangle' },
  { to: '/reports', label: 'nav.reports', icon: 'i-lucide-file-text' },
  { to: '/docs', label: 'nav.docs', icon: 'i-lucide-book-open' },
  { to: '/admin', label: 'nav.admin', icon: 'i-lucide-settings', capability: 'admin.view' },
  { to: '/integrations', label: 'nav.integrations', icon: 'i-lucide-plug', capability: 'admin.view' },
  { to: '/audit', label: 'nav.audit', icon: 'i-lucide-scroll-text', capability: 'audit.view' },
];

const menuItems = computed(() =>
  allItems.filter((item) => !item.capability || capabilities.value.includes(item.capability)),
);
</script>
