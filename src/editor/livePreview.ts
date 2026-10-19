// Live Preview: the markdown markers show only where the caret is, as decorations that are
// never part of the document. `#` before a heading, `**` around bold, `*` around italic.

import { Extension, getMarkRange } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import type { EditorState } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';

const key = new PluginKey<DecorationSet>('livePreview');

const MARKERS: [string, string][] = [
  ['bold', '**'],
  ['italic', '*'],
];

function marker(text: string): HTMLElement {
  const span = document.createElement('span');
  span.className = 'md-marker';
  span.textContent = text;
  return span;
}

export function markersAtCaret(state: EditorState): DecorationSet {
  const { $from } = state.selection;
  const decorations: Decoration[] = [];
  const parent = $from.parent;
  if (parent.type.name === 'heading') {
    const level = Number(parent.attrs.level);
    decorations.push(Decoration.widget($from.start(), () => marker(`${'#'.repeat(level)} `), { side: -1, key: `h${level}` }));
  }
  for (const [name, text] of MARKERS) {
    const type = state.schema.marks[name];
    if (type === undefined) continue;
    const range = getMarkRange($from, type);
    if (range === undefined) continue;
    decorations.push(Decoration.widget(range.from, () => marker(text), { side: -1, key: `${name}-open` }));
    decorations.push(Decoration.widget(range.to, () => marker(text), { side: 1, key: `${name}-close` }));
  }
  return DecorationSet.create(state.doc, decorations);
}

export const LivePreview = Extension.create({
  name: 'livePreview',
  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key,
        state: {
          init: (_, state) => markersAtCaret(state),
          apply: (tr, old, _oldState, newState) => (tr.docChanged || tr.selectionSet ? markersAtCaret(newState) : old),
        },
        props: {
          decorations(state) {
            return key.getState(state) ?? DecorationSet.empty;
          },
        },
      }),
    ];
  },
});
