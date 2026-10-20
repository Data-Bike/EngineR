/**
 * Catalog API endpoints (docs/28 §6).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type { CatalogRecord, CreateCatalogBody, UpdateCatalogBody } from '../types/index.js';

/** Catalog CRUD endpoint methods. */
export class CatalogEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** GET /catalogs — catalogs the principal can read (permission-filtered). */
  async list(): Promise<ParsedResponse<CatalogRecord[]>> {
    return this.client.get<CatalogRecord[]>('/catalogs');
  }

  /** GET /catalogs/{ref} — one catalog by alias or public_id. */
  async get(ref: string): Promise<ParsedResponse<CatalogRecord>> {
    return this.client.get<CatalogRecord>(`/catalogs/${encodeURIComponent(ref)}`);
  }

  /** POST /catalogs — create a custom catalog (catalog.manage). */
  async create(body: CreateCatalogBody): Promise<ParsedResponse<CatalogRecord>> {
    return this.client.post<CatalogRecord>('/catalogs', body);
  }

  /** PUT /catalogs/{ref} — update name/description/settings/status. */
  async update(ref: string, body: UpdateCatalogBody): Promise<ParsedResponse<CatalogRecord>> {
    return this.client.put<CatalogRecord>(`/catalogs/${encodeURIComponent(ref)}`, body);
  }

  /** POST /catalogs/{ref}/retire — retire (fails while non-empty). */
  async retire(ref: string): Promise<ParsedResponse<CatalogRecord>> {
    return this.client.post<CatalogRecord>(`/catalogs/${encodeURIComponent(ref)}/retire`);
  }
}
