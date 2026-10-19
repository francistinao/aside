// Shortcut strings travel in the global shortcut plugin's form ("Control+Alt+KeyR") and are
// shown the way each platform writes them: "⌃⌥ R" on macOS, "Ctrl+Alt+R" on Windows.

import type { ShortcutAction } from './api';

export const ACTIONS: { action: ShortcutAction; label: string }[] = [
  { action: 'pause', label: 'Pause or Resume' },
  { action: 'prev', label: 'Previous line' },
  { action: 'next', label: 'Next line' },
  { action: 'faster', label: 'Faster' },
  { action: 'slower', label: 'Slower' },
  { action: 'restart', label: 'Restart' },
  { action: 'toggleHidden', label: 'Show or Hide Prompter' },
  { action: 'stop', label: 'Stop' },
];

const MAC_MODIFIERS: Record<string, string> = { Control: '⌃', Alt: '⌥', Shift: '⇧', Super: '⌘' };
const WINDOWS_MODIFIERS: Record<string, string> = { Control: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Super: 'Win' };
const MODIFIER_ORDER = ['Control', 'Alt', 'Shift', 'Super'];

const MAC_KEYS: Record<string, string> = {
  ArrowLeft: '←',
  ArrowRight: '→',
  ArrowUp: '↑',
  ArrowDown: '↓',
  Enter: '↩',
  Escape: 'Esc',
  Backspace: '⌫',
  Tab: '⇥',
  Space: 'Space',
  Period: '.',
  Comma: ',',
};
const WINDOWS_KEYS: Record<string, string> = {
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  Enter: 'Enter',
  Escape: 'Esc',
  Backspace: 'Backspace',
  Tab: 'Tab',
  Space: 'Space',
  Period: '.',
  Comma: ',',
};

function keyLabel(code: string, mac: boolean): string {
  const named = (mac ? MAC_KEYS : WINDOWS_KEYS)[code];
  if (named !== undefined) return named;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  return code;
}

/// "Control+Alt+Space" to "⌃⌥ Space" on macOS or "Ctrl+Alt+Space" on Windows.
export function formatShortcut(accelerator: string, mac: boolean): string {
  const parts = accelerator.split('+').filter(Boolean);
  const modifiers = MODIFIER_ORDER.filter((modifier) => parts.includes(modifier));
  const key = parts.find((part) => !MODIFIER_ORDER.includes(part)) ?? '';
  if (mac) {
    const glyphs = modifiers.map((modifier) => MAC_MODIFIERS[modifier] ?? modifier).join('');
    return `${glyphs} ${keyLabel(key, true)}`.trim();
  }
  return [...modifiers.map((modifier) => WINDOWS_MODIFIERS[modifier] ?? modifier), keyLabel(key, false)].join('+');
}

export interface KeyPress {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const MODIFIER_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/;

/// The accelerator a key press means, or null while only modifiers are down or none is.
export function acceleratorFromPress(press: KeyPress): string | null {
  if (press.code === '' || MODIFIER_CODES.test(press.code)) return null;
  const modifiers: string[] = [];
  if (press.ctrlKey) modifiers.push('Control');
  if (press.altKey) modifiers.push('Alt');
  if (press.shiftKey) modifiers.push('Shift');
  if (press.metaKey) modifiers.push('Super');
  if (modifiers.length === 0) return null;
  return [...modifiers, press.code].join('+');
}
