<template>
  <UApp>
    <NuxtLayout>
      <NuxtPage />
    </NuxtLayout>
  </UApp>
</template>

<script setup lang="ts">
// Global setup: fetch session context on app load, but skip on public pages
const { fetchContext, isAuthenticated } = useAuth();
const route = useRoute();
const publicPaths = ['/login', '/health'];

onMounted(() => {
  if (!isAuthenticated.value && !publicPaths.includes(route.path)) {
    fetchContext();
  }
});
</script>
