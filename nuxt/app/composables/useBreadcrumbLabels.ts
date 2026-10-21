import { ref } from 'vue';

// Shared crumb-label overrides: detail pages register a human label for
// their raw id path segment (entity display name, check target…), and the
// breadcrumb bar renders it instead of the opaque uuid. Module-level ref —
// a single active page owns the current trail.
const labels = ref<Record<string, string>>({});

export function useBreadcrumbLabels() {
  function setBreadcrumbLabel(segment: string, label: string | null) {
    if (label) {
      labels.value[segment] = label;
    } else {
      delete labels.value[segment];
    }
  }
  return { breadcrumbLabels: labels, setBreadcrumbLabel };
}
