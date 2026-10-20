/**
 * Shared menu definitions — single source for the sidebar, breadcrumbs
 * and the permission editor's menu-path suggestions (docs/28, docs/16).
 *
 * `capability` mirrors the backend capability required to use the page;
 * `label` is an i18n key. URL rules (`ui_rules`) hide entries the same
 * way `dataperm::can_url` denies them server-side.
 */
export interface MenuItem {
  to: string;
  label: string;
  icon: string;
  /** Required capability — entry hidden without it. */
  capability?: string;
  /** Alternative capabilities — entry shown when ANY is granted
   *  (e.g. users.manage umbrella over user.read). */
  capabilities?: string[];
  /** Optional i18n key for a description line (admin dashboard cards). */
  desc?: string;
}

/** Primary sidebar entries. */
export const MENU_ITEMS: MenuItem[] = [
  { to: '/', label: 'nav.home', icon: 'i-lucide-home' },
  { to: '/objects', label: 'nav.objects', icon: 'i-lucide-database', capability: 'entity.read' },
  { to: '/requests', label: 'nav.requests', icon: 'i-lucide-clipboard-list', capability: 'entity.read' },
  { to: '/checks', label: 'nav.checks', icon: 'i-lucide-shield-check', capability: 'check.view' },
  { to: '/monitoring', label: 'nav.monitoring', icon: 'i-lucide-eye', capability: 'monitor.manage' },
  { to: '/risks', label: 'nav.risks', icon: 'i-lucide-alert-triangle', capability: 'risk.view' },
  { to: '/reports', label: 'nav.reports', icon: 'i-lucide-file-text', capability: 'report.view' },
  { to: '/docs', label: 'nav.docs', icon: 'i-lucide-book-open' },
  { to: '/admin', label: 'nav.admin', icon: 'i-lucide-settings', capability: 'admin.pages.access' },
  { to: '/integrations', label: 'nav.integrations', icon: 'i-lucide-plug', capability: 'connector.manage' },
  { to: '/audit', label: 'nav.audit', icon: 'i-lucide-scroll-text', capability: 'audit.read' },
];

/** Pages reachable from the /admin dashboard card grid. The /admin route
 *  itself is gated by admin.pages.access, so cards only add finer-grained
 *  capability + URL-rule checks on top. */
export const ADMIN_ITEMS: MenuItem[] = [
  { to: '/admin/schema', label: 'schema.title', icon: 'i-lucide-database', desc: 'admin.schema_desc' },
  { to: '/admin/types', label: 'admin.types_title', icon: 'i-lucide-boxes', desc: 'admin.types_desc' },
  { to: '/admin/catalogs', label: 'catalog.title', icon: 'i-lucide-layers', desc: 'catalog.desc' },
  { to: '/admin/predicates', label: 'admin.predicates_title', icon: 'i-lucide-share-2', desc: 'admin.predicates_desc' },
  { to: '/admin/permissions', label: 'admin.permissions_title', icon: 'i-lucide-shield', desc: 'admin.permissions_desc' },
  { to: '/admin/users', label: 'users.title', icon: 'i-lucide-users', capabilities: ['user.read', 'users.manage'], desc: 'users.desc' },
  { to: '/admin/dump', label: 'dump.title', icon: 'i-lucide-database-backup', capability: 'security.manage', desc: 'dump.desc' },
];

/** Segment → i18n key map for breadcrumbs (top-level menu paths plus
 *  admin sub-pages); unknown segments render as the raw path segment. */
export const SEGMENT_LABEL_KEYS: Record<string, string> = {
  objects: 'nav.objects',
  entities: 'nav.objects',
  requests: 'nav.requests',
  checks: 'nav.checks',
  monitoring: 'nav.monitoring',
  risks: 'nav.risks',
  reports: 'nav.reports',
  docs: 'nav.docs',
  admin: 'nav.admin',
  integrations: 'nav.integrations',
  audit: 'nav.audit',
  search: 'entity.search',
  create: 'entity.create',
  schema: 'schema.title',
  types: 'admin.types_title',
  catalogs: 'catalog.title',
  predicates: 'admin.predicates_title',
  permissions: 'admin.permissions_title',
  users: 'users.title',
  dump: 'dump.title',
};

/** Intermediate-crumb link overrides for segments whose own accumulated
 *  path is NOT a navigable page (e.g. /entities has no index route —
 *  entity cards live under /entities/[publicId]). */
export const SEGMENT_LINKS: Record<string, string> = {
  entities: '/objects',
};
