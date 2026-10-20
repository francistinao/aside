// Which file time the editor has already accounted for, so its own saves never read as
// edits made elsewhere. The Library reports the file's time whenever it reads the file or
// the editor saves it; only a time the editor has not seen means someone else wrote it.

export type FileChange = 'none' | 'reload' | 'ask';

export interface VersionTracker {
  /// The editor's own write landed with this time.
  markSaved(modified: number): void;
  /// The Library reports the file's time. Pending means the editor holds unsaved edits.
  onFileChanged(modified: number, pending: boolean): FileChange;
}

export function createVersionTracker(initial: number): VersionTracker {
  let seen = initial;
  return {
    markSaved(modified) {
      seen = modified;
    },
    onFileChanged(modified, pending) {
      if (modified === seen) return 'none';
      seen = modified;
      return pending ? 'ask' : 'reload';
    },
  };
}
