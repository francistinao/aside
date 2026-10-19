import { EditorContent, useEditor } from '@tiptap/react';
import { useEffect, useMemo } from 'react';
import type { MutableRefObject } from 'react';
import { vault } from '../library/vault';
import { createAutosave } from './autosave';
import { editorExtensions } from './extensions';
import styles from './editor.module.css';

interface Props {
  path: string;
  content: string;
  onSaved: (modified: number) => void;
  onError: (reason: unknown) => void;
  /// Set so the window can save before it closes or the app quits.
  flushRef: MutableRefObject<(() => Promise<void>) | null>;
}

/// One instance per script: the parent keys it by path, so switching scripts unmounts
/// this one (which flushes) and mounts a fresh one with the new content.
export function Editor({ path, content, onSaved, onError, flushRef }: Props) {
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

  return (
    <div className={styles.editor}>
      <EditorContent editor={editor} />
    </div>
  );
}
