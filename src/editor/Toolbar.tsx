import { PanelLeft } from 'lucide-react';
import styles from './editor.module.css';

interface Props {
  name: string | null;
  onToggleSidebar: () => void;
}

export function Toolbar({ name, onToggleSidebar }: Props) {
  return (
    <header className={styles.toolbar} data-tauri-drag-region>
      <button type="button" className={styles.iconButton} aria-label="Toggle Sidebar" title="Toggle Sidebar" onClick={onToggleSidebar}>
        <PanelLeft size={16} strokeWidth={1.5} aria-hidden="true" />
      </button>
      {name !== null && <span className={styles.title}>{name}</span>}
    </header>
  );
}
