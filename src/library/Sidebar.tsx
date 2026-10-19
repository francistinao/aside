import { Plus } from 'lucide-react';
import { useMemo, useRef } from 'react';
import type { PointerEvent as ReactPointerEvent, RefObject } from 'react';
import { SearchField } from './SearchField';
import { SearchResults } from './SearchResults';
import { SidebarRow } from './SidebarRow';
import { buildTree, moveSelection, parentRowId, sidebarRows } from './tree';
import type { Row, SmartId } from './tree';
import type { Hit, Tree } from './vault';
import styles from './sidebar.module.css';

export const MIN_SIDEBAR = 200;
export const MAX_SIDEBAR = 320;

interface Props {
  tree: Tree;
  view: SmartId;
  expanded: ReadonlySet<string>;
  selectedId: string | null;
  openPath: string | null;
  width: number;
  query: string;
  hits: Hit[];
  searchRef: RefObject<HTMLInputElement | null>;
  onQuery: (query: string) => void;
  onView: (view: SmartId) => void;
  onToggleFolder: (path: string, expanded: boolean) => void;
  onSelect: (id: string | null) => void;
  onOpen: (path: string) => void;
  onContextMenu: (row: Row, x: number, y: number) => void;
  onWidth: (width: number, commit: boolean) => void;
  onNewScript: () => void;
}

export function Sidebar(props: Props) {
  const { tree, view, expanded, selectedId, openPath, width, query, hits, searchRef, onQuery } = props;
  const { onView, onToggleFolder, onSelect, onOpen, onContextMenu, onWidth, onNewScript } = props;
  const searching = query.trim() !== '';
  const rows = useMemo(
    () => sidebarRows(buildTree(tree.entries), view, expanded, { recents: tree.recents, pinned: tree.pins }, tree.entries),
    [tree, view, expanded],
  );
  const drag = useRef<{ startX: number; startWidth: number } | null>(null);

  function activate(row: Row) {
    onSelect(row.id);
    if (row.kind === 'smart') onView(row.smart);
    else if (row.kind === 'folder') onToggleFolder(row.path, !row.expanded);
    else if (row.kind === 'script') onOpen(row.path);
  }

  function onKeyDown(event: React.KeyboardEvent) {
    const current = rows.find((row) => row.id === selectedId) ?? null;
    switch (event.key) {
      case 'ArrowDown':
      case 'ArrowUp':
        onSelect(moveSelection(rows, selectedId, event.key === 'ArrowDown' ? 1 : -1));
        break;
      case 'ArrowRight':
        if (current?.kind === 'folder' && !current.expanded) onToggleFolder(current.path, true);
        else if (current?.kind === 'folder') onSelect(moveSelection(rows, selectedId, 1));
        else return;
        break;
      case 'ArrowLeft':
        if (current?.kind === 'folder' && current.expanded) onToggleFolder(current.path, false);
        else {
          const parent = parentRowId(rows, selectedId);
          if (parent !== null) onSelect(parent);
          else return;
        }
        break;
      case 'Enter':
        if (current !== null) activate(current);
        else return;
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  function startResize(event: ReactPointerEvent<HTMLDivElement>) {
    drag.current = { startX: event.clientX, startWidth: width };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function resize(event: ReactPointerEvent<HTMLDivElement>, commit: boolean) {
    if (drag.current === null) return;
    const next = Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, drag.current.startWidth + event.clientX - drag.current.startX));
    onWidth(next, commit);
    if (commit) drag.current = null;
  }

  return (
    <>
      <SearchField value={query} onChange={onQuery} inputRef={searchRef} />
      {searching ? (
        <SearchResults query={query.trim()} hits={hits} openPath={openPath} onOpen={onOpen} />
      ) : (
        <div className={styles.list} role="tree" tabIndex={0} aria-label="Scripts" onKeyDown={onKeyDown}>
          {rows.map((row) => (
            <SidebarRow key={row.id} row={row} selected={row.id === selectedId} onSelect={activate} onContextMenu={onContextMenu} />
          ))}
        </div>
      )}
      <div className={styles.footer}>
        <button type="button" className={styles.newScript} onClick={onNewScript}>
          <Plus size={16} strokeWidth={1.5} aria-hidden="true" />
          New Script
        </button>
      </div>
      <div
        className={styles.handle}
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize sidebar"
        onPointerDown={startResize}
        onPointerMove={(event) => resize(event, false)}
        onPointerUp={(event) => resize(event, true)}
      />
    </>
  );
}
