/**
 * API envelope types (docs/06 §2).
 *
 * Mirrors `enginer-contracts::envelope`.
 */

/** Success envelope: `{ data, meta, links }`. */
export interface ApiEnvelope<T> {
  data: T;
  meta: ApiMeta;
  links?: ApiLinks;
}

/** Response metadata. */
export interface ApiMeta {
  request_id: string;
  schema_revision?: string;
  commit_revision?: string;
  policy_revision?: string;
}

/** Pagination / navigation links. */
export interface ApiLinks {
  next?: string;
  prev?: string;
}
