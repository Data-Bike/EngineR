<template>
  <div class="min-h-screen flex items-center justify-center p-4">
    <div class="text-center max-w-md">
      <h1 class="text-4xl font-bold mb-2">{{ error?.statusCode || 500 }}</h1>
      <p class="text-lg text-[var(--ui-text-muted)] mb-4">
        {{ error?.statusMessage || t('errors.internal_error') }}
      </p>
      <p v-if="error?.requestId" class="text-sm text-[var(--ui-text-muted)] mb-4">
        {{ t('common.request_id') }}: {{ error.requestId }}
      </p>
      <UButton @click="handleError">
        {{ t('nav.home') }}
      </UButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { NuxtError } from '#app';

const { t } = useI18n();

defineProps<{
  error: NuxtError & { requestId?: string };
}>();

function handleError() {
  clearError({ redirect: '/' });
}
</script>
