<template>
  <NuxtLayout>
    <NuxtPage />
  </NuxtLayout>
</template>

<script setup lang="ts">
// Global setup: fetch session context on app load (docs/16 §5)
const { fetchContext, isAuthenticated } = useAuth();

// Only fetch if not already authenticated (e.g. after page refresh)
onMounted(() => {
  if (!isAuthenticated.value) {
    fetchContext();
  }
});
</script>
