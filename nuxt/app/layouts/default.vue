<template>
  <div class="min-h-screen flex flex-col">
    <!-- Skip link for keyboard navigation (WCAG 2.2 AA, docs/16 §12) -->
    <a href="#main-content" class="skip-link">Skip to content</a>

    <!-- Header -->
    <AppHeader @toggle-sidebar="sidebarCollapsed = !sidebarCollapsed" />

    <!-- Body: sidebar + content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Collapsible left navigation -->
      <AppSidebar :collapsed="sidebarCollapsed" />

      <!-- Main content area -->
      <main id="main-content" class="flex-1 min-w-0 overflow-auto focus:outline-none" tabindex="-1">
        <NuxtPage />
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
const sidebarCollapsed = ref(false);
</script>
