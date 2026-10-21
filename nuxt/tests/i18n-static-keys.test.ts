import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import ru from '../i18n/locales/ru';

function flatten(obj: Record<string, unknown>, prefix = ''): Set<string> {
  const out = new Set<string>();
  for (const [k, v] of Object.entries(obj)) {
    const p = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      for (const x of flatten(v as Record<string, unknown>, p)) out.add(x);
    } else {
      out.add(p);
    }
  }
  return out;
}

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.(vue|ts)$/.test(name) ? [path] : [];
  });
}

describe('static i18n keys', () => {
  it('every literal t()/te() key used in app/ exists in the ru source locale', () => {
    const keys = flatten(ru as unknown as Record<string, unknown>);
    const root = join(process.cwd(), 'app');
    const call = /\b(?:\$?t|te)\(\s*['"]([a-zA-Z0-9_.]+)['"]/g;
    const missing = new Set<string>();
    for (const file of sourceFiles(root)) {
      for (const match of readFileSync(file, 'utf8').matchAll(call)) {
        if (!keys.has(match[1])) missing.add(`${relative(root, file)}: ${match[1]}`);
      }
    }
    expect([...missing]).toEqual([]);
  });
});
