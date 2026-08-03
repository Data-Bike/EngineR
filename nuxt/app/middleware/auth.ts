/**
 * auth middleware — redirects to /login if not authenticated.
 * Checks the Pinia auth store; if empty, attempts to fetch /me/context
 * once. If that fails, redirects to login (docs/16 §5).
 */

export default defineNuxtRouteMiddleware(async (to) => {
  // Skip for login page and public pages
  const publicPaths = ['/login', '/health'];
  if (publicPaths.includes(to.path)) return;

  const store = useAuthStore();

  if (!store.isAuthenticated && !store.loading) {
    await store.fetchContext();
  }

  if (!store.isAuthenticated) {
    return navigateTo({
      path: '/login',
      query: { redirect: to.fullPath },
    });
  }
});
