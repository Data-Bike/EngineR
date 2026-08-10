<template>
  <NuxtLink
    v-if="to"
    :to="to"
    class="stat-card block rounded-lg border border-[var(--ui-border)] p-4 hover:border-[var(--ui-border-muted)] transition-colors"
  >
    <div class="flex items-center gap-3">
      <UIcon :name="icon" class="text-2xl text-[var(--ui-text-muted)]" />
      <div>
        <p class="text-2xl font-bold">{{ value }}</p>
        <p class="text-sm text-[var(--ui-text-muted)]">{{ label }}</p>
      </div>
    </div>
  </NuxtLink>
  <div
    v-else
    class="stat-card rounded-lg border border-[var(--ui-border)] p-4"
  >
    <div class="flex items-center gap-3">
      <UIcon :name="icon" class="text-2xl text-[var(--ui-text-muted)]" />
      <div>
        <p class="text-2xl font-bold">{{ value }}</p>
        <p class="text-sm text-[var(--ui-text-muted)]">{{ label }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  label: string;
  value: string | number;
  icon: string;
  to?: string;
}>();
</script>
