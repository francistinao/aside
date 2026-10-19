import { describe, expect, it } from 'vitest';
import { formatEdited } from './edited';

const now = Date.UTC(2026, 9, 19, 12, 0, 0);
const ago = (ms: number) => now - ms;

describe('formatEdited', () => {
  it('speaks in minutes, hours and days', () => {
    expect(formatEdited(ago(5_000), now)).toBe('Edited just now');
    expect(formatEdited(ago(60_000), now)).toBe('Edited 1 min ago');
    expect(formatEdited(ago(2 * 60_000 + 30_000), now)).toBe('Edited 2 min ago');
    expect(formatEdited(ago(60 * 60_000), now)).toBe('Edited 1 hour ago');
    expect(formatEdited(ago(5 * 60 * 60_000), now)).toBe('Edited 5 hours ago');
    expect(formatEdited(ago(26 * 60 * 60_000), now)).toBe('Edited yesterday');
    expect(formatEdited(ago(3 * 24 * 60 * 60_000), now)).toBe('Edited 3 days ago');
  });

  it('falls back to a date after a week and never goes negative', () => {
    expect(formatEdited(ago(10 * 24 * 60 * 60_000), now)).toMatch(/^Edited \w+ \d+$/);
    expect(formatEdited(now + 60_000, now)).toBe('Edited just now');
  });
});
