<template>
  <div class="p-6 space-y-4">
    <h1 class="text-2xl font-bold">{{ t('nav.admin') }}</h1>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <NuxtLink to="/admin/schema" class="block">
        <UCard class="hover:border-[var(--ui-border-muted)]">
          <div class="flex items-center gap-3">
            <UIcon name="i-lucide-database" class="text-2xl" />
            <div>
              <p class="font-medium">{{ t('schema.title') }}</p>
              <p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.types') }}</p>
            </div>
          </div>
        </UCard>
      </NuxtLink>
    </div>
  </div>
</template>

<script setup lang="ts">
definePageMeta({
  middleware: ['auth'],
});

const { t } = useI18n();
const { requireCapability } = useCapability();
requireCapability('admin.view');
</script>
