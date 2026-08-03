<template>
  <div class="p-6 space-y-4">
    <h1 class="text-2xl font-bold">{{ t('nav.objects') }}</h1>
    <p class="text-[var(--ui-text-muted)]">{{ t('entity.empty') }}</p>
  </div>
</template>

<script setup lang="ts">
definePageMeta({
  middleware: 'auth',
});

const { t } = useI18n();
</script>
