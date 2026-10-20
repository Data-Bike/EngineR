<template>
  <nav v-if="items.length > 1" aria-label="Breadcrumb"
       class="border-b border-[var(--ui-border)] px-4 sm:px-6 py-2 shrink-0">
    <UBreadcrumb :items="items" />
  </nav>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { SEGMENT_LABEL_KEYS, SEGMENT_LINKS } from '~/utils/menu';

const { t, te } = useI18n();
const route = useRoute();
const router = useRouter();

// Path-derived crumbs: known segments map to i18n labels (nav/admin
// pages); dynamic ids (type alias, public_id) render as the segment.
// The last crumb is the current page — no link. Intermediate crumbs
// link only when the accumulated path resolves to a real route —
// otherwise they render as plain text instead of leading to a 404.
const items = computed(() => {
  const segs = route.path.split('/').filter(Boolean);
  const crumbs: { label: string; to?: string; icon?: string }[] = [
    { label: t('nav.home'), to: '/', icon: 'i-lucide-home' },
  ];
  let acc = '';
  segs.forEach((seg, i) => {
    acc += `/${seg}`;
    const key = SEGMENT_LABEL_KEYS[seg];
    const link = SEGMENT_LINKS[seg] ?? acc;
    const resolved =
      i === segs.length - 1
        ? undefined
        : safeResolve(link)
          ? link
          : undefined;
    crumbs.push({
      label: key && te(key) ? t(key) : decodeURIComponent(seg),
      ...(resolved ? { to: resolved } : {}),
    });
  });
  return crumbs;
});

function safeResolve(path: string): boolean {
  try {
    const r = router.resolve(path) as { matched?: unknown[] };
    return (r.matched?.length ?? 1) > 0;
  } catch {
    return true;
  }
}
</script>
