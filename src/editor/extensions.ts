// The editor's extensions. StarterKit brings headings, bold, italic, lists, link, code,
// code block, blockquote, strike and horizontal rule, so common markdown survives a round
// trip. Underline is off: it has no markdown.

import StarterKit from '@tiptap/starter-kit';
import { OrderedList } from '@tiptap/extension-list';
import { Markdown } from '@tiptap/markdown';
import type { AnyExtension } from '@tiptap/core';
import { LivePreview } from './livePreview';
import { MetaLine } from './meta';
import { Pause } from './pause';

/// The stock ordered list tokenizer also accepts letter and roman markers, so a sentence
/// like "Go. Now click" would turn into a list. Only digits start a list here.
const DIGIT_MARKER = /^\s{0,3}\d{1,9}[.)]\s/;

const original = OrderedList.config.markdownTokenizer;

const DigitOrderedList = original
  ? OrderedList.extend({
      markdownTokenizer: {
        ...original,
        tokenize: (src, tokens, lexer) => (DIGIT_MARKER.test(src) ? original.tokenize(src, tokens, lexer) : undefined),
      },
    })
  : OrderedList;

export function editorExtensions(): AnyExtension[] {
  return [
    StarterKit.configure({
      underline: false,
      orderedList: false,
      heading: { levels: [1, 2, 3] },
      link: { openOnClick: false, autolink: false, linkOnPaste: true },
    }),
    DigitOrderedList,
    Markdown,
    Pause,
    LivePreview,
    MetaLine,
  ];
}
