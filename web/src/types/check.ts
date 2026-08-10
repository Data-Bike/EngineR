/**
 * Check, Assessment, Workflow, and Task API types (docs/06 §8, docs/14 §8-10,
 * docs/13 §7-8).
 */

// --- Check ---

/** Check request body (docs/14 §4). */
export interface CheckRequestBody {
  target_entity_id: string;
  check_kind: string;
  connector_instance_id?: string;
  parameters: Record<string, unknown>;
}

/** Check response. */
export interface CheckResponse {
  check_id: string;
  status: string;
  created_at: string;
}

/** Evidence response. */
export interface EvidenceResponse {
  evidence: Array<Record<string, unknown>>;
}

/** Check kinds (docs/14 §4). */
export const CheckKind = {
  Kyc: 'kyc',
  Sanctions: 'sanctions',
  FinancialHealth: 'financial_health',
  LegalStatus: 'legal_status',
  Ownership: 'ownership',
  Custom: 'custom',
} as const;

export type CheckKind = (typeof CheckKind)[keyof typeof CheckKind];

/** Check statuses. */
export const CheckStatus = {
  Pending: 'pending',
  Running: 'running',
  Completed: 'completed',
  Failed: 'failed',
  Skipped: 'skipped',
  Cancelled: 'cancelled',
} as const;

export type CheckStatus = (typeof CheckStatus)[keyof typeof CheckStatus];

// --- Assessment ---

/** Assessment response (docs/14 §10). */
export interface AssessmentResponse {
  assessment_id: string;
  target_entity_id: string;
  final_score: number;
  level: RiskLevel;
  status: string;
  generated_at: string;
  factors: FactorSummary[];
}

/** Factor summary. */
export interface FactorSummary {
  factor_id: string;
  criterion_id: string;
  severity: string;
  status: string;
}

/** Risk level (docs/14 §10). */
export const RiskLevel = {
  Minimal: 'minimal',
  Low: 'low',
  Medium: 'medium',
  High: 'high',
  Critical: 'critical',
} as const;

export type RiskLevel = (typeof RiskLevel)[keyof typeof RiskLevel];

/** Trigger assessment request. */
export interface TriggerAssessmentBody {
  model_id?: string;
  force: boolean;
}

// --- Workflow ---

/** Start workflow request (docs/13 §7). */
export interface StartWorkflowBody {
  definition_id: string;
  target_entity_id: string;
  variables: Record<string, unknown>;
}

/** Workflow response. */
export interface WorkflowResponse {
  instance_id: string;
  definition_id: string;
  current_state: string;
  terminated: boolean;
}

/** Transition request. */
export interface TransitionBody {
  to: string;
  trigger: string;
}

// --- Task ---

/** Task response (docs/13 §8). */
export interface TaskResponse {
  task_id: string;
  title: string;
  kind: string;
  status: TaskStatus;
  priority: TaskPriority;
  assignee_id: string | null;
  due_date: string;
}

/** Task statuses. */
export const TaskStatus = {
  Pending: 'pending',
  Assigned: 'assigned',
  InProgress: 'in_progress',
  Completed: 'completed',
  Cancelled: 'cancelled',
  Overdue: 'overdue',
  Escalated: 'escalated',
} as const;

export type TaskStatus = (typeof TaskStatus)[keyof typeof TaskStatus];

/** Task priorities. */
export const TaskPriority = {
  Low: 'low',
  Normal: 'normal',
  High: 'high',
  Urgent: 'urgent',
} as const;

export type TaskPriority = (typeof TaskPriority)[keyof typeof TaskPriority];

/** Complete task request. */
export interface CompleteTaskBody {
  result: Record<string, unknown>;
}

/** Cancel task request. */
export interface CancelTaskBody {
  reason: string;
}

/** Decision kind (docs/14 §10). */
export const DecisionKind = {
  Approve: 'approve',
  Reject: 'reject',
  Escalate: 'escalate',
  Defer: 'defer',
  ConditionalApprove: 'conditional_approve',
} as const;

export type DecisionKind = (typeof DecisionKind)[keyof typeof DecisionKind];
