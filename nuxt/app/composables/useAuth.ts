/**
 * useAuth — session context and authorization state (docs/16 §5).
 *
 * GET /me/context returns profile, locale/timezone, menu capabilities,
 * available workspaces, policy revision and safe presentation grants.
 * Route middleware checks coarse capability for UX, but server re-checks.
 */

import { defineStore } from 'pinia';

/** User profile from /me/context. */
export interface UserProfile {
  principal_id: string;
  display_name: string;
  tenant_id: string;
  locale: string;
  timezone: string;
  auth_strength: string;
  groups_revision: number;
  capabilities: string[];
  workspaces: string[];
  policy_revision: number;
  csrf_token: string;
}

/** Auth store — holds session state. */
export const useAuthStore = defineStore('auth', {
  state: () => ({
    user: null as UserProfile | null,
    loading: false,
    error: null as string | null,
  }),

  getters: {
    isAuthenticated: (state) => state.user !== null,
    capabilities: (state) => state.user?.capabilities ?? [],
    policyRevision: (state) => state.user?.policy_revision ?? 0,
  },

  actions: {
    /** Fetch the session context from the server. */
    async fetchContext() {
      this.loading = true;
      this.error = null;
      try {
        // Use $fetch directly instead of ApiClient to avoid issues with
        // composables not being ready during middleware execution.
        const config = useRuntimeConfig();
        const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
        const url = `${baseUrl.replace(/\/$/, '')}/me/context`;
        const response = await $fetch<{ data: UserProfile; meta: unknown }>(url, {
          credentials: 'include',
          headers: { Accept: 'application/json' },
        });
        this.user = response.data;
      } catch (e: unknown) {
        this.user = null;
        this.error = e instanceof Error ? e.message : String(e);
      } finally {
        this.loading = false;
      }
    },

    /** Clear session (logout). Calls server logout endpoint to invalidate cookie. */
    async clear() {
      try {
        const config = useRuntimeConfig();
        const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
        const headers: Record<string, string> = { Accept: 'application/json' };
        if (this.user?.csrf_token) {
          headers['X-CSRF-Token'] = this.user.csrf_token;
        }
        await $fetch(`${baseUrl.replace(/\/$/, '')}/auth/logout`, {
          method: 'POST',
          credentials: 'include',
          headers,
        });
      } catch {
        // Ignore network errors on logout — still clear local state
      }
      this.user = null;
      this.error = null;
    },

    /** Check if user has a capability (coarse UX check; server re-checks). */
    hasCapability(cap: string): boolean {
      return this.capabilities.includes(cap);
    },
  },
});

/**
 * Composable wrapper for the auth store.
 * Also provides a `requireCapability` helper for route guards.
 */
export function useAuth() {
  const store = useAuthStore();

  function requireCapability(cap: string): boolean {
    if (!store.isAuthenticated) return false;
    return store.hasCapability(cap);
  }

  return {
    user: computed(() => store.user),
    isAuthenticated: computed(() => store.isAuthenticated),
    capabilities: computed(() => store.capabilities),
    loading: computed(() => store.loading),
    error: computed(() => store.error),
    fetchContext: () => store.fetchContext(),
    logout: () => store.clear(),
    requireCapability,
  };
}
