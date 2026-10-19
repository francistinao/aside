// @vitest-environment happy-dom
import { Editor } from '@tiptap/core';
import { describe, expect, it } from 'vitest';
import { editorExtensions } from './extensions';
import { formatEstimate } from './meta';

describe('formatEstimate', () => {
  it('rounds to the nearest 5 seconds and writes minutes and seconds', () => {
    expect(formatEstimate(412, 165, 150)).toBe('412 words · about 2 min 45 sec at 150 wpm');
    expect(formatEstimate(412, 167, 150)).toBe('412 words · about 2 min 45 sec at 150 wpm');
    expect(formatEstimate(412, 178, 150)).toBe('412 words · about 3 min at 150 wpm');
  });

  it('stays in seconds under a minute', () => {
    expect(formatEstimate(100, 44, 150)).toBe('100 words · about 45 sec at 150 wpm');
    expect(formatEstimate(1, 1, 80)).toBe('1 word · about 0 sec at 80 wpm');
  });
});

describe('meta line decoration', () => {
  it('renders after the first H1 and updates without changing the document', () => {
    const editor = new Editor({ extensions: editorExtensions(), content: '# Title\n\nBody text.', contentType: 'markdown' });
    try {
      const before = editor.getMarkdown();
      editor.commands.setMetaLine('2 words · about 0 sec at 150 wpm');
      const line = editor.view.dom.querySelector('.meta-line');
      expect(line?.textContent).toBe('2 words · about 0 sec at 150 wpm');
      expect(line?.previousElementSibling?.tagName).toBe('H1');
      expect(editor.getMarkdown()).toBe(before);
    } finally {
      editor.destroy();
    }
  });

  it('sits at the top when the script has no H1', () => {
    const editor = new Editor({ extensions: editorExtensions(), content: 'Just text.', contentType: 'markdown' });
    try {
      editor.commands.setMetaLine('2 words · about 0 sec at 150 wpm');
      expect(editor.view.dom.firstElementChild?.classList.contains('meta-line')).toBe(true);
    } finally {
      editor.destroy();
    }
  });
});
