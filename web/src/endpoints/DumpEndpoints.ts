/**
 * Database dump administration (export side of apps/dump).
 * Gated by the security.manage capability.
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';

export interface DumpFileInfo {
  name: string;
  size: number;
  download_url: string;
}

export interface DumpInfo {
  dump_id: string;
  status: 'running' | 'done' | 'failed' | string;
  exit_code?: number | null;
  finished_at?: string | null;
  files: DumpFileInfo[];
}

/** Dump administration endpoints. */
export class DumpEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** POST /admin/dump — start a full-database export. */
  async run(): Promise<ParsedResponse<{ dump_id: string; status: string }>> {
    return this.client.post('/admin/dump', {});
  }

  /** GET /admin/dump — list dump runs. */
  async list(): Promise<ParsedResponse<DumpInfo[]>> {
    return this.client.get<DumpInfo[]>('/admin/dump');
  }

  /** DELETE /admin/dump/{id} — remove a finished dump. */
  async remove(dumpId: string): Promise<ParsedResponse<{ deleted: boolean }>> {
    return this.client.delete(`/admin/dump/${encodeURIComponent(dumpId)}`);
  }
}
