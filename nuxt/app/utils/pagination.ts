/** Keyset cursor for the next page from a list response's `meta`, or null. */
export function nextCursorOf(meta: unknown): string | null {
  const cursor = (meta as { next_cursor?: unknown } | null | undefined)?.next_cursor
  return cursor === undefined || cursor === null || cursor === '' ? null : String(cursor)
}
