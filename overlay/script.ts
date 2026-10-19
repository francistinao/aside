// The script DOM: one span per word, built once per session from the tokens Rust sends.

export type Token =
  | { kind: 'word'; text: string; bold: boolean; italic: boolean }
  | { kind: 'paragraphEnd' }
  | { kind: 'heading'; text: string; level: number }
  | { kind: 'pause'; seconds: number };

export interface ScriptPayload {
  title: string;
  tokens: Token[];
  words: number;
  total: number;
}

export interface BuiltScript {
  /// Index is the word index Rust uses.
  words: HTMLSpanElement[];
  paragraphOf: number[];
  /// Section labels in order; section k shows headings[k - 1].
  headings: string[];
}

export function buildScript(payload: ScriptPayload): BuiltScript {
  const words: HTMLSpanElement[] = [];
  const paragraphOf: number[] = [];
  const headings: string[] = [];
  let paragraph = 0;
  let paragraphOpen = false;

  for (const token of payload.tokens) {
    switch (token.kind) {
      case 'word': {
        const span = document.createElement('span');
        span.className = 'word';
        if (token.bold) span.classList.add('bold');
        if (token.italic) span.classList.add('italic');
        span.textContent = token.text;
        words.push(span);
        paragraphOf.push(paragraph);
        paragraphOpen = true;
        break;
      }
      case 'paragraphEnd':
      case 'heading':
        if (token.kind === 'heading') headings.push(token.text);
        if (paragraphOpen) {
          paragraph += 1;
          paragraphOpen = false;
        }
        break;
      case 'pause':
        break;
    }
  }
  return { words, paragraphOf, headings };
}

/// Lays the words out as paragraphs so they wrap naturally, for measuring.
export function fillParagraphs(container: HTMLElement, script: BuiltScript): void {
  const paragraphs: HTMLParagraphElement[] = [];
  script.words.forEach((word, index) => {
    const paragraph = script.paragraphOf[index] ?? 0;
    let block = paragraphs[paragraph];
    if (block === undefined) {
      block = document.createElement('p');
      block.className = 'paragraph';
      paragraphs[paragraph] = block;
    } else {
      block.append(' ');
    }
    block.append(word);
  });
  container.replaceChildren(...paragraphs.filter((p): p is HTMLParagraphElement => p !== undefined));
}

/// Regroups the words into one block per visual line.
export function fillLines(container: HTMLElement, script: BuiltScript, ranges: [number, number][]): HTMLElement[] {
  const lines = ranges.map(([start, end]) => {
    const line = document.createElement('div');
    line.className = 'line';
    for (let i = start; i <= end; i += 1) {
      const word = script.words[i];
      if (word === undefined) continue;
      if (i > start) line.append(' ');
      line.append(word);
    }
    return line;
  });
  container.replaceChildren(...lines);
  return lines;
}
