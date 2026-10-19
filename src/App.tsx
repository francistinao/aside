import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import styles from './App.module.css';
import { EmptyLibrary } from './library/EmptyLibrary';
import { FolderScreen } from './library/FolderScreen';
import { Sidebar } from './library/Sidebar';
import { folderRowId, scriptRowId } from './library/tree';
import type { Row, SmartId } from './library/tree';
import { useVault } from './library/useVault';
import { describeError, parentOf, vault } from './library/vault';
import type { FileData, Hit } from './library/vault';
import { useShortcuts } from './shell/useShortcuts';

interface ShellSettings {
  sidebarWidth: number;
  sidebarVisible: boolean;
}

const DEFAULT_SHELL: ShellSettings = { sidebarWidth: 240, sidebarVisible: true };

interface MenuAction {
  id: string;
  path?: string;
}

/// Checkpoint stand-in for the editor: the open script, read only. Replaced in Part C.
function Preview({ path, file }: { path: string; file: FileData | null }) {
  const name = path.split('/').pop()?.replace(/\.md$/, '') ?? path;
  return (
    <div className={styles.preview}>
      <h1 className={styles.previewTitle}>{name}</h1>
      <pre className={styles.previewBody}>{file?.content ?? ''}</pre>
    </div>
  );
}

export function App() {
  const [shell, setShell] = useState<ShellSettings>(DEFAULT_SHELL);
  const [view, setView] = useState<SmartId>('all');
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [hits, setHits] = useState<Hit[]>([]);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [file, setFile] = useState<{ path: string; data: FileData } | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const library = useVault();
  const { open, openPath, refresh, createScript, createFolder } = library;
  const report = useCallback((reason: unknown) => setError(describeError(reason)), []);

  // The open script is read whenever it or the folder changes.
  useEffect(() => {
    if (openPath === null) return;
    let active = true;
    vault
      .read(openPath)
      .then((data) => {
        if (active) setFile({ path: openPath, data });
      })
      .catch(report);
    return () => {
      active = false;
    };
  }, [openPath, library.tree, report]);
  const shownFile = file !== null && file.path === openPath ? file.data : null;

  /// Opening from the sidebar selects the row and reveals its folders.
  const openScript = useCallback(
    (path: string) => {
      setSelectedId(scriptRowId(path));
      setExpanded((current) => {
        const next = new Set(current);
        let dir = parentOf(path);
        while (dir !== '') {
          next.add(dir);
          dir = parentOf(dir);
        }
        return next;
      });
      open(path);
    },
    [open],
  );

  const act = useCallback(
    (action: MenuAction) => {
      const path = action.path ?? openPath ?? '';
      const isFolder = library.tree?.entries.some((entry) => entry.path === path && entry.kind === 'folder') ?? false;
      const dir = path === '' ? '' : isFolder ? path : parentOf(path);
      const done = (task: Promise<unknown>) => task.then(() => refresh()).catch(report);
      switch (action.id) {
        case 'new-script':
          createScript(dir)
            .then((created) => setRenamingId(scriptRowId(created)))
            .catch(report);
          break;
        case 'new-folder':
          createFolder(dir)
            .then((created) => {
              setExpanded((current) => new Set(current).add(dir));
              setRenamingId(folderRowId(created));
            })
            .catch(report);
          break;
        case 'rename':
          setRenamingId(isFolder ? folderRowId(path) : scriptRowId(path));
          break;
        case 'pin':
        case 'unpin':
          done(vault.pin(path, action.id === 'pin'));
          break;
        case 'reveal':
          vault.reveal(path).catch(report);
          break;
        case 'trash':
          done(vault.trash(path)).then(() => {
            if (openPath !== null && (openPath === path || openPath.startsWith(`${path}/`))) open(null);
          });
          break;
        default:
          break;
      }
    },
    [openPath, library.tree, refresh, report, createScript, createFolder, open],
  );

  useEffect(() => {
    let active = true;
    let stop: (() => void) | null = null;
    void listen<MenuAction>('menu://action', (event) => act(event.payload)).then((unlisten) => {
      if (active) stop = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      stop?.();
    };
  }, [act]);

  function rename(row: Row, name: string | null) {
    setRenamingId(null);
    if (name === null || (row.kind !== 'script' && row.kind !== 'folder') || name.trim() === '' || name.trim() === row.name) return;
    vault
      .rename(row.path, name.trim())
      .then((renamed) => {
        if (row.kind === 'script') {
          if (openPath === row.path) openScript(renamed);
          else setSelectedId(scriptRowId(renamed));
        } else {
          setSelectedId(folderRowId(renamed));
        }
        return refresh();
      })
      .catch(report);
  }

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
    act({ id: 'new-script', path: openPath === null ? '' : parentOf(openPath) });
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
            selectedId={selectedId ?? (openPath === null ? null : scriptRowId(openPath))}
            openPath={openPath}
            width={shell.sidebarWidth}
            query={query}
            hits={hits}
            searchRef={searchRef}
            renamingId={renamingId}
            onQuery={setQuery}
            onRename={rename}
            onView={setView}
            onToggleFolder={toggleFolder}
            onSelect={setSelectedId}
            onOpen={openScript}
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
          {!folderProblem && !noScripts && openPath !== null && <Preview path={openPath} file={shownFile} />}
          {(error ?? library.error) !== null && !folderProblem && <p className={styles.error}>{error ?? library.error}</p>}
        </section>
      </main>
    </div>
  );
}
