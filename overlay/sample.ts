// Pulls plain reading lines out of a script for the M1 static bubble.
// Headings are skipped and pause markers split a paragraph into two lines.
const PAUSE_MARKER = /\{pause\s+\d+(?:\.\d+)?s\}/;

export function pickLines(markdown: string, count = 3): string[] {
  const lines: string[] = [];
  for (const raw of markdown.split(/\r?\n/)) {
    const text = raw.trim();
    if (text === '' || text.startsWith('#')) continue;
    for (const part of text.split(PAUSE_MARKER)) {
      const line = part.trim();
      if (line !== '') lines.push(line);
    }
    if (lines.length >= count) break;
  }
  return lines.slice(0, count);
}

// Greedy word wrap against a width test, so each visual line fits the bubble.
// A word that fits nowhere gets a line of its own rather than vanishing.
export function breakLines(text: string, fits: (line: string) => boolean): string[] {
  const lines: string[] = [];
  let current = '';
  for (const word of text.split(/\s+/).filter((word) => word !== '')) {
    const candidate = current === '' ? word : `${current} ${word}`;
    if (current === '' || fits(candidate)) {
      current = candidate;
    } else {
      lines.push(current);
      current = word;
    }
  }
  if (current !== '') lines.push(current);
  return lines;
}
