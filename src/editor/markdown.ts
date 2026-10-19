// The fidelity guard. Markdown the editor cannot hold (tables, raw HTML, footnotes) must not
// be lost silently, so a file is compared with its own round trip as two token trees, not
// as bytes. Style differences (`_` versus `*`, setext headings, list spacing) do not count.

import { marked } from 'marked';
import type { Token, Tokens } from 'marked';

export type Fidelity = 'exact' | 'lossy';

type Shape = Record<string, unknown>;

/// True when both texts describe the same document.
export function sameMarkdown(a: string, b: string): boolean {
  return JSON.stringify(normalize(marked.lexer(a))) === JSON.stringify(normalize(marked.lexer(b)));
}

export function fidelityOf(original: string, roundTrip: string): Fidelity {
  return sameMarkdown(original, roundTrip) ? 'exact' : 'lossy';
}

/// Strips positions and raw text, merges adjacent text, drops blank lines and list looseness.
export function normalize(tokens: Token[]): Shape[] {
  const out: Shape[] = [];
  for (const token of tokens) {
    const shape = shapeOf(token);
    if (shape === null) continue;
    const last = out[out.length - 1];
    if (shape.t === 'text' && last?.t === 'text') {
      last.text = collapse(`${String(last.text)} ${String(shape.text)}`);
      continue;
    }
    out.push(shape);
  }
  return out;
}

function collapse(text: string): string {
  return text.replace(/\s+/g, ' ').trim();
}

function children(token: { tokens?: Token[] }): Shape[] {
  return normalize(token.tokens ?? []);
}

function shapeOf(token: Token): Shape | null {
  switch (token.type) {
    case 'space':
      return null;
    case 'heading':
      return { t: 'heading', depth: (token as Tokens.Heading).depth, c: children(token as Tokens.Heading) };
    case 'paragraph':
      return { t: 'paragraph', c: children(token as Tokens.Paragraph) };
    case 'text': {
      const text = token as Tokens.Text;
      // Inside a tight list item marked wraps the content in a text token with children.
      if (text.tokens !== undefined && text.tokens.length > 0) return { t: 'paragraph', c: children(text) };
      return { t: 'text', text: collapse(text.text) };
    }
    case 'escape':
      return { t: 'text', text: collapse((token as Tokens.Escape).text) };
    case 'strong':
    case 'em':
    case 'del':
      return { t: token.type, c: children(token as Tokens.Strong) };
    case 'codespan':
      return { t: 'codespan', text: (token as Tokens.Codespan).text };
    case 'code':
      return { t: 'code', lang: (token as Tokens.Code).lang ?? '', text: (token as Tokens.Code).text };
    case 'link': {
      const link = token as Tokens.Link;
      return { t: 'link', href: link.href, title: link.title ?? '', c: children(link) };
    }
    case 'image': {
      const image = token as Tokens.Image;
      return { t: 'image', href: image.href, title: image.title ?? '', text: image.text };
    }
    case 'list': {
      const list = token as Tokens.List;
      return {
        t: 'list',
        ordered: list.ordered,
        start: list.ordered ? list.start || 1 : 0,
        items: list.items.map((item) => ({ task: item.task, checked: item.checked ?? false, c: normalize(item.tokens) })),
      };
    }
    case 'blockquote':
      return { t: 'blockquote', c: children(token as Tokens.Blockquote) };
    case 'hr':
      return { t: 'hr' };
    case 'br':
      return { t: 'br' };
    case 'html':
      return { t: 'html', text: collapse((token as Tokens.HTML).text) };
    case 'table': {
      const table = token as Tokens.Table;
      return { t: 'table', header: table.header.map((cell) => collapse(cell.text)), rows: table.rows.map((row) => row.map((cell) => collapse(cell.text))) };
    }
    default:
      return { t: token.type, raw: collapse(token.raw) };
  }
}
