// The pause marker: an inline atom with a seconds attribute. In markdown it is `{pause 2s}`,
// in the editor it is a chip. Clicking the chip raises a DOM event the view turns into a popover.

import { Node, mergeAttributes, nodeInputRule } from '@tiptap/core';
import type { JSONContent } from '@tiptap/core';

export const MIN_PAUSE = 0.5;
export const MAX_PAUSE = 10;
export const PAUSE_STEP = 0.5;
export const DEFAULT_PAUSE = 2;

const PAUSE_AT_START = /^\{pause\s+(\d+(?:\.\d+)?)s\}/;
const PAUSE_TYPED = /\{pause\s+(\d+(?:\.\d+)?)s\}$/;

export const PAUSE_CLICK_EVENT = 'aside:pause-click';

export interface PauseClick {
  pos: number;
  seconds: number;
  rect: DOMRect;
}

export function clampPause(seconds: number): number {
  if (!Number.isFinite(seconds)) return DEFAULT_PAUSE;
  const stepped = Math.round(seconds / PAUSE_STEP) * PAUSE_STEP;
  return Math.min(MAX_PAUSE, Math.max(MIN_PAUSE, stepped));
}

/// "2" or "1.5": no trailing zeros, as the file writes it.
export function formatPause(seconds: number): string {
  return String(clampPause(seconds));
}

export function renderPauseMarkdown(seconds: number): string {
  return `{pause ${formatPause(seconds)}s}`;
}

export function pauseLabel(seconds: number): string {
  return `Pause ${formatPause(seconds)}s`;
}

/// A marked style tokenizer: matches a marker at the start of the remaining text.
export function tokenizePause(src: string): { type: 'pause'; raw: string; seconds: number } | undefined {
  const match = PAUSE_AT_START.exec(src);
  if (match === null) return undefined;
  return { type: 'pause', raw: match[0], seconds: Number(match[1]) };
}

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    pause: {
      insertPause: (seconds?: number) => ReturnType;
      setPauseSeconds: (pos: number, seconds: number) => ReturnType;
    };
  }
}

// Lucide "pause", drawn by hand because the node view is plain DOM.
const PAUSE_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="14" y="3" width="5" height="18" rx="1"/><rect x="5" y="3" width="5" height="18" rx="1"/></svg>';

export const Pause = Node.create({
  name: 'pause',
  group: 'inline',
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      seconds: {
        default: DEFAULT_PAUSE,
        parseHTML: (element) => clampPause(Number(element.getAttribute('data-seconds'))),
        renderHTML: (attributes) => ({ 'data-seconds': formatPause(Number(attributes.seconds)) }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'span[data-pause]' }];
  },

  renderHTML({ node, HTMLAttributes }) {
    return ['span', mergeAttributes({ 'data-pause': '', class: 'pause-chip' }, HTMLAttributes), pauseLabel(Number(node.attrs.seconds))];
  },

  markdownTokenName: 'pause',
  markdownTokenizer: {
    name: 'pause',
    level: 'inline',
    start: (src: string) => src.indexOf('{pause'),
    tokenize: (src: string) => tokenizePause(src),
  },
  parseMarkdown: (token, helpers) => helpers.createNode('pause', { seconds: clampPause(Number(token.seconds)) }),
  renderMarkdown: (node: JSONContent) => renderPauseMarkdown(Number(node.attrs?.seconds ?? DEFAULT_PAUSE)),

  addCommands() {
    return {
      insertPause:
        (seconds = DEFAULT_PAUSE) =>
        ({ commands }) =>
          commands.insertContent({ type: this.name, attrs: { seconds: clampPause(seconds) } }),
      setPauseSeconds:
        (pos, seconds) =>
        ({ tr, state }) => {
          const node = state.doc.nodeAt(pos);
          if (node === null || node.type.name !== this.name) return false;
          tr.setNodeMarkup(pos, undefined, { seconds: clampPause(seconds) });
          return true;
        },
    };
  },

  addInputRules() {
    return [nodeInputRule({ find: PAUSE_TYPED, type: this.type, getAttributes: (match) => ({ seconds: clampPause(Number(match[1])) }) })];
  },

  addKeyboardShortcuts() {
    return { 'Mod-Alt-p': () => this.editor.commands.insertPause() };
  },

  addNodeView() {
    return ({ node, getPos }) => {
      const dom = document.createElement('span');
      dom.className = 'pause-chip';
      dom.contentEditable = 'false';
      dom.setAttribute('data-pause', '');
      dom.innerHTML = PAUSE_ICON;
      const label = document.createElement('span');
      label.textContent = pauseLabel(Number(node.attrs.seconds));
      dom.append(label);
      dom.addEventListener('click', (event) => {
        event.preventDefault();
        const pos = getPos();
        if (pos === undefined) return;
        const detail: PauseClick = { pos, seconds: Number(label.dataset.seconds ?? node.attrs.seconds), rect: dom.getBoundingClientRect() };
        dom.dispatchEvent(new CustomEvent<PauseClick>(PAUSE_CLICK_EVENT, { bubbles: true, detail }));
      });
      return {
        dom,
        update(updated) {
          if (updated.type.name !== 'pause') return false;
          label.textContent = pauseLabel(Number(updated.attrs.seconds));
          label.dataset.seconds = String(updated.attrs.seconds);
          return true;
        },
      };
    };
  },
});
