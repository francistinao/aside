// Saves 500 ms after the last change. Flush writes at once when the user leaves:
// another script, window blur, close or quit. Nothing is written without a change.

export interface Autosave {
  /// The producer runs when the save happens, so a keystroke never serializes the document.
  schedule(produce: () => string): void;
  flush(): Promise<void>;
  cancel(): void;
  pending(): boolean;
}

export const AUTOSAVE_DELAY_MS = 500;

export function createAutosave(
  save: (content: string) => Promise<void>,
  onError: (reason: unknown) => void,
  delay = AUTOSAVE_DELAY_MS,
): Autosave {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: (() => string) | null = null;
  let inflight: Promise<void> = Promise.resolve();

  function run(): Promise<void> {
    if (pending === null) return inflight;
    const produce = pending;
    pending = null;
    inflight = inflight.then(() => save(produce())).catch(onError);
    return inflight;
  }

  function clearTimer() {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  }

  return {
    schedule(produce) {
      pending = produce;
      clearTimer();
      timer = setTimeout(() => {
        timer = null;
        void run();
      }, delay);
    },
    flush() {
      clearTimer();
      return run();
    },
    cancel() {
      clearTimer();
      pending = null;
    },
    pending: () => pending !== null,
  };
}
