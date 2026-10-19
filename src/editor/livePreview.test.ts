// @vitest-environment happy-dom
import { Editor } from '@tiptap/core';
import { TextSelection } from '@tiptap/pm/state';
import { describe, expect, it } from 'vitest';
import { editorExtensions } from './extensions';

function markersAt(editor: Editor, pos: number): string[] {
  editor.view.dispatch(editor.state.tr.setSelection(TextSelection.create(editor.state.doc, pos)));
  return Array.from(editor.view.dom.querySelectorAll('.md-marker')).map((node) => node.textContent ?? '');
}

describe('live preview markers', () => {
  it('shows heading and emphasis markers only around the caret', () => {
    const editor = new Editor({ extensions: editorExtensions(), content: '## Head\n\nSome **bold** and *it* here.', contentType: 'markdown' });
    try {
      expect(markersAt(editor, 2)).toEqual(['## ']);
      const paragraphStart = editor.state.doc.firstChild?.nodeSize ?? 0;
      expect(markersAt(editor, paragraphStart + 1)).toEqual([]);
      expect(markersAt(editor, paragraphStart + 8)).toEqual(['**', '**']);
      expect(markersAt(editor, paragraphStart + 17)).toEqual(['*', '*']);
      expect(editor.state.doc.textContent).toBe('HeadSome bold and it here.');
    } finally {
      editor.destroy();
    }
  });
});
