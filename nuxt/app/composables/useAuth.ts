/**
 * useAuth — session context and authorization state (docs/16 §5).
 *
 * GET /me/context returns profile, locale/timezone, menu capabilities,
 * available workspaces, policy revision and safe presentation grants.
 * Route middleware checks coarse capability for UX, but server re-checks.
 */

import { defineStore } from 'pinia';

/** UI route rules delivered by /me/context (system.url rules, UI paths).
 * Mirrors dataperm::can_url: longest-prefix match wins, `blanket` means an
 * unmatched path is denied. */
export interface UiRules {
  rules: { path: string; allow: boolean }[];
  blanket: boolean;
}

/** Catalog entry from /me/context (docs/28 §6). */
export interface CatalogContextEntry {
  public_id: string;
  alias: string;
  name: string;
}

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
  ui_rules?: UiRules;
  /** Catalogs the principal can read — drives the catalog selector. */
  catalogs?: CatalogContextEntry[];
}

// Singleflight for /me/context: app.vue onMounted and the auth middleware can
// both trigger a fetch on cold load; concurrent calls must share one request,
// otherwise a middleware arriving mid-fetch sees a still-empty store and
// bounces a valid session to /login.
let contextInFlight: Promise<void> | null = null;

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
      if (contextInFlight) return contextInFlight;
      this.loading = true;
      this.error = null;
      contextInFlight = (async () => {
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
          contextInFlight = null;
        }
      })();
      return contextInFlight;
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

    /**
     * Check whether a frontend route path is allowed by the principal's
     * URL rules — identical semantics to `dataperm::can_url` on the server:
     * longest-prefix match wins (an allow rule can carve a hole in a broader
     * deny and vice versa); with `blanket` set, an unmatched path is denied.
     * The server re-checks every request.
     */
    canUrl(path: string): boolean {
      const uiRules = this.user?.ui_rules;
      if (!uiRules) return true;
      const matches = (prefix: string) =>
        path === prefix ||
        prefix === '/' ||
        prefix === '*' ||
        path.startsWith(prefix + '/');
      const sorted = [...(uiRules.rules ?? [])].sort(
        (a, b) => b.path.length - a.path.length,
      );
      for (const rule of sorted) {
        if (matches(rule.path)) return rule.allow;
      }
      return !uiRules.blanket;
    },

    /**
     * Menu access decision (capability + URL rule) — shared by the sidebar,
     * admin dashboard cards and any navigation surface so they cannot
     * drift apart. `capabilities` are alternatives (any-of, e.g. a manage
     * umbrella over a granular read). The server re-checks every API call.
     */
    canAccessMenuItem(item: {
      to: string;
      capability?: string;
      capabilities?: string[];
    }): boolean {
      const caps = [
        item.capability,
        ...(item.capabilities ?? []),
      ].filter((c): c is string => !!c);
      return (
        (caps.length === 0 || caps.some((c) => this.hasCapability(c))) &&
        this.canUrl(item.to)
      );
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
    hasCapability: (cap: string) => store.hasCapability(cap),
    canUrl: (path: string) => store.canUrl(path),
    canAccessMenuItem: (item: {
      to: string;
      capability?: string;
      capabilities?: string[];
    }) => store.canAccessMenuItem(item),
  };
}
