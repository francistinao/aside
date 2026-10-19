// Keyboard shortcuts handled in the webview. On macOS the app menu covers most of them
// (menu accelerators never reach the page), so this is mainly for Windows and the few
// keys without a menu item.

import { useEffect } from 'react';
import { isMac } from '../platform';

export interface ShortcutHandlers {
  toggleSidebar: () => void;
  find: () => void;
}

export function useShortcuts(handlers: ShortcutHandlers): void {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const key = event.key.toLowerCase();
      const mod = isMac ? event.metaKey : event.ctrlKey;
      const toggle = isMac ? event.metaKey && event.ctrlKey && key === 's' : event.ctrlKey && event.shiftKey && key === 's';
      if (toggle) {
        event.preventDefault();
        handlers.toggleSidebar();
      } else if (mod && !event.shiftKey && !event.altKey && key === 'f') {
        event.preventDefault();
        handlers.find();
      }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [handlers]);
}
