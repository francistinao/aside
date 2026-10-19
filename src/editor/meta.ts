// The meta line under the first H1: "412 words · about 2 min 45 sec at 150 wpm".
// A decoration fed by the timing engine's estimate, never saved.

import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import type { EditorState } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';

const key = new PluginKey<string>('metaLine');

/// Seconds round to the nearest 5. Under a minute reads "about 45 sec".
export function formatEstimate(words: number, seconds: number, wpm: number): string {
  const rounded = Math.max(0, Math.round(seconds / 5) * 5);
  const minutes = Math.floor(rounded / 60);
  const rest = rounded % 60;
  const time = minutes === 0 ? `about ${rest} sec` : rest === 0 ? `about ${minutes} min` : `about ${minutes} min ${rest} sec`;
  return `${words} ${words === 1 ? 'word' : 'words'} · ${time} at ${wpm} wpm`;
}

/// Right after the first H1, or at the top when there is none.
function metaPosition(state: EditorState): number {
  const first = state.doc.firstChild;
  if (first !== null && first.type.name === 'heading' && Number(first.attrs.level) === 1) return first.nodeSize;
  return 0;
}

function decorate(state: EditorState, text: string): DecorationSet {
  if (text === '') return DecorationSet.empty;
  const widget = Decoration.widget(
    metaPosition(state),
    () => {
      const line = document.createElement('div');
      line.className = 'meta-line';
      line.textContent = text;
      line.contentEditable = 'false';
      return line;
    },
    { side: -1, key: `meta:${text}` },
  );
  return DecorationSet.create(state.doc, [widget]);
}

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    metaLine: {
      setMetaLine: (text: string) => ReturnType;
    };
  }
}

export const MetaLine = Extension.create({
  name: 'metaLine',

  addCommands() {
    return {
      setMetaLine:
        (text) =>
        ({ tr, dispatch }) => {
          tr.setMeta(key, text);
          dispatch?.(tr);
          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin<string>({
        key,
        state: {
          init: () => '',
          apply: (tr, old) => {
            const next = tr.getMeta(key);
            return typeof next === 'string' ? next : old;
          },
        },
        props: {
          decorations(state) {
            return decorate(state, key.getState(state) ?? '');
          },
        },
      }),
    ];
  },
});
