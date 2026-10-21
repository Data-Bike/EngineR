import { describe, expect, it, vi } from 'vitest';
import ru from '../i18n/locales/ru';
import en from '../i18n/locales/en';
import ar from '../i18n/locales/ar';
import he from '../i18n/locales/he';
import fr from '../i18n/locales/fr';
import de from '../i18n/locales/de';
import es from '../i18n/locales/es';
import zh from '../i18n/locales/zh';

const locales = { ru, en, ar, he, fr, de, es, zh };

function leafKeys(obj: object, prefix = ''): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    v && typeof v === 'object' ? leafKeys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

describe('i18n keys resolve through vue-i18n', () => {
  it('every leaf key of every locale is reachable by its dotted path', async () => {
    const { createI18n } = await vi.importActual<typeof import('vue-i18n')>('vue-i18n');
    const i18n = createI18n({ legacy: false, locale: 'ru', messages: locales as never });
    const unresolved: string[] = [];
    for (const [code, messages] of Object.entries(locales)) {
      for (const key of leafKeys(messages)) {
        if (!i18n.global.te(key, code)) unresolved.push(`${code}: ${key}`);
      }
    }
    expect(unresolved).toEqual([]);
  });
});
