/**
 * SKR domain types: checks, risks, factors, assessments, tasks, monitors,
 * reports (docs/04, docs/05, docs/14).
 *
 * These types mirror the Rust contracts for the SKR package and are used
 * by the Nuxt UI tabs.
 */

// --- Checks (docs/04 §4) ---

export type CheckStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'skipped'
  | 'cancelled';

export type CheckKind =
  | 'sanction'
  | 'pep'
  | 'adverse_media'
  | 'registry'
  | 'identity'
  | 'address'
  | 'tax'
  | 'custom';

/** A check instance on an entity. */
export interface CheckInstance {
  check_id: string;
  entity_public_id: string;
  kind: CheckKind;
  status: CheckStatus;
  started_at?: string;
  completed_at?: string;
  connector_id?: string;
  /** Evidence items produced by the check. */
  evidence: EvidenceItem[];
  /** Error message if failed. */
  error?: string;
  /** Whether step-up auth is required to view results. */
  step_up_required: boolean;
  /** Whether results are data-denied for the current user. */
  data_denied: boolean;
}

/** Evidence item from a check. */
export interface EvidenceItem {
  evidence_id: string;
  source: string;
  retrieved_at: string;
  /** Raw payload (capability-gated). */
  payload?: unknown;
  /** Summary visible to user. */
  summary: string;
  /** URL to original source. */
  source_url?: string;
  /** Whether this evidence is adverse. */
  adverse: boolean;
}

// --- Risks (docs/04 §5, docs/05) ---

export type RiskLevel = 'low' | 'medium' | 'high' | 'critical';
export type RiskDecision = 'accept' | 'reject' | 'escalate' | 'review';

/** A risk factor on an entity. */
export interface RiskFactor {
  factor_id: string;
  entity_public_id: string;
  criterion_id: string;
  /** Display label. */
  label: string;
  level: RiskLevel;
  /** Whether the factor is resolved. */
  resolved: boolean;
  /** When the factor was detected. */
  detected_at: string;
  /** When the factor was resolved. */
  resolved_at?: string;
  /** Related check IDs. */
  source_checks: string[];
  /** Explanation. */
  explanation: string;
}

/** A risk assessment on an entity. */
export interface RiskAssessment {
  assessment_id: string;
  entity_public_id: string;
  /** Overall risk level. */
  overall_level: RiskLevel;
  /** Factors included in this assessment. */
  factors: RiskFactor[];
  /** Assessor display name. */
  assessor: string;
  assessed_at: string;
  /** Decision. */
  decision: RiskDecision;
  /** Decision rationale. */
  rationale?: string;
  /** Whether approval is pending. */
  approval_pending: boolean;
  /** Approver display name. */
  approver?: string;
  approved_at?: string;
}

// --- Tasks / Workflow (docs/04 §6) ---

export type TaskStatus =
  | 'not_started'
  | 'in_progress'
  | 'waiting'
  | 'completed'
  | 'cancelled'
  | 'error';

export type TaskKind =
  | 'request_processing'
  | 'check_execution'
  | 'risk_assessment'
  | 'monitoring_review'
  | 'report_generation'
  | 'approval'
  | 'notification';

/** A workflow task. */
export interface WorkflowTask {
  task_id: string;
  entity_public_id?: string;
  request_id?: string;
  kind: TaskKind;
  status: TaskStatus;
  title: string;
  description?: string;
  /** Assigned user. */
  assignee?: string;
  /** Due date. */
  due_date?: string;
  /** Created at. */
  created_at: string;
  /** Updated at. */
  updated_at: string;
  /** Priority (1=high, 5=low). */
  priority: number;
  /** Sub-task IDs. */
  sub_tasks: string[];
  /** Whether this task is blocking. */
  blocking: boolean;
}

// --- Monitoring (docs/04 §7) ---

export type MonitorFrequency = 'daily' | 'weekly' | 'monthly' | 'quarterly' | 'annual';

/** A monitoring schedule on an entity. */
export interface MonitorSchedule {
  monitor_id: string;
  entity_public_id: string;
  frequency: MonitorFrequency;
  /** Checks to run on each cycle. */
  check_kinds: CheckKind[];
  /** Next run date. */
  next_run: string;
  /** Last run date. */
  last_run?: string;
  /** Whether the monitor is active. */
  active: boolean;
  /** Last result summary. */
  last_result?: MonitorResult;
}

/** Result of a monitoring cycle. */
export interface MonitorResult {
  run_id: string;
  ran_at: string;
  /** New factors detected. */
  new_factors: number;
  /** Checks completed. */
  checks_completed: number;
  /** Checks failed. */
  checks_failed: number;
  /** Whether escalation is needed. */
  escalate: boolean;
}

// --- Reports (docs/04 §8) ---

export type ReportStatus = 'draft' | 'generating' | 'ready' | 'failed' | 'expired';
export type ReportFormat = 'pdf' | 'xlsx' | 'docx' | 'json' | 'csv';

/** A generated report. */
export interface ReportInstance {
  report_id: string;
  entity_public_id: string;
  /** Report template ID. */
  template_id: string;
  title: string;
  status: ReportStatus;
  format: ReportFormat;
  /** When generated. */
  generated_at?: string;
  /** Download URL (capability-gated). */
  download_url?: string;
  /** File size in bytes. */
  size_bytes?: number;
  /** Expiry date for the download link. */
  expires_at?: string;
  /** Generated by. */
  generated_by: string;
}
