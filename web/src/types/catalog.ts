/**
 * Catalog types (docs/28): second-level data isolation inside a tenant.
 * Every entity/type/schema stream belongs to exactly one catalog.
 */

/** Catalog row as returned by /catalogs. */
export interface CatalogRecord {
  catalog_id: number;
  public_id: string;
  alias: string;
  name: string;
  description: string;
  status: 'active' | 'retired';
  created_at: string;
  retired_at: string | null;
  entity_count: number;
  type_count: number;
}

/** Catalog slice embedded in /me/context — drives the UI selector. */
export interface CatalogContextEntry {
  public_id: string;
  alias: string;
  name: string;
}

/** POST /catalogs body. Alias: ^[a-z][a-z0-9_-]{0,62}$. */
export interface CreateCatalogBody {
  alias: string;
  name: string;
  description?: string;
  settings?: Record<string, unknown>;
}

/** PUT /catalogs/{ref} body. `status:'retired'` fails on non-empty catalogs. */
export interface UpdateCatalogBody {
  name?: string;
  description?: string;
  settings?: Record<string, unknown>;
  status?: 'active' | 'retired';
}
