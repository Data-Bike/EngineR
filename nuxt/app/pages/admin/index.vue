<template>
  <div class="p-6 space-y-4">
    <h1 class="text-2xl font-bold">{{ t('nav.admin') }}</h1>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <NuxtLink v-for="item in visibleCards" :key="item.to" :to="item.to" class="block"
                :data-testid="`admin-card-${item.to.split('/').pop()}`">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon :name="item.icon" class="text-2xl text-[var(--ui-primary)]" />
            <div>
              <p class="font-medium">{{ t(item.label) }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ item.desc ? t(item.desc) : '' }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
    </div>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ADMIN_ITEMS } from '~/utils/menu';

definePageMeta({
  middleware: ['auth', capability('admin.pages.access')],
});

const { t } = useI18n();
const { canAccessMenuItem } = useAuth();

// Cards share the menu access decision (capability + URL rule) — the
// server re-checks on every API call.
const visibleCards = computed(() => ADMIN_ITEMS.filter(canAccessMenuItem));
</script>
