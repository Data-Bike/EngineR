/**
 * UI metadata types: ViewDefinition, FieldKind, FieldRegistry (docs/16 §6).
 *
 * These types drive the metadata-driven UI — EntityFormRenderer and
 * EntityListRenderer consume them to render forms and tables without
 * hardcoded field knowledge.
 */

/** Scalar field kinds supported by the field registry (docs/16 §6). */
export type FieldKind =
  | 'text'
  | 'textarea'
  | 'rich_text'
  | 'boolean'
  | 'integer'
  | 'decimal'
  | 'money'
  | 'measurement'
  | 'date'
  | 'time'
  | 'datetime'
  | 'local_datetime'
  | 'timezone'
  | 'duration'
  | 'percent'
  | 'url'
  | 'uri'
  | 'email'
  | 'phone'
  | 'ltext'
  | 'uuid'
  | 'enum'
  | 'dictionary'
  | 'tags'
  | 'json'
  | 'geo'
  | 'blob'
  | 'entity_reference'
  | 'multi_value'
  | 'repeating_group'
  | 'address'
  | 'contact'
  | 'document'
  | 'file'
  | 'derived'
  | 'relation_editor';

/** A field definition in a view or form. */
export interface FieldDefinition {
  /** Predicate ID (UUID) or system field name. */
  predicate_id: string;
  /** Stable alias for column/label i18n key. */
  alias: string;
  /** Display label i18n key (e.g. `fields.company.name`). */
  label_key: string;
  /** Field kind for renderer selection. */
  kind: FieldKind;
  /** Whether the field is required. */
  required: boolean;
  /** Whether the field is read-only (derived/system). */
  read_only: boolean;
  /** Whether the field participates in identity (shows raw/canonical preview). */
  identity: boolean;
  /** Whether the field is hidden for the current user (server-determined). */
  hidden: boolean;
  /** Min cardinality (for multi-value / repeating groups). */
  min_cardinality: number;
  /** Max cardinality (undefined = unbounded). */
  max_cardinality?: number;
  /** Order in the form/list. */
  order: number;
  /** Group/section label key for form grouping. */
  group?: string;
  /** Enum/dictionary options (for enum/dictionary kinds). */
  options?: FieldOption[];
  /** Entity reference target type ID (for entity_reference kind). */
  target_type_id?: string;
  /** Validation constraints. */
  validation?: FieldValidation;
  /** Help text i18n key. */
  help_key?: string;
  /** Placeholder i18n key. */
  placeholder_key?: string;
}

/** Enum/dictionary option. */
export interface FieldOption {
  value: string;
  label_key: string;
}

/** Validation constraints for a field. */
export interface FieldValidation {
  min?: number;
  max?: number;
  pattern?: string;
  min_length?: number;
  max_length?: number;
  currency?: string;
  unit?: string;
  precision?: number;
}

/** A view section (groups fields in the form/card). */
export interface ViewSection {
  /** Section label i18n key. */
  label_key: string;
  /** Section order. */
  order: number;
  /** Fields in this section. */
  fields: FieldDefinition[];
  /** Whether the section is collapsible. */
  collapsible: boolean;
  /** Whether the section starts collapsed. */
  default_collapsed: boolean;
}

/** A view definition — drives both form and list rendering (docs/16 §6). */
export interface ViewDefinition {
  /** View ID. */
  view_id: string;
  /** View type: form, list, card. */
  view_type: 'form' | 'list' | 'card';
  /** Target type ID. */
  type_id: string;
  /** Schema revision. */
  schema_revision: number;
  /** View name i18n key. */
  name_key: string;
  /** Sections (for form/card views). */
  sections: ViewSection[];
  /** Columns (for list views). */
  columns: FieldDefinition[];
  /** Default sort keys. */
  default_sort: SortKey[];
  /** Available filters. */
  filters: FieldDefinition[];
  /** Available facets. */
  facets: FieldDefinition[];
  /** Row actions (capability-gated). */
  row_actions: RowAction[];
  /** Bulk actions (capability-gated). */
  bulk_actions: RowAction[];
  /** Whether export is allowed. */
  export_allowed: boolean;
}

/** Sort key for view definitions. */
export interface SortKey {
  field: string;
  direction: 'asc' | 'desc';
}

/** Row/bulk action. */
export interface RowAction {
  action_id: string;
  label_key: string;
  icon: string;
  capability: string;
}

/** Field value input for form submission. */
export interface FieldValueInput {
  predicate_id: string;
  value: unknown;
}

/** Form submission body. */
export interface FormSubmission {
  type_id: string;
  schema_revision: number;
  identity_mode: 'return_existing' | 'reject' | 'create_candidate';
  fields: FieldValueInput[];
}

/** Form validation error from server (docs/16 §11). */
export interface FieldValidationError {
  predicate_id: string;
  row_path?: string;
  code: string;
  message_key: string;
  parameters?: Record<string, unknown>;
}
