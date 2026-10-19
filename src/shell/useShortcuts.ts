// Keyboard shortcuts handled in the webview. On macOS the app menu covers most of them
// (menu accelerators never reach the page), so this is mainly for Windows and the few
// keys without a menu item.

import { useEffect } from 'react';
import { isMac } from '../platform';

/// Returns the action id for a key event, or null. Ids match the menu item ids in menu.rs.
export function shortcutAction(event: KeyboardEvent): string | null {
  const key = event.key.toLowerCase();
  const mod = isMac ? event.metaKey : event.ctrlKey;
  if (!mod) return null;
  const { shiftKey, altKey, ctrlKey } = event;
  if (isMac ? ctrlKey && !shiftKey && !altKey && key === 's' : shiftKey && !altKey && key === 's') return 'view.toggle-sidebar';
  if (!shiftKey && !altKey && key === 'f') return 'edit.find';
  if (!altKey && key === 'n') return shiftKey ? 'file.new-folder' : 'file.new-script';
  if (altKey && !shiftKey) {
    if (event.code === 'Digit1') return 'format.title';
    if (event.code === 'Digit2') return 'format.heading';
    if (event.code === 'Digit3') return 'format.subheading';
    if (event.code === 'Digit0') return 'format.body';
  }
  return null;
}

export function useShortcuts(dispatch: (id: string) => void): void {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const id = shortcutAction(event);
      if (id === null) return;
      event.preventDefault();
      dispatch(id);
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [dispatch]);
}
