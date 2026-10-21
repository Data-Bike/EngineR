/**
 * Entity domain types (docs/03 §5, docs/04 §10-11, docs/06 §6).
 *
 * Mirrors `enginer-domain::entity`, `enginer-domain::statement`,
 * `enginer-domain::identity`, and the entity route DTOs in `apps/api`.
 */

/** Entity lifecycle status (docs/10 §5). */
export type EntityStatus = 'active' | 'retired';

/** Entity class (docs/03 §5). */
export type EntityClass = 'data' | 'schema' | 'policy' | 'workflow' | 'system';

/** Statement status (docs/10 §6). */
export type StatementStatus = 'active' | 'superseded' | 'revoked' | 'quarantined';

/** Identity mode (docs/04 §6). */
export type IdentityMode = 'exact' | 'fuzzy';

/** Dedup outcome (docs/04 §10). */
export type DedupOutcome = 'created' | 'deduplicated';

/** Canonical scalar kind (docs/03 §7). */
export type ScalarKind =
  | 'text'
  | 'localized_text'
  | 'boolean'
  | 'integer'
  | 'decimal'
  | 'date'
  | 'instant'
  | 'local_datetime'
  | 'uuid'
  | 'uri'
  | 'email'
  | 'phone'
  | 'money'
  | 'measurement'
  | 'geo'
  | 'json'
  | 'blob';

/** A canonical scalar value (docs/04 §7). The `kind` tag discriminates. */
export type CanonicalScalar =
  | { kind: 'text'; value_nfc: string; language_tag?: string }
  | { kind: 'boolean'; value: boolean }
  | { kind: 'integer'; value: string } // i128 as string to avoid JS precision loss
  | { kind: 'decimal'; value: string }
  | { kind: 'date'; value: string }
  | { kind: 'instant'; value: string; source_zone?: string }
  | { kind: 'local_datetime'; value: string; zone: string }
  | { kind: 'uuid'; value: string }
  | { kind: 'uri'; value: string }
  | { kind: 'email'; local: string; domain: string }
  | { kind: 'phone'; e164: string; extension?: string }
  | { kind: 'money'; amount: string; currency: string }
  | { kind: 'measurement'; amount: string; unit: string }
  | { kind: 'geo'; value: string } // canonical GeoJSON text
  | { kind: 'json'; value: string } // canonical JSON text
  | { kind: 'blob'; content_hash: string; size_bytes: number; mime: string };

/** Statement object: entity (edge) or scalar (property). */
export type StatementObject =
  | { object_kind: 'entity'; entity: string } // entity public id (UUID)
  | { object_kind: 'scalar' } & CanonicalScalar;

/** A statement (logical atom, docs/03 §3). */
export interface Statement {
  id: string;
  tenant_id: string;
  subject: string; // entity public id
  predicate: string; // predicate id (UUID)
  object: StatementObject;
  validity: { valid_from: string; valid_to?: string };
  recorded: string;
  provenance: ProvenanceRef;
  confidence: string;
  status: StatementStatus;
  revision: number;
  fingerprint: string;
  supersedes_statement_id?: string;
  revoked_by_statement_id?: string;
}

/** Provenance reference (docs/03 §8). */
export interface ProvenanceRef {
  source?: SourceRef;
  fetched_at?: string;
  effective_at?: string;
  raw_payload_hash?: string;
  legal_basis?: string;
  parser_version?: string;
  actor?: string;
  job?: string;
  correlation?: string;
  confidence?: string;
  verification?: string;
}

/** Source record reference. */
export interface SourceRef {
  source_system_id: string;
  namespace: string;
  external_id: string;
  observation_scope: string;
}

/** Entity aggregate (docs/03 §5). */
export interface Entity {
  tenant_id: string;
  entity_id: number;
  public_id: string;
  type_id: string;
  entity_class: EntityClass;
  revision: number;
  /** Human-readable title resolved from the type's display fields. */
  display_name?: string;
  status: EntityStatus;
  created_at: string;
  updated_at?: string;
  /** Current materialized predicate values used by metadata-driven forms. */
  fields?: Record<string, unknown>;
  created_by?: string;
  retired_at?: string;
  retired_by?: string;
  retire_reason?: string;
  canonical_entity_id?: number;
  classification_id?: string;
  business_unit_id?: string;
  /** Owning catalog alias (docs/28) — entities never change catalogs. */
  catalog?: string;
}

// --- Entity API DTOs (docs/06 §6) ---

/** Create entity request body. */
export interface CreateEntityBody {
  type_id: string;
  schema_revision: number;
  identity_mode: 'return_existing' | 'reject' | 'create_candidate';
  statements: StatementInput[];
}

/** A statement input in the create body. */
export interface StatementInput {
  predicate_id: string;
  value: StatementValueInput | RawStatementValue;
}

/** A raw form value — the server coerces it by the declared field kind. */
export type RawStatementValue = string | number | boolean | null | { [key: string]: unknown } | unknown[];

/** The value side of a statement input (docs/06 §6). */
export type StatementValueInput =
  | { kind: 'text'; value: string }
  | { kind: 'integer'; value: string }
  | { kind: 'boolean'; value: boolean }
  | { kind: 'email'; local: string; domain: string }
  | { kind: 'phone'; e164: string; extension?: string }
  | { kind: 'uuid'; value: string }
  | { kind: 'entity'; entity_public_id: string };

/** Create entity response. */
export interface CreateEntityResponse {
  public_id: string;
  entity_id: number;
  revision: number;
  outcome: DedupOutcome;
}

/** Replace or add materialized predicate values on an entity revision. */
export interface UpdateEntityBody {
  revision: number;
  fields: StatementInput[];
}

/** Node returned by progressive graph expansion. */
export interface EntityGraphNode {
  id: string;
  label: string;
  type_label: string;
  type_id: string;
  status: EntityStatus;
  has_children: boolean;
  cycle: boolean;
}

/** Edge returned by progressive graph expansion. */
export interface EntityGraphEdge {
  id: string;
  source: string;
  target: string;
  predicate: string;
  label: string;
  cycle: boolean;
}

export interface EntityGraphResponse {
  nodes: EntityGraphNode[];
  edges: EntityGraphEdge[];
}

/** One preview field on a relation target card (`RelationPreviewField`). */
export interface RelationPreviewField {
  field: string;
  value: unknown;
}

/** An onward edge of a trans-object (`RelationThroughEdge`). */
export interface RelationThroughEdge {
  relation_id: string;
  predicate: string;
  /** Direction relative to the trans-object. */
  direction: 'outgoing' | 'incoming' | string;
  target_entity_id: string;
  target_display_name: string;
  target_type: string;
  preview?: RelationPreviewField[];
}

/** A single relation row (`RelationRecord`). */
export interface EntityRelation {
  relation_id: string;
  predicate: string;
  direction: 'outgoing' | 'incoming' | string;
  target_entity_id: string;
  target_display_name: string;
  target_type: string;
  valid_from: string;
  valid_to: string | null;
  source: string;
  qualifiers: Record<string, unknown>;
  /** Relation group id from the predicate's `group` schema attribute. */
  group?: string;
  /** Ordered preview fields of the target (the type's `preview` view). */
  preview?: RelationPreviewField[];
  /** True when the target is a schema-declared trans-object. */
  trans?: boolean;
  /** The trans-object's other edges (one hop). */
  through?: RelationThroughEdge[];
}

/** Retire entity request body. */
export interface RetireEntityBody {
  reason: string;
}

/** Merge entity request body. */
export interface MergeEntityBody {
  loser_public_id: string;
  reason: string;
}

/** Retire entity response. */
export interface RetireEntityResponse {
  public_id: string;
  revision: number;
  status: EntityStatus;
}

/** Merge entity response. */
export interface MergeEntityResponse {
  canonical_public_id: string;
  loser_public_id: string;
  canonical_revision: number;
  idempotent: boolean;
}
