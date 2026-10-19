import { EditorContent, useEditor } from '@tiptap/react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { MutableRefObject } from 'react';
import { vault } from '../library/vault';
import { createAutosave } from './autosave';
import { editorExtensions } from './extensions';
import { fidelityOf } from './markdown';
import { PAUSE_CLICK_EVENT } from './pause';
import type { PauseClick } from './pause';
import { PausePopover } from './PausePopover';
import { ReadOnlyBanner } from './ReadOnlyBanner';
import { Toolbar } from './Toolbar';
import styles from './editor.module.css';

interface Props {
  path: string;
  name: string;
  content: string;
  onSaved: (modified: number) => void;
  onError: (reason: unknown) => void;
  onToggleSidebar: () => void;
  /// Set so the window can save before it closes or the app quits.
  flushRef: MutableRefObject<(() => Promise<void>) | null>;
}

interface PopoverState {
  pos: number;
  seconds: number;
  left: number;
  top: number;
}

/// One open script: its toolbar, editor and autosave. The parent keys it by path, so
/// switching scripts unmounts this one (which flushes) and mounts a fresh one.
export function ScriptView({ path, name, content, onSaved, onError, onToggleSidebar, flushRef }: Props) {
  const [readOnly, setReadOnly] = useState(false);
  const [popover, setPopover] = useState<PopoverState | null>(null);
  const container = useRef<HTMLDivElement | null>(null);
  const autosave = useMemo(
    () => createAutosave((markdown) => vault.write(path, markdown).then((data) => onSaved(data.modified)), onError),
    [path, onSaved, onError],
  );

  const editor = useEditor({
    extensions: editorExtensions(),
    content,
    contentType: 'markdown',
    editorProps: { attributes: { class: styles.prose ?? '', spellcheck: 'true' } },
    onUpdate: ({ editor: current }) => autosave.schedule(current.getMarkdown()),
    // The guard: a file the editor cannot reproduce opens read only, nothing is lost silently.
    onCreate: ({ editor: created }) => {
      if (fidelityOf(content, created.getMarkdown()) === 'lossy') {
        created.setEditable(false);
        setReadOnly(true);
      }
    },
  });

  useEffect(() => {
    flushRef.current = () => autosave.flush();
    const onBlur = () => void autosave.flush();
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('blur', onBlur);
      flushRef.current = null;
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
      <Toolbar editor={readOnly ? null : editor} name={name} onToggleSidebar={onToggleSidebar} />
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
