/**
 * useCapability — coarse UX-level capability check (docs/16 §5).
 * Server re-checks on every request.
 *
 * Usage in pages:
 *   const { requireCapability } = useCapability();
 *   definePageMeta({ middleware: () => useCapability().requireCapability('admin.view') })
 */

export function useCapability() {
  const store = useAuthStore();

  function requireCapability(cap: string): boolean {
    if (!store.isAuthenticated) return false;
    if (!store.hasCapability(cap)) {
      throw createError({
        statusCode: 403,
        statusMessage: 'Capability denied',
      });
    }
    return true;
  }

  function hasCapability(cap: string): boolean {
    return store.hasCapability(cap);
  }

  return { requireCapability, hasCapability };
}
