/**
 * Query engine AST types (docs/07 §2-3).
 *
 * Mirrors `enginer-query-engine::ast`.
 */

/** System fields queryable without a statement join. */
export type SystemField = 'public_id' | 'entity_id' | 'created_at' | 'status' | 'revision';

/** A field reference: predicate id (data) or system field. */
export type FieldRef =
  | { predicate_id: string }
  | { system: SystemField };

/** Comparison operator (docs/07 §3). */
export type Operator =
  | 'eq'
  | 'ne'
  | 'in'
  | 'is_null'
  | 'is_not_null'
  | 'lt'
  | 'lte'
  | 'gt'
  | 'gte'
  | 'between'
  | 'contains'
  | 'prefix'
  | 'full_text'
  | 'valid_at'
  | 'relation_exists'
  | 'relation_not_exists';

/** Relation direction. */
export type RelationDirection = 'outgoing' | 'incoming' | 'both';

/** A where expression tree. */
export type WhereExpr =
  | {
      comparison: {
        field: FieldRef;
        op: Operator;
        value?: unknown;
        values?: unknown[];
        valid_at?: string;
      };
    }
  | {
      relation: {
        predicate_id: string;
        direction: RelationDirection;
        target_type_id?: string;
        filter?: WhereExpr;
      };
    }
  | { and: WhereExpr[] }
  | { or: WhereExpr[] }
  | { not: WhereExpr };

/** A select field. */
export interface SelectField {
  field: FieldRef;
  alias: string;
}

/** Sort direction. */
export type SortDirection = 'asc' | 'desc';

/** A sort key. */
export interface SortKey {
  field: FieldRef;
  direction: SortDirection;
}

/** Keyset page. */
export interface KeysetPage {
  size: number;
  after?: string;
}

/** The `from` clause: root type + alias. */
export interface FromClause {
  type_id: string;
  alias: string;
}

/** The root query AST (docs/07 §2). */
export interface QueryAst {
  from: FromClause;
  select: SelectField[];
  where?: WhereExpr;
  sort: SortKey[];
  page: KeysetPage;
  as_of?: string;
}

/** Query request body (docs/06 §9). */
export interface QueryBody {
  query: QueryAst;
}

/** Query response (docs/06 §9). */
export interface QueryResponse {
  rows: Record<string, unknown>[];
  cursor?: string;
}
