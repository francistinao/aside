// Visual line breaks. Rust gets them once per layout through session_set_lines.

export interface WordRect {
  /// Which paragraph the word belongs to. Lines never cross paragraphs.
  paragraph: number;
  /// Top edge of the word's first client rect, in CSS pixels.
  top: number;
}

/// Groups consecutive words that share a paragraph and a top edge into inclusive ranges.
/// A word wider than the line still has one top, so it forms a line of its own.
export function linesFromRects(rects: WordRect[], tolerance = 1): [number, number][] {
  const lines: [number, number][] = [];
  let start = 0;
  for (let i = 1; i <= rects.length; i += 1) {
    const previous = rects[i - 1];
    const current = rects[i];
    const breaks =
      current === undefined ||
      previous === undefined ||
      current.paragraph !== previous.paragraph ||
      Math.abs(current.top - previous.top) > tolerance;
    if (breaks) {
      if (previous !== undefined) lines.push([start, i - 1]);
      start = i;
    }
  }
  return lines;
}

/// Reads each word's first client rect through a Range at the real width and font.
export function measure(words: HTMLElement[], paragraphOf: number[]): WordRect[] {
  const range = document.createRange();
  return words.map((word, index) => {
    range.selectNodeContents(word);
    const rect = range.getClientRects()[0];
    return { paragraph: paragraphOf[index] ?? 0, top: rect?.top ?? 0 };
  });
}
