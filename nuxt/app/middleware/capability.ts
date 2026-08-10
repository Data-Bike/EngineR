/**
 * capability middleware factory — checks coarse capability for UX.
 * Server re-checks on every request (docs/16 §5).
 *
 * Usage in pages:
 *   definePageMeta({ middleware: capability('entity.create') })
 */

export function capability(cap: string) {
  return defineNuxtRouteMiddleware((to) => {
    const store = useAuthStore();
    if (!store.isAuthenticated) {
      return navigateTo({
        path: '/login',
        query: { redirect: to.fullPath },
      });
    }
    if (!store.hasCapability(cap)) {
      throw createError({
        statusCode: 403,
        statusMessage: 'Capability denied',
      });
    }
  });
}
