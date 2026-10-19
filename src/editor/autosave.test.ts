import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createAutosave } from './autosave';

describe('createAutosave', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('saves once, 500 ms after the last change', async () => {
    const save = vi.fn(() => Promise.resolve());
    const autosave = createAutosave(save, () => undefined);
    autosave.schedule('a');
    await vi.advanceTimersByTimeAsync(300);
    autosave.schedule('ab');
    await vi.advanceTimersByTimeAsync(499);
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith('ab');
  });

  it('flush writes the pending change at once and nothing when there is none', async () => {
    const save = vi.fn(() => Promise.resolve());
    const autosave = createAutosave(save, () => undefined);
    await autosave.flush();
    expect(save).not.toHaveBeenCalled();
    autosave.schedule('x');
    await autosave.flush();
    expect(save).toHaveBeenCalledWith('x');
    await vi.advanceTimersByTimeAsync(1000);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it('cancel drops the pending change', async () => {
    const save = vi.fn(() => Promise.resolve());
    const autosave = createAutosave(save, () => undefined);
    autosave.schedule('gone');
    autosave.cancel();
    await vi.advanceTimersByTimeAsync(1000);
    expect(save).not.toHaveBeenCalled();
    expect(autosave.pending()).toBe(false);
  });

  it('reports save failures and keeps working', async () => {
    const errors: unknown[] = [];
    const save = vi.fn((content: string) => (content === 'bad' ? Promise.reject(new Error('disk')) : Promise.resolve()));
    const autosave = createAutosave(save, (reason) => errors.push(reason));
    autosave.schedule('bad');
    await autosave.flush();
    expect(errors).toHaveLength(1);
    autosave.schedule('good');
    await autosave.flush();
    expect(save).toHaveBeenLastCalledWith('good');
  });
});
