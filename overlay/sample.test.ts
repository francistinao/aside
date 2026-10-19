import { describe, expect, it } from 'vitest';
import sample from '../assets/sample-script.md?raw';
import { pickLines } from './sample';

describe('pickLines', () => {
  it('takes the first three reading lines from the sample script', () => {
    expect(pickLines(sample)).toEqual([
      'This script is a short tour. Press Start in the bar below and read along.',
      'The script travels with your pointer, so you can keep your eyes on what you are clicking.',
      'Move your mouse around now. Notice how the bubble drifts behind it and settles.',
    ]);
  });

  it('skips headings and blank lines', () => {
    expect(pickLines('# Title\n\n## Part\n\nOne.\n\nTwo.')).toEqual(['One.', 'Two.']);
  });

  it('splits at pause markers and drops them', () => {
    expect(pickLines('Open it. {pause 2s} Now click. {pause 0.5s} Done.')).toEqual(['Open it.', 'Now click.', 'Done.']);
  });

  it('returns fewer lines when the script is short', () => {
    expect(pickLines('Only one line.')).toEqual(['Only one line.']);
  });
});
