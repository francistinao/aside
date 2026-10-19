import { describe, expect, it } from 'vitest';
import { formatClock, formatProgress } from './format';

describe('formatClock', () => {
  it('formats whole seconds as m:ss', () => {
    expect(formatClock(0)).toBe('0:00');
    expect(formatClock(58.9)).toBe('0:58');
    expect(formatClock(165)).toBe('2:45');
    expect(formatClock(725)).toBe('12:05');
  });

  it('treats bad input as zero', () => {
    expect(formatClock(-3)).toBe('0:00');
    expect(formatClock(Number.NaN)).toBe('0:00');
  });
});

describe('formatProgress', () => {
  it('joins elapsed and total', () => {
    expect(formatProgress(58, 165)).toBe('0:58 / 2:45');
  });
});
