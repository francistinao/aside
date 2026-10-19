import { invoke } from '@tauri-apps/api/core';

interface PlatformInfo {
  os: 'macos' | 'windows';
  backdrop: boolean;
}

export const isMac = /Macintosh|Mac OS X/.test(navigator.userAgent);

/// Known synchronously so the first paint already has the right chrome.
export function applyPlatformClass(): void {
  document.documentElement.classList.add(isMac ? 'platform-macos' : 'platform-windows');
}

/// Mica exists on Windows 11 only; Windows 10 gets an opaque sidebar instead.
export async function applyBackdrop(): Promise<void> {
  try {
    const info = await invoke<PlatformInfo>('platform_info');
    document.documentElement.dataset.backdrop = info.backdrop ? 'yes' : 'no';
  } catch {
    document.documentElement.dataset.backdrop = 'no';
  }
}
