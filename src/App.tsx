import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import styles from './App.module.css';
import { EmptyLibrary } from './library/EmptyLibrary';
import { FolderScreen } from './library/FolderScreen';
import { Sidebar } from './library/Sidebar';
import type { Row, SmartId } from './library/tree';
import { useVault } from './library/useVault';
import { describeError, parentOf, vault } from './library/vault';
import type { Hit } from './library/vault';
import { useShortcuts } from './shell/useShortcuts';

interface ShellSettings {
  sidebarWidth: number;
  sidebarVisible: boolean;
}

const DEFAULT_SHELL: ShellSettings = { sidebarWidth: 240, sidebarVisible: true };

export function App() {
  const [shell, setShell] = useState<ShellSettings>(DEFAULT_SHELL);
  const [view, setView] = useState<SmartId>('all');
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [hits, setHits] = useState<Hit[]>([]);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const library = useVault();
  const report = useCallback((reason: unknown) => setError(describeError(reason)), []);

  useEffect(() => {
    let active = true;
    invoke<ShellSettings>('settings_get')
      .then((value) => {
        if (active) setShell({ sidebarWidth: value.sidebarWidth, sidebarVisible: value.sidebarVisible });
      })
      .catch(report);
    return () => {
      active = false;
    };
  }, [report]);

  const saveShell = useCallback((patch: Partial<ShellSettings>) => {
    setShell((current) => ({ ...current, ...patch }));
    invoke('settings_set', { patch }).catch(report);
  }, [report]);

  const shortcuts = useMemo(
    () => ({
      toggleSidebar: () => saveShell({ sidebarVisible: !shell.sidebarVisible }),
      find: () => {
        if (!shell.sidebarVisible) saveShell({ sidebarVisible: true });
        searchRef.current?.focus();
        searchRef.current?.select();
      },
    }),
    [saveShell, shell.sidebarVisible],
  );
  useShortcuts(shortcuts);

  // Results follow the query with a short rest, and refresh when the folder changes.
  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed === '') return;
    let active = true;
    const timer = setTimeout(() => {
      vault
        .search(trimmed)
        .then((found) => {
          if (active) setHits(found);
        })
        .catch(report);
    }, 100);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [query, library.tree, report]);

  function toggleFolder(path: string, open: boolean) {
    setExpanded((current) => {
      const next = new Set(current);
      if (open) next.add(path);
      else next.delete(path);
      return next;
    });
  }

  function contextMenu(row: Row, x: number, y: number) {
    if (row.kind !== 'folder' && row.kind !== 'script') return;
    const pinned = row.kind === 'script' && (library.tree?.pins.includes(row.path) ?? false);
    vault.contextMenu(row.path, row.kind, pinned, x, y).catch(report);
  }

  function newScript() {
    const dir = library.openPath === null ? '' : parentOf(library.openPath);
    library
      .createScript(dir)
      .then((path) => setSelectedId(`script:${path}`))
      .catch(report);
  }

  const width = shell.sidebarVisible ? shell.sidebarWidth : 0;
  const folderProblem = library.phase === 'noFolder' || library.phase === 'missing';
  const noScripts = library.phase === 'ready' && !library.tree?.entries.some((entry) => entry.kind === 'script');

  return (
    <div className={styles.shell} style={{ '--sidebar-width': `${width}px` } as React.CSSProperties}>
      <aside className={styles.sidebar} aria-label="Library">
        <div className={styles.sidebarTop} data-tauri-drag-region />
        {library.tree !== null && (
          <Sidebar
            tree={library.tree}
            view={view}
            expanded={expanded}
            selectedId={selectedId}
            openPath={library.openPath}
            width={shell.sidebarWidth}
            query={query}
            hits={hits}
            searchRef={searchRef}
            onQuery={setQuery}
            onView={setView}
            onToggleFolder={toggleFolder}
            onSelect={setSelectedId}
            onOpen={library.open}
            onContextMenu={contextMenu}
            onWidth={(value, commit) => (commit ? saveShell({ sidebarWidth: value }) : setShell((s) => ({ ...s, sidebarWidth: value })))}
            onNewScript={newScript}
          />
        )}
      </aside>
      <main className={styles.main}>
        <header className={styles.toolbar} data-tauri-drag-region />
        <section className={styles.content}>
          {folderProblem && <FolderScreen missing={library.phase === 'missing'} onChoose={() => void library.chooseFolder()} />}
          {noScripts && <EmptyLibrary onNewScript={newScript} />}
          {(error ?? library.error) !== null && !folderProblem && <p className={styles.error}>{error ?? library.error}</p>}
        </section>
      </main>
    </div>
  );
}
