/**
 * Security API contract types (docs/09 §3-5).
 *
 * Mirrors `enginer-security-api::contract`.
 */

/** Decision effect wire form. */
export type DecisionEffect =
  | 'allow'
  | 'allow_with_obligations'
  | 'deny'
  | 'indeterminate'
  | 'challenge';

/** Principal slice of the request context (docs/09 §3). */
export interface SecurityContextDto {
  request_id: string;
  tenant_id: string;
  principal_id: string;
  session_id: string;
  auth_strength: string;
  groups_revision: number;
  purpose: string;
  locale: string;
  time: string;
  client_service: string;
}

/** Action decision request (docs/09 §4). */
export interface ActionDecisionRequest {
  context: SecurityContextDto;
  action: string;
  resource_kind: string;
  resource_id?: string;
  parameters?: Record<string, unknown>;
}

/** Query decision request (docs/09 §5). */
export interface QueryDecisionRequest {
  context: SecurityContextDto;
  action: string;
  root_type_ids: string[];
  selected_predicate_ids: string[];
  relation_predicate_ids: string[];
  operations: string[];
  requested_page_size: number;
  requested_graph_depth: number;
  query_fingerprint: string;
}

/** Action decision response (docs/09 §4). */
export interface ActionDecisionResponse {
  decision_id: string;
  effect: DecisionEffect;
  policy_revision: number;
  obligations: unknown;
  expires_at: string;
}

/** Query decision response (docs/09 §5). */
export interface QueryDecisionResponse {
  decision_id: string;
  effect: DecisionEffect;
  policy_revision: number;
  obligations: unknown;
  expires_at: string;
}
