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
