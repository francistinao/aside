// Times keystrokes in a 20,000 word script with the real editor extensions in WebKit.
// A keystroke is one ProseMirror insertText transaction plus the layout it forces before
// the next frame. docs/PERFORMANCE.md asks for under 16 ms each.
import { Editor } from '@tiptap/core';
import { Marked, marked } from 'marked';
import { editorExtensions } from '../../src/editor/extensions';
import '../../src/styles/tokens.css';
import '../../src/styles/library.css';
import '../../src/editor/editor.module.css';

const WORDS = ['Now', 'click', 'the', 'settings', 'cog', 'and', 'choose', 'a', 'folder', 'for', 'your', 'scripts'];

function script(words: number, plain: boolean): string {
  const lines: string[] = ['# A long walkthrough', ''];
  let count = 0;
  let paragraph = 0;
  while (count < words) {
    if (paragraph % 8 === 0) lines.push(`## Section ${paragraph / 8 + 1}`, '');
    const parts: string[] = [];
    for (let i = 0; i < 100 && count < words; i += 1, count += 1) {
      const word = WORDS[(count + paragraph) % WORDS.length] ?? 'now';
      parts.push(!plain && i === 3 ? `**${word}**` : !plain && i === 40 ? `*${word}*` : word);
    }
    lines.push(`${parts.join(' ')}.`, '');
    if (!plain) lines.push('{pause 1s}', '');
    paragraph += 1;
  }
  return lines.join('\n');
}

function nextFrame(): Promise<number> {
  return new Promise((done) => {
    requestAnimationFrame(() => {
      // Force style and layout so the measurement includes them.
      void document.body.offsetHeight;
      done(performance.now());
    });
  });
}

function stats(samples: number[]): { avg: number; p95: number; max: number } {
  const sorted = [...samples].sort((a, b) => a - b);
  const avg = sorted.reduce((sum, value) => sum + value, 0) / sorted.length;
  return { avg, p95: sorted[Math.floor(sorted.length * 0.95)] ?? 0, max: sorted[sorted.length - 1] ?? 0 };
}

declare global {
  interface Window {
    measure: () => Promise<string>;
  }
}

// ?words=5000 shrinks the script; ?skip=livePreview,metaLine drops extensions by name;
// ?plain=1 leaves out bold, italic and pauses.
window.measure = async () => {
  const params = new URLSearchParams(location.search);
  const skip = new Set((params.get('skip') ?? '').split(',').filter(Boolean));
  const markdown = script(Number(params.get('words') ?? 20_000), params.get('plain') === '1');
  const element = document.getElementById('editor');
  if (element === null) throw new Error('no editor element');
  const extensions = editorExtensions().filter((extension) => !skip.has(extension.name));
  const phases: Record<string, number> = {};
  let phase = performance.now();
  const editor = new Editor({ element, extensions });
  phases.createMs = Math.round(performance.now() - phase);
  phase = performance.now();
  const tokens = marked.lexer(markdown);
  phases.lexMs = Math.round(performance.now() - phase);
  phases.blocks = tokens.length;
  // A fresh instance has none of the editor's tokenizers.
  phase = performance.now();
  new Marked().lexer(markdown);
  phases.freshLexMs = Math.round(performance.now() - phase);
  phase = performance.now();
  const loadStart = performance.now();
  editor.commands.setContent(markdown, { contentType: 'markdown', emitUpdate: false });
  phases.setContentMs = Math.round(performance.now() - phase);
  const loadEnd = await nextFrame();
  phases.firstFrameMs = Math.round(loadEnd - loadStart - phases.setContentMs);
  const words = editor.getText().split(/\s+/).filter(Boolean).length;

  // Type in the middle of the document, where every transaction shifts the second half.
  editor.commands.focus(Math.floor(editor.state.doc.content.size / 2));
  await nextFrame();
  // The keystroke cost is the synchronous work: the transaction, the DOM update and the
  // style and layout it forces. The time to the next frame is paced by the display.
  const samples: number[] = [];
  const frames: number[] = [];
  for (let i = 0; i < 100; i += 1) {
    const started = performance.now();
    const { tr } = editor.state;
    editor.view.dispatch(tr.insertText(i % 7 === 6 ? ' ' : 'a'));
    void document.body.offsetHeight;
    samples.push(performance.now() - started);
    frames.push((await nextFrame()) - started);
  }

  const serializeStart = performance.now();
  const out = editor.getMarkdown();
  const serializeMs = performance.now() - serializeStart;
  editor.destroy();
  return JSON.stringify({
    words,
    skipped: [...skip],
    phases,
    loadMs: Math.round(loadEnd - loadStart),
    keystrokeMs: stats(samples),
    toNextFrameMs: stats(frames),
    serializeMs: Math.round(serializeMs * 10) / 10,
    serializedChars: out.length,
  });
};
