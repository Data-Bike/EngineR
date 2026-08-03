<template>
  <template v-if="!isAuthenticated">
    <NuxtPage />
  </template>
  <template v-else>
    <NuxtLayout name="default">
      <NuxtPage />
    </NuxtLayout>
  </template>
</template>

<script setup lang="ts">
const { isAuthenticated } = useAuth();
</script>
