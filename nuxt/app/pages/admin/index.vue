<template>
  <div class="p-6 space-y-4">
    <h1 class="text-2xl font-bold">{{ t('nav.admin') }}</h1>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <NuxtLink to="/admin/schema" class="block">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-database" class="text-2xl text-[var(--ui-primary)]" />
            <div>
              <p class="font-medium">{{ t('schema.title') }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ t('admin.schema_desc') }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
      <NuxtLink to="/admin/types" class="block">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-boxes" class="text-2xl text-[var(--ui-primary)]" />
            <div>
              <p class="font-medium">{{ t('admin.types_title') }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ t('admin.types_desc') }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
      <NuxtLink to="/admin/predicates" class="block">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-share-2" class="text-2xl text-[var(--ui-primary)]" />
            <div>
              <p class="font-medium">{{ t('admin.predicates_title') }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ t('admin.predicates_desc') }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
      <NuxtLink to="/admin/permissions" class="block">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-shield" class="text-2xl text-[var(--ui-primary)]" />
            <div>
              <p class="font-medium">{{ t('admin.permissions_title') }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ t('admin.permissions_desc') }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
      <NuxtLink v-if="hasCapability('security.manage')" to="/admin/dump" class="block" data-testid="admin-dump-card">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-database-backup" class="text-2xl text-[var(--ui-primary)]" />
            <div>
              <p class="font-medium">{{ t('dump.title') }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ t('dump.desc') }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
      <NuxtLink v-if="hasCapability('users.manage') || hasCapability('user.read')" to="/admin/users" class="block" data-testid="admin-users-card">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-users" class="text-2xl text-[var(--ui-primary)]" />
            <div>
              <p class="font-medium">{{ t('users.title') }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ t('users.desc') }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
    </div>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';

definePageMeta({
  middleware: ['auth', capability('admin.pages.access')],
});

const { t } = useI18n();
const store = useAuthStore();
const hasCapability = (cap: string) => store.hasCapability(cap);
</script>
