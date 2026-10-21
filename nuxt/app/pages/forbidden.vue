<template>
  <div class="flex items-center justify-center min-h-[60vh] p-4">
    <div class="text-center max-w-md">
      <UIcon name="i-lucide-shield-off" class="w-12 h-12 mx-auto mb-4 text-[var(--ui-text-muted)]" />
      <h1 class="text-2xl font-bold mb-2">{{ t('errors.forbidden') }}</h1>
      <p v-if="from" class="text-sm text-[var(--ui-text-muted)] mb-6 break-all">{{ from }}</p>
      <UButton to="/">{{ t('nav.home') }}</UButton>
    </div>
  </div>
</template>

<script setup lang="ts">
const { t } = useI18n();
const route = useRoute();
const from = computed(() => (typeof route.query.from === 'string' ? route.query.from : ''));

definePageMeta({ title: 'forbidden' });
</script>
