import { invoke } from '@tauri-apps/api/core';
import type { Editor } from '@tiptap/core';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { EditorContent, useEditor } from '@tiptap/react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { MutableRefObject } from 'react';
import { vault } from '../library/vault';
import { createAutosave } from './autosave';
import { editorExtensions } from './extensions';
import { fidelityOf } from './markdown';
import { formatEstimate } from './meta';
import { PAUSE_CLICK_EVENT } from './pause';
import type { PauseClick } from './pause';
import { PausePopover } from './PausePopover';
import { ReadOnlyBanner } from './ReadOnlyBanner';
import { Toolbar } from './Toolbar';
import styles from './editor.module.css';

const ESTIMATE_DELAY_MS = 300;

interface Estimate {
  words: number;
  seconds: number;
}

interface Props {
  path: string;
  name: string;
  content: string;
  /// The file's modification time. A change while the script is open means it was edited elsewhere.
  modified: number;
  wpm: number;
  onSaved: (modified: number) => void;
  onError: (reason: unknown) => void;
  onToggleSidebar: () => void;
  /// Set so the window can save before it closes or the app quits.
  flushRef: MutableRefObject<(() => Promise<void>) | null>;
  /// Set so menu and keyboard actions can reach the editor.
  editorRef: MutableRefObject<Editor | null>;
}

type ConflictChoice = 'keep' | 'load';

interface PopoverState {
  pos: number;
  seconds: number;
  left: number;
  top: number;
}

/// One open script: its toolbar, editor and autosave. The parent keys it by path, so
/// switching scripts unmounts this one (which flushes) and mounts a fresh one.
export function ScriptView({ path, name, content, modified, wpm, onSaved, onError, onToggleSidebar, flushRef, editorRef }: Props) {
  const [readOnly, setReadOnly] = useState(false);
  const [popover, setPopover] = useState<PopoverState | null>(null);
  const container = useRef<HTMLDivElement | null>(null);
  // Serialized once per document version, shared by the estimate and the save.
  const serialized = useRef<{ doc: ProseMirrorNode; markdown: string } | null>(null);
  const estimateTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const autosave = useMemo(
    () => createAutosave((markdown) => vault.write(path, markdown).then((data) => onSaved(data.modified)), onError),
    [path, onSaved, onError],
  );

  const serialize = useCallback((current: Editor): string => {
    if (serialized.current?.doc !== current.state.doc) {
      serialized.current = { doc: current.state.doc, markdown: current.getMarkdown() };
    }
    return serialized.current.markdown;
  }, []);

  const requestEstimate = useCallback(
    (current: Editor) => {
      if (estimateTimer.current !== null) clearTimeout(estimateTimer.current);
      estimateTimer.current = setTimeout(() => {
        estimateTimer.current = null;
        if (current.isDestroyed) return;
        invoke<Estimate>('session_estimate', { markdown: serialize(current), wpm })
          .then((estimate) => {
            if (!current.isDestroyed) current.commands.setMetaLine(formatEstimate(estimate.words, estimate.seconds, wpm));
          })
          .catch(onError);
      }, ESTIMATE_DELAY_MS);
    },
    [serialize, wpm, onError],
  );

  const editor = useEditor({
    extensions: editorExtensions(),
    content,
    contentType: 'markdown',
    editorProps: { attributes: { class: styles.prose ?? '', spellcheck: 'true' } },
    onUpdate: ({ editor: current }) => {
      autosave.schedule(() => serialize(current));
      requestEstimate(current);
    },
    // The guard: a file the editor cannot reproduce opens read only, nothing is lost silently.
    onCreate: ({ editor: created }) => {
      if (fidelityOf(content, serialize(created)) === 'lossy') {
        created.setEditable(false);
        setReadOnly(true);
      }
      requestEstimate(created);
    },
  });

  // A speed change re-estimates the open script.
  useEffect(() => {
    requestEstimate(editor);
  }, [editor, requestEstimate]);

  useEffect(() => {
    editorRef.current = editor;
    return () => {
      editorRef.current = null;
    };
  }, [editor, editorRef]);

  // The file changed on disk. With no unsaved edits it reloads quietly; otherwise the user
  // chooses between their edits and the version on disk.
  const seenModified = useRef(modified);
  useEffect(() => {
    if (modified === seenModified.current) return;
    seenModified.current = modified;
    const load = () => {
      autosave.cancel();
      editor.commands.setContent(content, { contentType: 'markdown', emitUpdate: false });
      serialized.current = null;
      requestEstimate(editor);
    };
    if (!autosave.pending()) {
      load();
      return;
    }
    invoke<ConflictChoice>('vault_conflict_dialog', { name })
      .then((choice) => {
        if (choice === 'load') load();
        else void autosave.flush();
      })
      .catch(onError);
  }, [modified, content, name, autosave, editor, requestEstimate, onError]);

  useEffect(() => {
    flushRef.current = () => autosave.flush();
    const onBlur = () => void autosave.flush();
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('blur', onBlur);
      flushRef.current = null;
      if (estimateTimer.current !== null) clearTimeout(estimateTimer.current);
      void autosave.flush();
    };
  }, [autosave, flushRef]);

  // A chip click bubbles up from the node view with its position and rectangle.
  useEffect(() => {
    const host = container.current;
    if (host === null) return;
    function onPauseClick(event: Event) {
      const { detail } = event as CustomEvent<PauseClick>;
      const bounds = host?.getBoundingClientRect();
      if (bounds === undefined) return;
      setPopover({ pos: detail.pos, seconds: detail.seconds, left: detail.rect.left - bounds.left, top: detail.rect.bottom - bounds.top + 6 });
    }
    host.addEventListener(PAUSE_CLICK_EVENT, onPauseClick);
    return () => host.removeEventListener(PAUSE_CLICK_EVENT, onPauseClick);
  }, []);

  function editAnyway() {
    editor.setEditable(true);
    setReadOnly(false);
    editor.commands.focus();
  }

  function changePause(seconds: number) {
    if (popover === null) return;
    editor.commands.setPauseSeconds(popover.pos, seconds);
    setPopover({ ...popover, seconds });
  }

  return (
    <>
      <Toolbar editor={readOnly ? null : editor} name={name} modified={modified} onToggleSidebar={onToggleSidebar} onError={onError} />
      <section className={styles.scroll}>
        <div ref={container} className={styles.editor}>
          {readOnly && <ReadOnlyBanner onEditAnyway={editAnyway} />}
          <EditorContent editor={editor} />
          {popover !== null && (
            <PausePopover seconds={popover.seconds} left={popover.left} top={popover.top} onChange={changePause} onClose={() => setPopover(null)} />
          )}
        </div>
      </section>
    </>
  );
}
