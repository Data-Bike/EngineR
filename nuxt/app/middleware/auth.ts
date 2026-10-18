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

  // fetchContext is singleflight: an in-flight call (e.g. app.vue onMounted)
  // is awaited instead of skipped, so a valid session is never bounced to
  // /login by a cold-load race.
  if (!store.isAuthenticated) {
    await store.fetchContext();
  }

  if (!store.isAuthenticated) {
    return navigateTo({
      path: '/login',
      query: { redirect: to.fullPath },
    });
  }

  // URL access rules (system.url, UI paths): deny prefixes and allowlists
  // block route rendering entirely — no blank page, no leaked layout.
  if (!store.canUrl(to.path)) {
    throw createError({
      statusCode: 403,
      statusMessage: 'URL access denied',
    });
  }
});
