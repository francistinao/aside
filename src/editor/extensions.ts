// The editor's extensions. StarterKit brings headings, bold, italic, lists, link, code,
// code block, blockquote, strike and horizontal rule, so common markdown survives a round
// trip. Underline is off: it has no markdown.

import StarterKit from '@tiptap/starter-kit';
import { Markdown } from '@tiptap/markdown';
import type { AnyExtension } from '@tiptap/core';

export function editorExtensions(): AnyExtension[] {
  return [
    StarterKit.configure({
      underline: false,
      heading: { levels: [1, 2, 3] },
      link: { openOnClick: false, autolink: false, linkOnPaste: true },
    }),
    Markdown,
  ];
}
