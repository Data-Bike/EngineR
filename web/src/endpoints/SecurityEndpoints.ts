/**
 * Security API endpoints (docs/09).
 *
 * These are internal endpoints under /internal/security/v1, used by the PEP
 * in the API server. The Nuxt client calls the public admin surface, not
 * these directly — but the types are shared.
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type {
  ActionDecisionRequest,
  ActionDecisionResponse,
  QueryDecisionRequest,
  QueryDecisionResponse,
} from '../types/index.js';

/** Security API endpoint methods (internal). */
export class SecurityEndpoints {
  constructor(private readonly client: ApiClient) {}

  /** POST /internal/security/v1/decisions/action — action decision (docs/09 §4). */
  async actionDecision(req: ActionDecisionRequest): Promise<ParsedResponse<ActionDecisionResponse>> {
    return this.client.post<ActionDecisionResponse>('/decisions/action', req);
  }

  /** POST /internal/security/v1/decisions/query — query decision (docs/09 §5). */
  async queryDecision(req: QueryDecisionRequest): Promise<ParsedResponse<QueryDecisionResponse>> {
    return this.client.post<QueryDecisionResponse>('/decisions/query', req);
  }
}
