/**
 * capability middleware factory — checks coarse capability for UX.
 * Server re-checks on every request (docs/16 §5).
 *
 * Usage in pages:
 *   definePageMeta({ middleware: capability('entity.create') })
 *   definePageMeta({ middleware: capability(['users.manage', 'user.read']) })
 * An array means "any of" — for surfaces accepting an umbrella grant.
 */

export function capability(cap: string | string[]) {
  const required = Array.isArray(cap) ? cap : [cap];
  return defineNuxtRouteMiddleware((to) => {
    const store = useAuthStore();
    if (!store.isAuthenticated) {
      return navigateTo({
        path: '/login',
        query: { redirect: to.fullPath },
      });
    }
    if (!required.some((c) => store.hasCapability(c))) {
      // navigateTo instead of throwing createError — a thrown error on a
      // client-side deep link surfaces as a [NUXT_E1005] console diagnostic
      // even though the error page renders correctly.
      return navigateTo({ path: '/forbidden', query: { from: to.fullPath } });
    }
  });
}
