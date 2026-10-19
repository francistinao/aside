import type { Editor } from '@tiptap/core';
import { PanelLeft, Pause } from 'lucide-react';
import styles from './editor.module.css';

interface Props {
  editor: Editor | null;
  name: string | null;
  onToggleSidebar: () => void;
}

const ICON = { size: 16, strokeWidth: 1.5 } as const;

export function Toolbar({ editor, name, onToggleSidebar }: Props) {
  return (
    <header className={styles.toolbar} data-tauri-drag-region>
      <button type="button" className={styles.iconButton} aria-label="Toggle Sidebar" title="Toggle Sidebar" onClick={onToggleSidebar}>
        <PanelLeft {...ICON} aria-hidden="true" />
      </button>
      {name !== null && <span className={styles.title}>{name}</span>}
      <span className={styles.spacer} data-tauri-drag-region />
      {editor !== null && (
        <div className={styles.capsule}>
          <button
            type="button"
            className={styles.capsuleButton}
            title="Insert Pause"
            onClick={() => editor.chain().focus().insertPause().run()}
          >
            <Pause {...ICON} aria-hidden="true" />
            <span>Insert Pause</span>
          </button>
        </div>
      )}
    </header>
  );
}
