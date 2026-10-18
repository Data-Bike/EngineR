<template>
  <UBadge :color="color" :variant="level === 'critical' ? 'solid' : 'subtle'" size="sm">
    <UIcon :name="icon" class="mr-1" />
    {{ t(`entity.risks.level.${level}`) }}
  </UBadge>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { RiskLevel } from '@enginer/api-client';

const props = defineProps<{
  level: RiskLevel;
}>();

const { t } = useI18n();

type BadgeColor = 'primary' | 'secondary' | 'success' | 'info' | 'warning' | 'error' | 'neutral';

const color = computed<BadgeColor>(() => {
  const colors: Record<RiskLevel, BadgeColor> = {
    minimal: 'neutral',
    low: 'success',
    medium: 'warning',
    high: 'error',
    critical: 'error',
  };
  return colors[props.level];
});

const icon = computed(() => {
  const icons: Record<RiskLevel, string> = {
    minimal: 'i-lucide-shield-check',
    low: 'i-lucide-check',
    medium: 'i-lucide-alert-circle',
    high: 'i-lucide-alert-triangle',
    critical: 'i-lucide-octagon-alert',
  };
  return icons[props.level];
});
</script>
