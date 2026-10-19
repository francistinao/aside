import { ChevronRight, Clock, FileText, Folder, Pin } from 'lucide-react';
import type { Row, SmartId } from './tree';
import styles from './sidebar.module.css';

const SMART_ICON: Record<SmartId, typeof FileText> = { all: FileText, recents: Clock, pinned: Pin };
const ICON = { size: 16, strokeWidth: 1.5 } as const;

interface Props {
  row: Row;
  selected: boolean;
  onSelect: (row: Row) => void;
  onContextMenu: (row: Row, x: number, y: number) => void;
}

export function SidebarRow({ row, selected, onSelect, onContextMenu }: Props) {
  if (row.kind === 'header') {
    return <div className={styles.header}>{row.label}</div>;
  }
  const depth = 'depth' in row ? row.depth : 0;
  const Icon = row.kind === 'smart' ? SMART_ICON[row.smart] : row.kind === 'folder' ? Folder : FileText;
  const label = row.kind === 'smart' ? row.label : row.name;
  const count = row.kind === 'smart' || row.kind === 'folder' ? row.count : null;
  return (
    <div
      role="treeitem"
      aria-selected={selected}
      aria-expanded={row.kind === 'folder' ? row.expanded : undefined}
      className={`${styles.row} ${selected ? styles.selected : ''}`}
      style={{ paddingLeft: `calc(var(--gap-8) + ${depth} * var(--gap-16))` }}
      data-row-id={row.id}
      onClick={() => onSelect(row)}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(row, event.clientX, event.clientY);
      }}
    >
      {row.kind === 'folder' ? (
        <ChevronRight {...ICON} className={`${styles.chevron} ${row.expanded ? styles.open : ''}`} aria-hidden="true" />
      ) : (
        <span className={styles.chevronSpace} />
      )}
      <Icon {...ICON} className={styles.icon} aria-hidden="true" />
      <span className={styles.label}>{label}</span>
      {count !== null && count > 0 && <span className={styles.count}>{count}</span>}
    </div>
  );
}
