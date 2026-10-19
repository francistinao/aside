import { invoke } from '@tauri-apps/api/core';
import type { Editor } from '@tiptap/core';
import { useEditorState } from '@tiptap/react';
import { Bold, Heading, Italic, List, PanelLeft, Pause } from 'lucide-react';
import { useEffect, useState } from 'react';
import { formatEdited } from './edited';
import styles from './editor.module.css';

interface Props {
  editor: Editor | null;
  name: string | null;
  modified: number | null;
  onToggleSidebar: () => void;
  onError: (reason: unknown) => void;
}

const ICON = { size: 16, strokeWidth: 1.5 } as const;
const EDITED_REFRESH_MS = 60_000;

/// The script's modified time, refreshed every minute.
function EditedAgo({ modified }: { modified: number }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), EDITED_REFRESH_MS);
    return () => clearInterval(timer);
  }, []);
  return <span className={styles.edited}>{formatEdited(modified, Math.max(now, modified))}</span>;
}

function FormatButtons({ editor, onError }: { editor: Editor; onError: (reason: unknown) => void }) {
  const active = useEditorState({
    editor,
    selector: ({ editor: current }) => ({
      bold: current.isActive('bold'),
      italic: current.isActive('italic'),
      heading: current.isActive('heading'),
      list: current.isActive('bulletList'),
    }),
  });

  function headingMenu(event: React.MouseEvent<HTMLButtonElement>) {
    const rect = event.currentTarget.getBoundingClientRect();
    invoke('library_popup_menu', { kind: 'heading', x: rect.left, y: rect.bottom + 4 }).catch(onError);
  }

  return (
    <>
      <div className={styles.capsule} role="group" aria-label="Formatting">
        <button type="button" className={styles.capsuleButton} aria-label="Bold" title="Bold" aria-pressed={active.bold} onClick={() => editor.chain().focus().toggleBold().run()}>
          <Bold {...ICON} aria-hidden="true" />
        </button>
        <button type="button" className={styles.capsuleButton} aria-label="Italic" title="Italic" aria-pressed={active.italic} onClick={() => editor.chain().focus().toggleItalic().run()}>
          <Italic {...ICON} aria-hidden="true" />
        </button>
        <button type="button" className={styles.capsuleButton} aria-label="Heading" title="Heading" aria-pressed={active.heading} onClick={headingMenu}>
          <Heading {...ICON} aria-hidden="true" />
        </button>
        <button type="button" className={styles.capsuleButton} aria-label="List" title="Bulleted List" aria-pressed={active.list} onClick={() => editor.chain().focus().toggleBulletList().run()}>
          <List {...ICON} aria-hidden="true" />
        </button>
      </div>
      <div className={styles.capsule}>
        <button type="button" className={styles.capsuleButton} title="Insert Pause" onClick={() => editor.chain().focus().insertPause().run()}>
          <Pause {...ICON} aria-hidden="true" />
          <span>Insert Pause</span>
        </button>
      </div>
    </>
  );
}

export function Toolbar({ editor, name, modified, onToggleSidebar, onError }: Props) {
  return (
    <header className={styles.toolbar} data-tauri-drag-region>
      <button type="button" className={styles.iconButton} aria-label="Toggle Sidebar" title="Toggle Sidebar" onClick={onToggleSidebar}>
        <PanelLeft {...ICON} aria-hidden="true" />
      </button>
      {name !== null && (
        <span className={styles.titleBlock}>
          <span className={styles.title}>{name}</span>
          {modified !== null && <EditedAgo modified={modified} />}
        </span>
      )}
      <span className={styles.spacer} data-tauri-drag-region />
      {editor !== null && <FormatButtons editor={editor} onError={onError} />}
    </header>
  );
}
