import { describe, expect, it } from 'vitest';
import { createVersionTracker } from './fileVersion';

describe('createVersionTracker', () => {
  it('never treats the editor\'s own save as an outside change', () => {
    const tracker = createVersionTracker(1);
    tracker.markSaved(2);
    expect(tracker.onFileChanged(2, false)).toBe('none');
    expect(tracker.onFileChanged(2, true)).toBe('none');
  });

  it('reloads an outside change when nothing is pending, once', () => {
    const tracker = createVersionTracker(1);
    expect(tracker.onFileChanged(3, false)).toBe('reload');
    expect(tracker.onFileChanged(3, false)).toBe('none');
  });

  it('asks when the editor holds unsaved edits', () => {
    const tracker = createVersionTracker(1);
    expect(tracker.onFileChanged(3, true)).toBe('ask');
  });

  it('does nothing for the time it opened with', () => {
    const tracker = createVersionTracker(1);
    expect(tracker.onFileChanged(1, true)).toBe('none');
  });
});
