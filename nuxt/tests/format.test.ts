import { describe, expect, it } from 'vitest';
import { displayError, enumLabel, formatDateTime } from '../app/utils/format';

describe('formatDateTime', () => {
  it('formats an ISO timestamp to a locale string', () => {
    const out = formatDateTime('2026-10-21T01:46:39.088162+00:00');
    expect(out).not.toBe('2026-10-21T01:46:39.088162+00:00');
    expect(out).not.toBe('—');
    expect(out.length).toBeGreaterThan(5);
  });

  it('returns — for empty input', () => {
    expect(formatDateTime(null)).toBe('—');
    expect(formatDateTime(undefined)).toBe('—');
    expect(formatDateTime('')).toBe('—');
  });

  it('passes through unparseable values', () => {
    expect(formatDateTime('not-a-date')).toBe('not-a-date');
  });
});

describe('enumLabel', () => {
  const dict: Record<string, string> = {
    'checks.kind.kyc': 'KYC',
    'checks.status.pending': 'Pending',
  };
  const t = (k: string) => dict[k] ?? k;
  const te = (k: string) => k in dict;

  it('translates known keys', () => {
    expect(enumLabel(t, te, 'checks.kind', 'kyc')).toBe('KYC');
  });

  it('falls back to the raw value for unknown keys', () => {
    expect(enumLabel(t, te, 'checks.kind', '__bogus__')).toBe('__bogus__');
    expect(enumLabel(t, te, 'checks.kind', 'future_kind')).toBe('future_kind');
  });

  it('tries prefixes in order', () => {
    expect(enumLabel(t, te, ['entity.checks.kind', 'checks.kind'], 'kyc')).toBe('KYC');
    expect(enumLabel(t, te, ['nope.kind', 'checks.status'], 'pending')).toBe('Pending');
  });

  it('returns — for empty values', () => {
    expect(enumLabel(t, te, 'checks.kind', '')).toBe('—');
    expect(enumLabel(t, te, 'checks.kind', null)).toBe('—');
  });
});

describe('displayError', () => {
  const dict: Record<string, string> = {
    'errors.not_found': 'Не найдено',
    'errors.conflict': 'Конфликт',
    'errors.unprocessable_entity': 'Ошибка валидации',
    'errors.schema.validation_failed': 'Схема не прошла проверку',
    'common.upload_requires_https': 'Нужен HTTPS',
    'common.error': 'Ошибка',
  };
  const t = (k: string) => dict[k] ?? k;
  const te = (k: string) => k in dict;
  const apiError = (statusCode: number, code: string, message: string, detail?: string) =>
    Object.assign(new Error(message), { messageKey: `errors.${code}`, statusCode, code, detail });

  it('prefers the translated server message key over the raw server text', () => {
    expect(displayError(apiError(404, 'not_found', 'entity not found', 'entity not found'), t, te)).toBe('Не найдено');
  });

  it('keeps the specific server detail for validation and conflict errors', () => {
    expect(
      displayError(apiError(422, 'unprocessable_entity', 'required fields missing: name', 'required fields missing: name'), t, te),
    ).toBe('Ошибка валидации: required fields missing: name');
    expect(
      displayError(apiError(409, 'catalog.retired', 'Каталог погашен', 'Каталог погашен'), t, te),
    ).toBe('Конфликт: Каталог погашен');
  });

  it('names the untranslated machine code next to the status label', () => {
    expect(
      displayError(apiError(422, 'due_date.invalid', 'Запрос не прошёл валидацию'), t, te),
    ).toBe('Ошибка валидации (due_date.invalid)');
  });

  it('shows a translated specific code as is', () => {
    expect(
      displayError(apiError(422, 'schema.validation_failed', 'Запрос не прошёл валидацию'), t, te),
    ).toBe('Схема не прошла проверку');
  });

  it('translates i18n-keyed sentinel errors', () => {
    expect(displayError(new Error('common.upload_requires_https'), t, te)).toBe('Нужен HTTPS');
    expect(displayError(new Error('schema.validation_failed'), t, te)).toBe('Схема не прошла проверку');
  });

  it('returns the raw message when nothing matches', () => {
    expect(displayError(new Error('Network down'), t, te)).toBe('Network down');
    expect(displayError('plain text', t, te)).toBe('plain text');
  });

  it('falls back to the generic error label for values without a message', () => {
    expect(displayError({ unexpected: true }, t, te)).toBe('Ошибка');
    expect(displayError(null, t, te)).toBe('Ошибка');
  });
});
