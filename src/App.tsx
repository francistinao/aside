import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Editor } from '@tiptap/core';
import { useCallback, useEffect, useRef, useState } from 'react';
import styles from './App.module.css';
import { ControlBar } from './controls/ControlBar';
import type { CaptureStatus, PrompterSettings } from './controls/ControlBar';
import { ScriptView } from './editor/ScriptView';
import { Toolbar } from './editor/Toolbar';
import { EmptyLibrary } from './library/EmptyLibrary';
import { FolderScreen } from './library/FolderScreen';
import { Sidebar } from './library/Sidebar';
import { folderRowId, scriptRowId } from './library/tree';
import type { Row, SmartId } from './library/tree';
import { useVault } from './library/useVault';
import { describeError, parentOf, vault } from './library/vault';
import type { FileData, Hit } from './library/vault';
import { useShortcuts } from './shell/useShortcuts';

interface ShellSettings extends PrompterSettings {
  sidebarWidth: number;
  sidebarVisible: boolean;
}

const DEFAULT_SHELL: ShellSettings = {
  sidebarWidth: 240,
  sidebarVisible: true,
  wpm: 150,
  highlight: 'word',
  opacity: 85,
  corner: 'bottom-right',
  hideFromCapture: true,
};
const PERSIST_DELAY_MS = 150;

type SessionState = 'idle' | 'countdown' | 'running' | 'paused' | 'ended';

/// What settings_get and settings://changed carry, beyond the shell's own fields.
interface SettingsPayload extends ShellSettings {
  scriptsFolder: string | null;
}

const SHELL_KEYS = ['sidebarWidth', 'sidebarVisible', 'wpm', 'highlight', 'opacity', 'corner', 'hideFromCapture'] as const;

function shellOf(value: SettingsPayload): ShellSettings {
  return {
    sidebarWidth: value.sidebarWidth,
    sidebarVisible: value.sidebarVisible,
    wpm: value.wpm,
    highlight: value.highlight,
    opacity: value.opacity,
    corner: value.corner,
    hideFromCapture: value.hideFromCapture,
  };
}

/// Takes the values another window or the menu bar extra changed, except the ones this
/// window is still about to write.
function mergeShell(current: ShellSettings, incoming: ShellSettings, pending: Partial<ShellSettings>): ShellSettings {
  const merged = { ...current };
  for (const key of SHELL_KEYS) {
    if (!(key in pending)) Object.assign(merged, { [key]: incoming[key] });
  }
  return merged;
}

interface MenuAction {
  id: string;
  path?: string;
}

interface FlushRequest {
  then: 'close' | 'quit';
}

function scriptName(path: string): string {
  return path.split('/').pop()?.replace(/\.md$/, '') ?? path;
}

export function App() {
  const [shell, setShell] = useState<ShellSettings>(DEFAULT_SHELL);
  const [shellLoaded, setShellLoaded] = useState(false);
  const [sessionState, setSessionState] = useState<SessionState>('idle');
  const [capture, setCapture] = useState<CaptureStatus>('visible');
  const pendingPatch = useRef<Partial<ShellSettings>>({});
  const scriptsFolder = useRef<string | null>(null);
  const [view, setView] = useState<SmartId>('all');
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [hits, setHits] = useState<Hit[]>([]);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [file, setFile] = useState<{ path: string; data: FileData } | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const flushRef = useRef<(() => Promise<void>) | null>(null);
  const editorRef = useRef<Editor | null>(null);
  const library = useVault();
  const { open, openPath, refresh, createScript, createFolder } = library;
  const report = useCallback((reason: unknown) => setError(describeError(reason)), []);

  useEffect(() => {
    let active = true;
    invoke<SettingsPayload>('settings_get')
      .then((value) => {
        if (!active) return;
        scriptsFolder.current = value.scriptsFolder;
        setShell(shellOf(value));
        setShellLoaded(true);
      })
      .catch(report);
    invoke<CaptureStatus>('capture_status').then(setCapture).catch(report);
    return () => {
      active = false;
    };
  }, [report]);

  // Every change shows at once and is written after a short rest, so a slider drag or a
  // sidebar resize does not hit the store on every pixel.
  useEffect(() => {
    if (!shellLoaded) return;
    const timer = setTimeout(() => {
      const patch = pendingPatch.current;
      pendingPatch.current = {};
      if (Object.keys(patch).length > 0) invoke('settings_set', { patch }).catch(report);
    }, PERSIST_DELAY_MS);
    return () => clearTimeout(timer);
  }, [shell, shellLoaded, report]);

  const changeSettings = useCallback((patch: Partial<ShellSettings>) => {
    pendingPatch.current = { ...pendingPatch.current, ...patch };
    setShell((current) => ({ ...current, ...patch }));
  }, []);

  const toggleSidebar = useCallback(() => {
    setShell((current) => {
      pendingPatch.current = { ...pendingPatch.current, sidebarVisible: !current.sidebarVisible };
      return { ...current, sidebarVisible: !current.sidebarVisible };
    });
  }, []);

  const find = useCallback(() => {
    changeSettings({ sidebarVisible: true });
    searchRef.current?.focus();
    searchRef.current?.select();
  }, [changeSettings]);

  // The prompter's state and capture status drive the Start button and the switch tooltip.
  useEffect(() => {
    let active = true;
    const stops: (() => void)[] = [];
    const keep = (stop: () => void) => {
      if (active) stops.push(stop);
      else stop();
    };
    void listen<{ state: SessionState }>('session://state', (event) => setSessionState(event.payload.state)).then(keep);
    void listen<CaptureStatus>('capture://status', (event) => setCapture(event.payload)).then(keep);
    void listen<SettingsPayload>('settings://changed', (event) => {
      const incoming = event.payload;
      setShell((current) => mergeShell(current, shellOf(incoming), pendingPatch.current));
      if (incoming.scriptsFolder !== scriptsFolder.current) {
        scriptsFolder.current = incoming.scriptsFolder;
        void refresh();
      }
    }).then(keep);
    return () => {
      active = false;
      for (const stop of stops) stop();
    };
  }, [refresh]);

  const startSession = useCallback(() => {
    if (openPath === null) return;
    const flush = flushRef.current?.() ?? Promise.resolve();
    void flush
      .then(() => {
        const markdown = editorRef.current?.getMarkdown() ?? '';
        return invoke('session_start', { markdown, path: openPath });
      })
      .catch(report);
  }, [openPath, report]);

  const stopSession = useCallback(() => {
    invoke('session_control', { action: 'stop' }).catch(report);
  }, [report]);

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

  const saved = useCallback((modified: number, content: string) => {
    setFile((current) => (current === null ? current : { ...current, data: { content, modified } }));
  }, []);

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

  // Rust asks for a flush before the window closes or the app quits, then finishes the job.
  useEffect(() => {
    let active = true;
    let stop: (() => void) | null = null;
    void listen<FlushRequest>('library://flush', (event) => {
      const flush = flushRef.current?.() ?? Promise.resolve();
      void flush.finally(() => invoke('library_flushed', { then: event.payload.then }).catch(report));
    }).then((unlisten) => {
      if (active) stop = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      stop?.();
    };
  }, [report]);

  /// Every menu item, context menu item and keyboard shortcut lands here.
  const act = useCallback(
    (action: MenuAction) => {
      const path = action.path ?? openPath ?? '';
      const isFolder = library.tree?.entries.some((entry) => entry.path === path && entry.kind === 'folder') ?? false;
      const dir = path === '' ? '' : isFolder ? path : parentOf(path);
      const done = (task: Promise<unknown>) => task.then(() => refresh()).catch(report);
      const chain = () => editorRef.current?.chain().focus();
      switch (action.id) {
        case 'session.toggle':
          if (sessionState === 'idle') startSession();
          else stopSession();
          break;
        case 'view.toggle-sidebar':
          toggleSidebar();
          break;
        case 'edit.find':
          find();
          break;
        case 'file.show-folder':
          vault.reveal('').catch(report);
          break;
        case 'format.bold':
          chain()?.toggleBold().run();
          break;
        case 'format.italic':
          chain()?.toggleItalic().run();
          break;
        case 'format.title':
          chain()?.toggleHeading({ level: 1 }).run();
          break;
        case 'format.heading':
          chain()?.toggleHeading({ level: 2 }).run();
          break;
        case 'format.subheading':
          chain()?.toggleHeading({ level: 3 }).run();
          break;
        case 'format.body':
          chain()?.setParagraph().run();
          break;
        case 'format.bulleted-list':
          chain()?.toggleBulletList().run();
          break;
        case 'format.numbered-list':
          chain()?.toggleOrderedList().run();
          break;
        case 'format.insert-pause':
          chain()?.insertPause().run();
          break;
        case 'file.new-script':
        case 'new-script':
          createScript(dir)
            .then((created) => setRenamingId(scriptRowId(created)))
            .catch(report);
          break;
        case 'file.new-folder':
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
    [openPath, library.tree, refresh, report, createScript, createFolder, open, toggleSidebar, find, sessionState, startSession, stopSession],
  );

  const dispatch = useCallback((id: string) => act({ id }), [act]);
  useShortcuts(dispatch);

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

  function toggleFolder(path: string, isOpen: boolean) {
    setExpanded((current) => {
      const next = new Set(current);
      if (isOpen) next.add(path);
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
  const showScript = !folderProblem && !noScripts && openPath !== null;

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
            onWidth={(value) => changeSettings({ sidebarWidth: value })}
            onNewScript={newScript}
          />
        )}
      </aside>
      <main className={styles.main}>
        {showScript && openPath !== null && shownFile !== null ? (
          <>
            <ScriptView
              key={openPath}
              path={openPath}
              name={scriptName(openPath)}
              content={shownFile.content}
              modified={shownFile.modified}
              wpm={shell.wpm}
              onSaved={saved}
              onError={report}
              onToggleSidebar={toggleSidebar}
              flushRef={flushRef}
              editorRef={editorRef}
            />
            <ControlBar
              settings={shell}
              capture={capture}
              running={sessionState !== 'idle'}
              onChange={changeSettings}
              onStart={startSession}
              onStop={stopSession}
            />
          </>
        ) : (
          <>
            <Toolbar editor={null} name={null} modified={null} onToggleSidebar={toggleSidebar} onError={report} />
            <section className={styles.content}>
              {folderProblem && <FolderScreen missing={library.phase === 'missing'} onChoose={() => void library.chooseFolder()} />}
              {noScripts && <EmptyLibrary onNewScript={newScript} />}
            </section>
          </>
        )}
        {(error ?? library.error) !== null && !folderProblem && <p className={styles.error}>{error ?? library.error}</p>}
      </main>
    </div>
  );
}
