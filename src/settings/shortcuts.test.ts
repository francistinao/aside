import { describe, expect, it } from 'vitest';
import { acceleratorFromPress, formatShortcut, isAllowed } from './shortcuts';

describe('formatShortcut', () => {
  it('writes the macOS form with glyphs and a space before the key', () => {
    expect(formatShortcut('Control+Alt+Space', true)).toBe('⌃⌥ Space');
    expect(formatShortcut('Control+Alt+ArrowLeft', true)).toBe('⌃⌥ ←');
    expect(formatShortcut('Control+Alt+KeyR', true)).toBe('⌃⌥ R');
    expect(formatShortcut('Control+Alt+Period', true)).toBe('⌃⌥ .');
    expect(formatShortcut('Shift+Super+Digit8', true)).toBe('⇧⌘ 8');
  });

  it('writes the Windows form with plus signs', () => {
    expect(formatShortcut('Control+Alt+Space', false)).toBe('Ctrl+Alt+Space');
    expect(formatShortcut('Control+Alt+ArrowUp', false)).toBe('Ctrl+Alt+Up');
    expect(formatShortcut('Control+Alt+Period', false)).toBe('Ctrl+Alt+.');
  });

  it('orders modifiers the same way whatever the stored order', () => {
    expect(formatShortcut('Alt+Control+KeyH', false)).toBe('Ctrl+Alt+H');
  });
});

describe('acceleratorFromPress', () => {
  const press = (code: string, mods: Partial<Omit<Parameters<typeof acceleratorFromPress>[0], 'code'>> = {}) =>
    acceleratorFromPress({ code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });

  it('needs a modifier and a key', () => {
    expect(press('KeyR')).toBeNull();
    expect(press('ControlLeft', { ctrlKey: true })).toBeNull();
    expect(press('AltRight', { ctrlKey: true, altKey: true })).toBeNull();
  });

  it('builds the stored form in a fixed modifier order', () => {
    expect(press('KeyR', { altKey: true, ctrlKey: true })).toBe('Control+Alt+KeyR');
    expect(press('Space', { ctrlKey: true, altKey: true })).toBe('Control+Alt+Space');
    expect(press('Digit8', { shiftKey: true, metaKey: true })).toBe('Shift+Super+Digit8');
  });
});

describe('isAllowed', () => {
  it('needs Control or Option in the combination', () => {
    expect(isAllowed('Control+Alt+KeyR')).toBe(true);
    expect(isAllowed('Alt+KeyR')).toBe(true);
    expect(isAllowed('Super+KeyQ')).toBe(false);
    expect(isAllowed('Shift+Digit8')).toBe(false);
  });
});
