/**
 * useCatalog — active catalog selection (docs/28 §6).
 *
 * `/me/context.catalogs` lists every catalog the principal can read. The
 * header shows a selector when more than one is available; the choice is
 * persisted in localStorage and passed to catalog-aware API calls via
 * `catalogParams()` (`?catalog=` query parameter).
 *
 * An empty selection means "all accessible catalogs" — valid for list
 * pages; entity creation still needs a concrete catalog (the type's own
 * catalog wins server-side when the parameter is absent).
 */

import { useAuthStore, type CatalogContextEntry } from './useAuth';

const STORAGE_KEY = 'enginer.catalog';

export function useCatalog() {
  const authStore = useAuthStore();

  // SSR-safe shared state for the selected alias ('' = all catalogs).
  const selected = useState<string>('catalog.selected', () => '');

  const available = computed<CatalogContextEntry[]>(
    () => authStore.user?.catalogs ?? [],
  );

  // Restore the persisted choice once, on the client, and drop it when it
  // no longer resolves to an accessible catalog (permissions changed).
  // `authStore.user` being loaded is the signal — before it resolves,
  // `available` is legitimately empty and the selection must survive.
  if (import.meta.client) {
    watchEffect(() => {
      const stored = localStorage.getItem(STORAGE_KEY) ?? '';
      if (!selected.value && stored) selected.value = stored;
      if (
        authStore.user &&
        selected.value &&
        !available.value.some((c) => c.alias === selected.value)
      ) {
        selected.value = '';
        localStorage.removeItem(STORAGE_KEY);
      }
    });
  }

  const current = computed<CatalogContextEntry | null>(
    () => available.value.find((c) => c.alias === selected.value) ?? null,
  );

  /** Show the header selector only when a real choice exists. */
  const hasMultiple = computed(() => available.value.length > 1);

  function select(alias: string) {
    selected.value = alias;
    if (import.meta.client) {
      if (alias) localStorage.setItem(STORAGE_KEY, alias);
      else localStorage.removeItem(STORAGE_KEY);
    }
  }

  /** Query params fragment for catalog-aware endpoints. */
  function catalogParams(): { catalog?: string } {
    return selected.value ? { catalog: selected.value } : {};
  }

  return { available, current, selected, hasMultiple, select, catalogParams };
}
