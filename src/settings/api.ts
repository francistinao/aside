// Invoke wrappers and the settings shape for the Settings window.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

export type Feel = 'snappy' | 'smooth' | 'floaty';
export type Distance = 'close' | 'normal' | 'far';
export type PeekKey = 'control' | 'option' | 'shift';
export type TextSize = 'small' | 'medium' | 'large';
export type Highlight = 'word' | 'line';
export type Corner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
export type CaptureStatus = 'hidden' | 'mayBeVisible' | 'visible' | 'unsupported' | 'failed';
export type ShortcutAction = 'pause' | 'prev' | 'next' | 'faster' | 'slower' | 'restart' | 'toggleHidden' | 'stop';

export interface Settings {
  wpm: number;
  highlight: Highlight;
  opacity: number;
  corner: Corner;
  textSize: TextSize;
  hideFromCapture: boolean;
  follow: Feel;
  distance: Distance;
  flipAtEdges: boolean;
  peekKey: PeekKey;
  fadeWhileClicking: boolean;
  shortcuts: Record<ShortcutAction, string>;
  openAtLogin: boolean;
  scriptsFolder: string | null;
  sidebarWidth: number;
  sidebarVisible: boolean;
}

export type SettingsPatch = Partial<
  Pick<Settings, 'textSize' | 'hideFromCapture' | 'follow' | 'distance' | 'flipAtEdges' | 'peekKey' | 'fadeWhileClicking' | 'openAtLogin'>
>;

export interface ShortcutFailure {
  action: ShortcutAction;
  message: string;
}

export interface ShortcutStatus {
  failed: ShortcutFailure[];
}

export const api = {
  settings: () => invoke<Settings>('settings_get'),
  patch: (patch: SettingsPatch) => invoke<Settings>('settings_set', { patch }),
  captureStatus: () => invoke<CaptureStatus>('capture_status'),
  shortcutStatus: () => invoke<ShortcutStatus>('shortcuts_status'),
  setShortcut: (action: ShortcutAction, accelerator: string) => invoke<Settings>('shortcuts_set', { action, accelerator }),
  resetShortcut: (action: ShortcutAction) => invoke<Settings>('shortcuts_reset', { action }),
  chooseFolder: () => invoke<unknown>('vault_choose_folder'),
  onSettings: (handler: (settings: Settings) => void): Promise<UnlistenFn> =>
    listen<Settings>('settings://changed', (event) => handler(event.payload)),
  onCapture: (handler: (status: CaptureStatus) => void): Promise<UnlistenFn> =>
    listen<CaptureStatus>('capture://status', (event) => handler(event.payload)),
  onShortcuts: (handler: (status: ShortcutStatus) => void): Promise<UnlistenFn> =>
    listen<ShortcutStatus>('shortcuts://status', (event) => handler(event.payload)),
};

/// Turns a rejected invoke into the message the user should read.
export function describeError(reason: unknown): string {
  if (typeof reason === 'string') return reason;
  if (reason instanceof Error) return reason.message;
  return 'Something did not work. Try again.';
}
