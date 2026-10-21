/**
 * Schema registry types (docs/03 §4-6, docs/12).
 *
 * Mirrors `enginer-domain::schema`.
 */

import type { EntityClass } from './entity.js';

/** Predicate direction (docs/03 §4). */
export type PredicateDirection = 'directed' | 'symmetric';

/** Schema revision lifecycle state (docs/12 §1). */
export type SchemaRevisionState =
  | 'draft'
  | 'validating'
  | 'impact_ready'
  | 'approved'
  | 'publishing'
  | 'published'
  | 'retired'
  | 'failed';

/** A predicate definition. */
export interface PredicateDefinition {
  predicate_id: string;
  stable_alias: string;
  is_edge: boolean;
  direction: PredicateDirection;
  domain_type_id?: string;
  range_type_id?: string;
  scalar_kind?: string;
  min_cardinality: number;
  max_cardinality?: number;
  participates_in_identity: boolean;
  confidentiality_class: string;
  normalizer_version?: string;
}

/** A predicate slot within a type definition. */
export interface TypePredicateSlot {
  predicate_id: string;
  required: boolean;
  min_cardinality: number;
  max_cardinality?: number;
  order?: number;
  group?: string;
}

/** A type definition (docs/03 §5). */
export interface TypeDefinition {
  type_id: string;
  stable_alias: string;
  entity_class: EntityClass;
  predicates: TypePredicateSlot[];
  identity_policy_ids: string[];
}

/** A schema revision (immutable once published). */
export interface SchemaRevision {
  revision_id: number;
  tenant_id: string;
  state: SchemaRevisionState;
  parent_revision_id?: number;
  semantic_version: string;
  content_hash: string;
  types: TypeDefinition[];
  predicates: PredicateDefinition[];
}

/** Compatibility classification produced by schema impact analysis. */
export type SchemaCompatibilityClass =
  | 'presentation_only'
  | 'additive'
  | 'conditionally_compatible'
  | 'semantic'
  | 'breaking'
  | 'meta_kernel';

/** Supported declarative change kinds in a schema draft. */
export type SchemaChangeKind =
  | 'add'
  | 'update'
  | 'retire'
  | 'remove'
  | 'rename'
  | 'reclassify';

/** Human-reviewable change included in a schema draft. */
export interface SchemaChange {
  kind: SchemaChangeKind;
  resource_kind: string;
  resource_id: string;
  summary: string;
  breaking: boolean;
}

/** Validation finding with a stable code and source path. */
export interface SchemaValidationFinding {
  severity: 'error' | 'warning' | 'info';
  code: string;
  message: string;
  path?: string | null;
}

/** Complete validation report for one draft version. */
export interface SchemaValidationReport {
  revision_id: string;
  valid: boolean;
  checked_at: string;
  findings: SchemaValidationFinding[];
}

/** Detailed field-level counts from impact analysis. */
export interface SchemaImpactDetails {
  new_fields: number;
  removed_fields: number;
  changed_fields: number;
  migration_required: boolean;
}

/** Compatibility and data-impact report required before publication. */
export interface SchemaImpactReport {
  revision_id: string;
  generated_at: string;
  compatibility: SchemaCompatibilityClass;
  breaking: boolean;
  affected_entities: number;
  affected_statements: number;
  details: SchemaImpactDetails;
  blockers: string[];
}

/** Draft or immutable revision as returned by the schema admin API. */
export interface SchemaRevisionRecord {
  revision_id: string;
  display_name: string;
  description: string;
  status: SchemaRevisionState;
  semantic_version: string;
  parent_revision_id?: string | null;
  version: number;
  etag: string;
  types: Record<string, unknown>[];
  predicates: Record<string, unknown>[];
  /** Relation group registry ({id, label, order}) used to group relations in the UI. */
  relation_groups?: Record<string, unknown>[];
  changes: SchemaChange[];
  type_count: number;
  predicate_count: number;
  created_at: string;
  updated_at: string;
  published_at?: string | null;
  retired_at?: string | null;
  content_hash?: string | null;
  validation?: SchemaValidationReport | null;
  impact?: SchemaImpactReport | null;
  publish_reason?: string | null;
  retire_reason?: string | null;
  /** Owning catalog alias — present on stored records, docs/28 §3. */
  catalog?: string;
}

/** Create a draft, optionally based on a published revision. */
export interface CreateSchemaDraftBody {
  name: string;
  description?: string;
  base_revision?: string;
  semantic_version?: string;
}

/** Replace editable draft content. */
export interface UpdateSchemaDraftBody {
  display_name: string;
  description: string;
  semantic_version: string;
  types: Record<string, unknown>[];
  predicates: Record<string, unknown>[];
  relation_groups?: Record<string, unknown>[];
  changes: SchemaChange[];
}

/** Result of deleting a non-published draft. */
export interface SchemaDraftTombstone {
  revision_id: string;
  deleted: boolean;
}

/** Background migration progress for a published revision. */
export interface SchemaMigrationStatus {
  revision_id: string;
  status: 'not_required' | 'pending' | 'running' | 'completed' | 'failed';
  migration_required: boolean;
  completed_steps: number;
  total_steps: number;
  issues: number;
  updated_at: string;
}
