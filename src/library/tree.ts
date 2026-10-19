// Pure sidebar logic: the folder tree, the visible rows, and keyboard moves.

import type { Entry } from './vault';
import { parentOf } from './vault';

export interface FolderNode {
  path: string;
  name: string;
  depth: number;
  folders: FolderNode[];
  scripts: Entry[];
  /// Scripts inside, including nested folders.
  count: number;
}

export type SmartId = 'all' | 'recents' | 'pinned';

export type Row =
  | { kind: 'header'; id: string; label: string }
  | { kind: 'smart'; id: string; smart: SmartId; label: string; count: number }
  | { kind: 'folder'; id: string; path: string; name: string; depth: number; count: number; expanded: boolean }
  | { kind: 'script'; id: string; path: string; name: string; depth: number };

export const SMART_LABEL: Record<SmartId, string> = {
  all: 'All Scripts',
  recents: 'Recents',
  pinned: 'Pinned',
};

export function smartRowId(smart: SmartId): string {
  return `smart:${smart}`;
}

export function folderRowId(path: string): string {
  return `folder:${path}`;
}

export function scriptRowId(path: string): string {
  return `script:${path}`;
}

/// Entries arrive flat, each folder before its contents, so one pass nests them.
export function buildTree(entries: Entry[]): FolderNode {
  const root: FolderNode = { path: '', name: '', depth: 0, folders: [], scripts: [], count: 0 };
  const nodes = new Map<string, FolderNode>([['', root]]);
  for (const entry of entries) {
    const parent = nodes.get(parentOf(entry.path)) ?? root;
    if (entry.kind === 'folder') {
      const node: FolderNode = { path: entry.path, name: entry.name, depth: parent.depth + 1, folders: [], scripts: [], count: 0 };
      parent.folders.push(node);
      nodes.set(entry.path, node);
    } else {
      parent.scripts.push(entry);
    }
  }
  const count = (node: FolderNode): number => {
    node.count = node.scripts.length + node.folders.reduce((sum, folder) => sum + count(folder), 0);
    return node.count;
  };
  count(root);
  return root;
}

export interface Lists {
  recents: string[];
  pinned: string[];
}

/// The rows the sidebar shows for a view: the Library section, then the tree or a list.
export function sidebarRows(tree: FolderNode, view: SmartId, expanded: ReadonlySet<string>, lists: Lists, entries: Entry[]): Row[] {
  const rows: Row[] = [
    { kind: 'header', id: 'header:library', label: 'Library' },
    { kind: 'smart', id: smartRowId('all'), smart: 'all', label: SMART_LABEL.all, count: tree.count },
    { kind: 'smart', id: smartRowId('recents'), smart: 'recents', label: SMART_LABEL.recents, count: lists.recents.length },
    { kind: 'smart', id: smartRowId('pinned'), smart: 'pinned', label: SMART_LABEL.pinned, count: lists.pinned.length },
  ];
  if (view === 'all') {
    rows.push({ kind: 'header', id: 'header:folders', label: 'Folders' });
    pushFolder(rows, tree, expanded, 0);
  } else {
    const names = new Map(entries.map((entry) => [entry.path, entry.name]));
    rows.push({ kind: 'header', id: `header:${view}`, label: SMART_LABEL[view] });
    for (const path of view === 'recents' ? lists.recents : lists.pinned) {
      const name = names.get(path);
      if (name !== undefined) rows.push({ kind: 'script', id: scriptRowId(path), path, name, depth: 0 });
    }
  }
  return rows;
}

function pushFolder(rows: Row[], node: FolderNode, expanded: ReadonlySet<string>, depth: number): void {
  for (const folder of node.folders) {
    const isOpen = expanded.has(folder.path);
    rows.push({ kind: 'folder', id: folderRowId(folder.path), path: folder.path, name: folder.name, depth, count: folder.count, expanded: isOpen });
    if (isOpen) pushFolder(rows, folder, expanded, depth + 1);
  }
  for (const script of node.scripts) {
    rows.push({ kind: 'script', id: scriptRowId(script.path), path: script.path, name: script.name, depth });
  }
}

/// The next selectable row in a direction, skipping headers. Stays put at the ends.
export function moveSelection(rows: Row[], currentId: string | null, delta: 1 | -1): string | null {
  const selectable = rows.filter((row) => row.kind !== 'header');
  if (selectable.length === 0) return null;
  const index = selectable.findIndex((row) => row.id === currentId);
  if (index < 0) return (delta > 0 ? selectable[0] : selectable[selectable.length - 1])?.id ?? null;
  const next = selectable[Math.min(Math.max(index + delta, 0), selectable.length - 1)];
  return next?.id ?? null;
}

/// Left on a row: collapse an open folder, otherwise jump to the folder that holds the row.
export function parentRowId(rows: Row[], currentId: string | null): string | null {
  const index = rows.findIndex((row) => row.id === currentId);
  const current = rows[index];
  if (current === undefined || (current.kind !== 'folder' && current.kind !== 'script') || current.depth === 0) return null;
  for (let i = index - 1; i >= 0; i -= 1) {
    const row = rows[i];
    if (row?.kind === 'folder' && row.depth === current.depth - 1) return row.id;
  }
  return null;
}
