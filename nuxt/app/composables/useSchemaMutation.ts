import type { SchemaChange } from '@enginer/api-client';

export interface SchemaMutationOptions {
  /** Human-readable draft name, e.g. "Изменение типа person". */
  name: string;
  /** Reason recorded on the published revision. */
  reason: string;
  /** Change-log entries for validation/impact reporting. */
  changes: SchemaChange[];
  /** Mutate copies of the draft's types/predicates/relation_groups arrays in place. */
  mutate: (
    types: Record<string, unknown>[],
    predicates: Record<string, unknown>[],
    relationGroups: Record<string, unknown>[],
  ) => void;
}

/**
 * Apply a catalog change (type/predicate add/update/remove) through the
 * versioned schema lifecycle: draft → update → validate → impact → publish.
 * There is intentionally no direct `POST /schemas/types` endpoint — every
 * catalog mutation must produce an auditable published revision.
 *
 * If validation or publish fails, the draft is left in place so the user can
 * inspect findings on /admin/schema.
 */
export function useSchemaMutation() {
  const api = useApi();

  async function applySchemaMutation(options: SchemaMutationOptions): Promise<string> {
    const created = await api.schemas.createDraft({
      name: options.name,
      description: options.name,
    });
    const draftId = created.data.revision_id;

    const draft = await api.schemas.getDraft(draftId);
    const types = (draft.data.types ?? []) as Record<string, unknown>[];
    const predicates = (draft.data.predicates ?? []) as Record<string, unknown>[];
    const relationGroups = (draft.data.relation_groups ?? []) as Record<string, unknown>[];
    options.mutate(types, predicates, relationGroups);

    await api.schemas.updateDraft(
      draftId,
      {
        display_name: options.name,
        description: options.name,
        semantic_version: draft.data.semantic_version,
        types,
        predicates,
        relation_groups: relationGroups,
        changes: options.changes,
      },
      draft.data.etag,
    );

    const validation = await api.schemas.validateDraft(draftId);
    if (!validation.data.valid) {
      throw new Error('schema.validation_failed');
    }
    await api.schemas.impactDraft(draftId);

    // validate/impact bump the draft version — fetch the fresh etag.
    const fresh = await api.schemas.getDraft(draftId);
    const published = await api.schemas.publishDraft(draftId, fresh.data.etag, options.reason);
    return published.data.revision_id;
  }

  return { applySchemaMutation };
}
