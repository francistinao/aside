import { describe, expect, it } from 'vitest';
import { linesFromRects } from './lines';

const at = (paragraph: number, top: number) => ({ paragraph, top });

describe('linesFromRects', () => {
  it('puts words with the same top on one line', () => {
    expect(linesFromRects([at(0, 0), at(0, 0), at(0, 0)])).toEqual([[0, 2]]);
  });

  it('starts a new line when the top moves down', () => {
    expect(linesFromRects([at(0, 0), at(0, 0), at(0, 26), at(0, 26), at(0, 52)])).toEqual([
      [0, 1],
      [2, 3],
      [4, 4],
    ]);
  });

  it('never joins words from different paragraphs', () => {
    expect(linesFromRects([at(0, 0), at(1, 0), at(1, 0)])).toEqual([
      [0, 0],
      [1, 2],
    ]);
  });

  it('gives a word wider than the bubble its own line', () => {
    expect(linesFromRects([at(0, 0), at(0, 26), at(0, 52), at(0, 52)])).toEqual([
      [0, 0],
      [1, 1],
      [2, 3],
    ]);
  });

  it('ignores sub pixel jitter inside the tolerance', () => {
    expect(linesFromRects([at(0, 0), at(0, 0.4), at(0, 0.9)])).toEqual([[0, 2]]);
  });

  it('returns nothing for an empty script', () => {
    expect(linesFromRects([])).toEqual([]);
  });
});
