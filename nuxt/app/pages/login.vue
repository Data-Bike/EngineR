<template>
  <div class="min-h-screen flex items-center justify-center bg-[var(--ui-bg-muted)] p-4">
    <UCard class="w-full max-w-md">
      <template #header>
        <h1 class="text-2xl font-bold text-center">
          {{ t('auth.login') }}
        </h1>
        <p class="text-sm text-[var(--ui-text-muted)] text-center mt-1">
          {{ t('app.tagline') }}
        </p>
      </template>

      <form @submit.prevent="onSubmit" class="space-y-4">
        <UFormField :label="t('auth.username')" name="username">
          <UInput
            v-model="form.username"
            :placeholder="t('auth.username')"
            autocomplete="username"
            required
            class="w-full"
          />
        </UFormField>

        <UFormField :label="t('auth.password')" name="password">
          <UInput
            v-model="form.password"
            type="password"
            :placeholder="t('auth.password')"
            autocomplete="current-password"
            required
            class="w-full"
          />
        </UFormField>

        <UAlert
          v-if="error"
          color="error"
          :title="t('auth.error')"
          :description="error"
        />

        <UButton
          type="submit"
          block
          :loading="loading"
        >
          {{ t('auth.submit') }}
        </UButton>
      </form>
    </UCard>
  </div>
</template>

<script setup lang="ts">
definePageMeta({
  layout: 'auth',
});

const { t } = useI18n();
const { fetchContext } = useAuth();

const form = reactive({ username: '', password: '' });
const loading = ref(false);
const error = ref<string | null>(null);

async function onSubmit() {
  loading.value = true;
  error.value = null;
  try {
    // Use $fetch directly for login (cookie-session).
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    await $fetch(`${baseUrl.replace(/\/$/, '')}/auth/login`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: { username: form.username, password: form.password },
    });
    await fetchContext();

    // Redirect to originally requested page or home
    const route = useRoute();
    const redirect = (route.query.redirect as string) ?? '/';
    await navigateTo(redirect);
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : t('auth.error');
  } finally {
    loading.value = false;
  }
}
</script>
