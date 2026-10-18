import { describe, it, expect } from 'vitest';
import ru from '../i18n/locales/ru';
import en from '../i18n/locales/en';
import ar from '../i18n/locales/ar';
import he from '../i18n/locales/he';
import fr from '../i18n/locales/fr';
import de from '../i18n/locales/de';
import es from '../i18n/locales/es';
import zh from '../i18n/locales/zh';

const allLocales: Record<string, typeof ru> = {
  ru,
  en,
  ar,
  he,
  fr,
  de,
  es,
  zh,
};

const localeNames: Record<string, string> = {
  ru: 'Русский',
  en: 'English',
  ar: 'العربية',
  he: 'עברית',
  fr: 'Français',
  de: 'Deutsch',
  es: 'Español',
  zh: '中文',
};

describe('i18n locale files', () => {
  // Test each locale individually for clear failure messages
  for (const [code, locale] of Object.entries(allLocales)) {
    describe(`locale: ${code} (${localeNames[code]})`, () => {
      it('has all required top-level keys', () => {
        expect(locale.app).toBeDefined();
        expect(locale.nav).toBeDefined();
        expect(locale.auth).toBeDefined();
        expect(locale.entity).toBeDefined();
        expect(locale.schema).toBeDefined();
        expect(locale.common).toBeDefined();
        expect(locale.errors).toBeDefined();
      });

      it('has app.name and app.tagline', () => {
        expect(locale.app.name).toBe('EngineR');
        expect(locale.app.tagline).toBeTruthy();
        expect(locale.app.tagline.length).toBeGreaterThan(5);
      });

      it('has all 11 nav keys', () => {
        const navKeys = Object.keys(locale.nav);
        expect(navKeys).toContain('home');
        expect(navKeys).toContain('objects');
        expect(navKeys).toContain('requests');
        expect(navKeys).toContain('checks');
        expect(navKeys).toContain('monitoring');
        expect(navKeys).toContain('risks');
        expect(navKeys).toContain('reports');
        expect(navKeys).toContain('docs');
        expect(navKeys).toContain('admin');
        expect(navKeys).toContain('integrations');
        expect(navKeys).toContain('audit');
        expect(navKeys.length).toBe(11);
      });

      it('has all auth keys', () => {
        expect(locale.auth.login).toBeTruthy();
        expect(locale.auth.logout).toBeTruthy();
        expect(locale.auth.username).toBeTruthy();
        expect(locale.auth.password).toBeTruthy();
        expect(locale.auth.submit).toBeTruthy();
        expect(locale.auth.error).toBeTruthy();
        expect(locale.auth.step_up_required).toBeTruthy();
      });

      it('has all 8 entity tab keys', () => {
        const tabs = Object.keys(locale.entity.tabs);
        expect(tabs).toContain('overview');
        expect(tabs).toContain('relations');
        expect(tabs).toContain('graph');
        expect(tabs).toContain('checks');
        expect(tabs).toContain('risks');
        expect(tabs).toContain('tasks');
        expect(tabs).toContain('documents');
        expect(tabs).toContain('history');
        expect(tabs.length).toBe(8);
      });

      it('has entity status keys', () => {
        expect(locale.entity.status.active).toBeTruthy();
        expect(locale.entity.status.retired).toBeTruthy();
      });

      it('has all entity action keys', () => {
        expect(locale.entity.create).toBeTruthy();
        expect(locale.entity.search).toBeTruthy();
        expect(locale.entity.advanced_search).toBeTruthy();
        expect(locale.entity.edit).toBeTruthy();
        expect(locale.entity.retire).toBeTruthy();
        expect(locale.entity.merge).toBeTruthy();
      });

      it('has all schema keys', () => {
        expect(locale.schema.title).toBeTruthy();
        expect(locale.schema.drafts).toBeTruthy();
        expect(locale.schema.published).toBeTruthy();
        expect(locale.schema.create_draft).toBeTruthy();
        expect(locale.schema.validate).toBeTruthy();
        expect(locale.schema.impact).toBeTruthy();
        expect(locale.schema.publish).toBeTruthy();
        expect(locale.schema.retire).toBeTruthy();
        expect(locale.schema.types).toBeTruthy();
        expect(locale.schema.predicates).toBeTruthy();
        expect(locale.schema.description).toBeTruthy();
        expect(locale.schema.status.impact_ready).toBeTruthy();
        expect(locale.schema.compatibility.breaking).toBeTruthy();
        expect(locale.schema.publish_acknowledgement).toBeTruthy();
        expect(locale.schema.audit_reason).toBeTruthy();
      });

      it('has all common keys', () => {
        expect(locale.common.save).toBeTruthy();
        expect(locale.common.cancel).toBeTruthy();
        expect(locale.common.delete).toBeTruthy();
        expect(locale.common.confirm).toBeTruthy();
        expect(locale.common.close).toBeTruthy();
        expect(locale.common.yes).toBeTruthy();
        expect(locale.common.no).toBeTruthy();
        expect(locale.common.loading).toBeTruthy();
        expect(locale.common.error).toBeTruthy();
        expect(locale.common.retry).toBeTruthy();
        expect(locale.common.request_id).toBeTruthy();
      });

      it('has all 33 error code keys', () => {
        const errorKeys = Object.keys(locale.errors);
        // Generic (10)
        expect(errorKeys).toContain('bad_request');
        expect(errorKeys).toContain('unauthorized');
        expect(errorKeys).toContain('forbidden');
        expect(errorKeys).toContain('not_found');
        expect(errorKeys).toContain('conflict');
        expect(errorKeys).toContain('precondition_failed');
        expect(errorKeys).toContain('unprocessable_entity');
        expect(errorKeys).toContain('too_many_requests');
        expect(errorKeys).toContain('internal_error');
        expect(errorKeys).toContain('service_unavailable');
        // Domain (3)
        expect(errorKeys).toContain('entity.version_conflict');
        expect(errorKeys).toContain('identity.conflict');
        expect(errorKeys).toContain('idempotency.conflict');
        // Schema (2)
        expect(errorKeys).toContain('schema.state_invalid');
        expect(errorKeys).toContain('schema.validation_failed');
        // Query (11)
        expect(errorKeys).toContain('query.syntax_invalid');
        expect(errorKeys).toContain('query.unknown_type');
        expect(errorKeys).toContain('query.unknown_predicate');
        expect(errorKeys).toContain('query.operator_not_supported');
        expect(errorKeys).toContain('query.field_forbidden');
        expect(errorKeys).toContain('query.too_complex');
        expect(errorKeys).toContain('query.requires_batch');
        expect(errorKeys).toContain('query.cursor_invalid');
        expect(errorKeys).toContain('query.cursor_stale');
        expect(errorKeys).toContain('query.projection_lag');
        expect(errorKeys).toContain('query.timeout');
        // Security (7)
        expect(errorKeys).toContain('security.unauthenticated');
        expect(errorKeys).toContain('security.capability_denied');
        expect(errorKeys).toContain('security.data_denied');
        expect(errorKeys).toContain('security.step_up_required');
        expect(errorKeys).toContain('security.policy_stale');
        expect(errorKeys).toContain('security.pdp_unavailable');
        expect(errorKeys).toContain('security.context_invalid');
        expect(errorKeys.length).toBe(33);
      });
    });
  }

  it('all locales have identical key structure', () => {
    const ruKeys = JSON.stringify(Object.keys(ru).sort());
    for (const [code, locale] of Object.entries(allLocales)) {
      const localeKeys = JSON.stringify(Object.keys(locale).sort());
      expect(localeKeys, `locale ${code} key structure mismatch`).toBe(ruKeys);
    }
  });

  it('all locales have identical error code keys', () => {
    const ruErrorKeys = JSON.stringify(Object.keys(ru.errors).sort());
    for (const [code, locale] of Object.entries(allLocales)) {
      const errorKeys = JSON.stringify(Object.keys(locale.errors).sort());
      expect(errorKeys, `locale ${code} error keys mismatch`).toBe(ruErrorKeys);
    }
  });

  it('all locales have identical nav keys', () => {
    const ruNavKeys = JSON.stringify(Object.keys(ru.nav).sort());
    for (const [code, locale] of Object.entries(allLocales)) {
      const navKeys = JSON.stringify(Object.keys(locale.nav).sort());
      expect(navKeys, `locale ${code} nav keys mismatch`).toBe(ruNavKeys);
    }
  });

  it('all locales have identical entity tab keys', () => {
    const ruTabs = JSON.stringify(Object.keys(ru.entity.tabs).sort());
    for (const [code, locale] of Object.entries(allLocales)) {
      const tabs = JSON.stringify(Object.keys(locale.entity.tabs).sort());
      expect(tabs, `locale ${code} entity tab keys mismatch`).toBe(ruTabs);
    }
  });

  it('all locale values are non-empty strings', () => {
    function checkNonEmpty(obj: Record<string, unknown>, path: string) {
      for (const [key, value] of Object.entries(obj)) {
        const fullPath = `${path}.${key}`;
        if (typeof value === 'string') {
          expect(value.length, `${fullPath} should not be empty`).toBeGreaterThan(0);
        } else if (typeof value === 'object' && value !== null) {
          checkNonEmpty(value as Record<string, unknown>, fullPath);
        }
      }
    }
    for (const [code, locale] of Object.entries(allLocales)) {
      checkNonEmpty(locale as Record<string, unknown>, code);
    }
  });
});
