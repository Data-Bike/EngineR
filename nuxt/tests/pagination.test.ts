import { describe, expect, it } from 'vitest';
import { nextCursorOf } from '../app/utils/pagination';

describe('nextCursorOf', () => {
  it('returns the next keyset cursor as a string', () => {
    expect(nextCursorOf({ next_cursor: '01a15e86-b685-7bd1-beef-6d9c56b83046' })).toBe('01a15e86-b685-7bd1-beef-6d9c56b83046');
    expect(nextCursorOf({ next_cursor: 2334 })).toBe('2334');
  });

  it('returns null at the last page or without meta', () => {
    expect(nextCursorOf({ next_cursor: null })).toBeNull();
    expect(nextCursorOf({ next_cursor: '' })).toBeNull();
    expect(nextCursorOf({})).toBeNull();
    expect(nextCursorOf(undefined)).toBeNull();
  });
});
