// @vitest-environment happy-dom
import { Editor } from '@tiptap/core';
import { describe, expect, it } from 'vitest';
import { editorExtensions } from './extensions';
import { clampPause, formatPause, renderPauseMarkdown, tokenizePause } from './pause';

describe('pause marker', () => {
  it('tokenizes valid markers only at the start of the text', () => {
    expect(tokenizePause('{pause 2s} next')).toEqual({ type: 'pause', raw: '{pause 2s}', seconds: 2 });
    expect(tokenizePause('{pause 1.5s}')).toEqual({ type: 'pause', raw: '{pause 1.5s}', seconds: 1.5 });
    expect(tokenizePause('{pause two}')).toBeUndefined();
    expect(tokenizePause('{pause 2}')).toBeUndefined();
    expect(tokenizePause('text {pause 2s}')).toBeUndefined();
  });

  it('clamps and steps seconds', () => {
    expect(clampPause(0.1)).toBe(0.5);
    expect(clampPause(99)).toBe(10);
    expect(clampPause(1.3)).toBe(1.5);
    expect(clampPause(Number.NaN)).toBe(2);
    expect(formatPause(2)).toBe('2');
    expect(formatPause(1.5)).toBe('1.5');
    expect(renderPauseMarkdown(0.5)).toBe('{pause 0.5s}');
  });

  it('round trips through the editor as a node', () => {
    const editor = new Editor({ extensions: editorExtensions(), content: 'Go. {pause 2s} Now. {pause 0.5s}', contentType: 'markdown' });
    try {
      const nodes: { type: string; seconds?: number }[] = [];
      editor.state.doc.descendants((node) => {
        if (node.type.name === 'pause') nodes.push({ type: 'pause', seconds: Number(node.attrs.seconds) });
      });
      expect(nodes).toEqual([
        { type: 'pause', seconds: 2 },
        { type: 'pause', seconds: 0.5 },
      ]);
      expect(editor.getMarkdown().trimEnd()).toBe('Go. {pause 2s} Now. {pause 0.5s}');
      editor.commands.insertPause(99);
      expect(editor.getMarkdown()).toContain('{pause 10s}');
    } finally {
      editor.destroy();
    }
  });

  it('leaves malformed markers as text', () => {
    const editor = new Editor({ extensions: editorExtensions(), content: 'Wait {pause two} here', contentType: 'markdown' });
    try {
      expect(editor.state.doc.textContent).toBe('Wait {pause two} here');
      expect(editor.getMarkdown().trimEnd()).toBe('Wait {pause two} here');
    } finally {
      editor.destroy();
    }
  });
});
