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
import { MENU_ITEMS } from '~/utils/menu';

const { t } = useI18n();
const { canAccessMenuItem } = useAuth();

defineProps<{ collapsed?: boolean }>();

// Menu entries are hidden (not rendered at all) when the capability or the
// URL rule denies access — the server re-checks on every API call.
const menuItems = computed(() => MENU_ITEMS.filter(canAccessMenuItem));
</script>
