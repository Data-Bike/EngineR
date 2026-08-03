/**
 * Check, Assessment, Workflow, and Task API endpoints (docs/06 §8, docs/14
 * §8-10, docs/13 §7-8).
 */

import type { ApiClient, ParsedResponse } from '../client/ApiClient.js';
import type {
  CheckRequestBody,
  CheckResponse,
  EvidenceResponse,
  AssessmentResponse,
  TriggerAssessmentBody,
  FactorSummary,
  StartWorkflowBody,
  WorkflowResponse,
  TransitionBody,
  TaskResponse,
  CompleteTaskBody,
  CancelTaskBody,
} from '../types/index.js';

/** Check + Assessment + Workflow + Task API endpoint methods. */
export class CheckEndpoints {
  constructor(private readonly client: ApiClient) {}

  // --- Check ---

  /** POST /checks — request a check. */
  async requestCheck(body: CheckRequestBody): Promise<ParsedResponse<CheckResponse>> {
    return this.client.post<CheckResponse>('/checks', body);
  }

  /** GET /checks/{id} — get check status. */
  async getCheck(id: string): Promise<ParsedResponse<CheckResponse>> {
    return this.client.get<CheckResponse>(`/checks/${id}`);
  }

  /** GET /checks/{id}/evidence — get check evidence. */
  async getCheckEvidence(id: string): Promise<ParsedResponse<EvidenceResponse>> {
    return this.client.get<EvidenceResponse>(`/checks/${id}/evidence`);
  }

  // --- Assessment ---

  /** GET /entities/{id}/assessment — get latest assessment. */
  async getAssessment(entityId: string): Promise<ParsedResponse<AssessmentResponse>> {
    return this.client.get<AssessmentResponse>(`/entities/${entityId}/assessment`);
  }

  /** POST /entities/{id}/assessment — trigger assessment. */
  async triggerAssessment(
    entityId: string,
    body: TriggerAssessmentBody,
  ): Promise<ParsedResponse<AssessmentResponse>> {
    return this.client.post<AssessmentResponse>(`/entities/${entityId}/assessment`, body);
  }

  /** GET /entities/{id}/factors — get risk factors. */
  async getFactors(entityId: string): Promise<ParsedResponse<FactorSummary[]>> {
    return this.client.get<FactorSummary[]>(`/entities/${entityId}/factors`);
  }

  // --- Workflow ---

  /** POST /workflows — start a workflow. */
  async startWorkflow(body: StartWorkflowBody): Promise<ParsedResponse<WorkflowResponse>> {
    return this.client.post<WorkflowResponse>('/workflows', body);
  }

  /** GET /workflows/{id} — get workflow status. */
  async getWorkflow(id: string): Promise<ParsedResponse<WorkflowResponse>> {
    return this.client.get<WorkflowResponse>(`/workflows/${id}`);
  }

  /** POST /workflows/{id}/transition — transition a workflow. */
  async transitionWorkflow(
    id: string,
    body: TransitionBody,
  ): Promise<ParsedResponse<WorkflowResponse>> {
    return this.client.post<WorkflowResponse>(`/workflows/${id}/transition`, body);
  }

  /** GET /workflows/{id}/tasks — list workflow tasks. */
  async listWorkflowTasks(id: string): Promise<ParsedResponse<TaskResponse[]>> {
    return this.client.get<TaskResponse[]>(`/workflows/${id}/tasks`);
  }

  // --- Task ---

  /** POST /tasks/{id}/complete — complete a task. */
  async completeTask(
    id: string,
    body: CompleteTaskBody,
  ): Promise<ParsedResponse<TaskResponse>> {
    return this.client.post<TaskResponse>(`/tasks/${id}/complete`, body);
  }

  /** POST /tasks/{id}/cancel — cancel a task. */
  async cancelTask(
    id: string,
    body: CancelTaskBody,
  ): Promise<ParsedResponse<TaskResponse>> {
    return this.client.post<TaskResponse>(`/tasks/${id}/cancel`, body);
  }
}
