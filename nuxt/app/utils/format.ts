/** Shared formatting helpers (auto-imported by Nuxt). */

/** ISO/RFC3339 timestamp → locale string; invalid input passes through. */
export function formatDateTime(value?: string | null): string {
  if (!value) return '—'
  const d = new Date(value)
  return Number.isNaN(d.getTime()) ? value : d.toLocaleString()
}

/**
 * Translated label for a server-supplied enum value. Prefixes are tried in
 * order; unknown keys fall back to the raw value so a newer backend (or
 * fuzzed data) never renders a literal `prefix.value` i18n key.
 */
export function enumLabel(
  t: (key: string) => string,
  te: (key: string) => boolean,
  prefixes: string | string[],
  value: string | null | undefined,
): string {
  if (!value) return '—'
  for (const prefix of Array.isArray(prefixes) ? prefixes : [prefixes]) {
    const key = `${prefix}.${value}`
    if (te(key)) return t(key)
  }
  return value
}

const STATUS_ERROR_KEYS: Record<number, string> = {
  400: 'bad_request',
  401: 'unauthorized',
  403: 'forbidden',
  404: 'not_found',
  409: 'conflict',
  412: 'precondition_failed',
  422: 'unprocessable_entity',
  429: 'too_many_requests',
  500: 'internal_error',
  503: 'service_unavailable',
}

const DETAIL_STATUSES = new Set([400, 409, 412, 422])

/**
 * User-visible text for a caught error. Server errors and locally thrown
 * sentinel errors may carry an i18n code as the message (e.g.
 * `errors.schema.validation_failed` or `schema.validation_failed`) —
 * translate it when a matching key exists, otherwise show the message.
 */
export function displayError(
  e: unknown,
  t: (key: string) => string,
  te: (key: string) => boolean,
): string {
  const { messageKey, statusCode, code, detail } = (e ?? {}) as {
    messageKey?: string
    statusCode?: number
    code?: string
    detail?: string
  }
  if (messageKey) {
    const specific = te(messageKey)
    const genericKey = statusCode && STATUS_ERROR_KEYS[statusCode] ? `errors.${STATUS_ERROR_KEYS[statusCode]}` : ''
    const label = specific ? t(messageKey) : genericKey && te(genericKey) ? t(genericKey) : ''
    if (label) {
      if (detail && statusCode && DETAIL_STATUSES.has(statusCode)) return `${label}: ${detail}`
      return !specific && code ? `${label} (${code})` : label
    }
  }
  const msg = e instanceof Error ? e.message : typeof e === 'string' ? e : ''
  if (te(msg)) return t(msg)
  const prefixed = `errors.${msg}`
  if (te(prefixed)) return t(prefixed)
  return msg || t('common.error')
}
