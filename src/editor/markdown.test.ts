// @vitest-environment happy-dom
import { Editor } from '@tiptap/core';
import { describe, expect, it } from 'vitest';
import { editorExtensions } from './extensions';
import { fidelityOf, sameMarkdown } from './markdown';

const fixtures = import.meta.glob<string>('../../fixtures/markdown/*.md', { query: '?raw', import: 'default', eager: true });
const fixture = (name: string): string => fixtures[`../../fixtures/markdown/${name}`] ?? '';

function roundTrip(markdown: string): string {
  const editor = new Editor({ extensions: editorExtensions(), content: markdown, contentType: 'markdown' });
  try {
    return editor.getMarkdown();
  } finally {
    editor.destroy();
  }
}

describe('markdown round trip', () => {
  const files = Object.keys(fixtures)
    .map((key) => key.split('/').pop() ?? key)
    .filter((name) => name !== 'unsupported.md');

  it('has the expected fixtures', () => {
    expect(files.sort()).toEqual(['elements.md', 'korean.md', 'pauses.md', 'sample-script.md']);
  });

  for (const name of files) {
    it(`${name} survives load and save byte for byte, except trailing whitespace`, () => {
      const original = fixture(name);
      const saved = roundTrip(original);
      expect(saved.trimEnd()).toBe(original.trimEnd());
      expect(fidelityOf(original, saved)).toBe('exact');
    });
  }

  it('unsupported.md trips the fidelity guard', () => {
    const original = fixture('unsupported.md');
    expect(fidelityOf(original, roundTrip(original))).toBe('lossy');
  });
});

describe('sameMarkdown', () => {
  it('ignores style only differences', () => {
    expect(sameMarkdown('Title\n=====\n\nSome _italic_ and __bold__ text.', '# Title\n\nSome *italic* and **bold** text.')).toBe(true);
    expect(sameMarkdown('* one\n* two', '- one\n\n- two')).toBe(true);
    expect(sameMarkdown('line one\nline two', 'line one line two')).toBe(true);
  });

  it('catches dropped content', () => {
    expect(sameMarkdown('a\n\n| x | y |\n| - | - |\n| 1 | 2 |', 'a')).toBe(false);
    expect(sameMarkdown('a <b>raw</b> b', 'a raw b')).toBe(false);
    expect(sameMarkdown('# One\n\ntext', '## One\n\ntext')).toBe(false);
  });
});
