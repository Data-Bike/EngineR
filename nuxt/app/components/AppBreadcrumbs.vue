<template>
  <nav v-if="items.length > 1" aria-label="Breadcrumb"
       class="border-b border-[var(--ui-border)] px-4 sm:px-6 py-2 shrink-0">
    <UBreadcrumb :items="items" />
  </nav>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useBreadcrumbLabels } from '~/composables/useBreadcrumbLabels';
import { SEGMENT_LABEL_KEYS, SEGMENT_LINKS } from '~/utils/menu';

const { t, te, locale } = useI18n();
const route = useRoute();
const router = useRouter();
const localePath = useLocalePath();
const { breadcrumbLabels } = useBreadcrumbLabels();

// Path-derived crumbs: known segments map to i18n labels (nav/admin
// pages); dynamic ids (type alias, public_id) render as the segment.
// The last crumb is the current page — no link. Intermediate crumbs
// link only when the accumulated path resolves to a real route —
// otherwise they render as plain text instead of leading to a 404.
const items = computed(() => {
  const segs = route.path.split('/').filter(Boolean);
  // `prefix_except_default` prepends /{locale} to non-default locale
  // routes — it is not a breadcrumb level, drop it before mapping.
  if (segs.length > 0 && segs[0] === locale.value) segs.shift();
  const crumbs: { label: string; to?: string; icon?: string }[] = [
    { label: t('nav.home'), to: localePath('/'), icon: 'i-lucide-home' },
  ];
  let acc = '';
  segs.forEach((seg, i) => {
    acc += `/${seg}`;
    const key = SEGMENT_LABEL_KEYS[seg];
    const link = localePath(SEGMENT_LINKS[seg] ?? acc);
    const resolved =
      i === segs.length - 1
        ? undefined
        : safeResolve(link)
          ? link
          : undefined;
    const label =
      breadcrumbLabels.value[seg] ??
      (key && te(key) ? t(key) : decodeURIComponent(seg));
    crumbs.push({
      // Cap very long display names so a single crumb can't blow up the
      // nav; the full name remains on the page heading.
      label: label.length > 60 ? `${label.slice(0, 57)}…` : label,
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
