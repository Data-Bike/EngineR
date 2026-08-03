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
