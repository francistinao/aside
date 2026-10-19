import type { ReactNode } from 'react';
import styles from './settings.module.css';

interface RowProps {
  label: string;
  hint?: string;
  children: ReactNode;
}

/// One setting: the label on the left, the control on the right, a hairline below.
export function Row({ label, hint, children }: RowProps) {
  return (
    <div className={styles.row}>
      <div className={styles.rowText}>
        <span className={styles.rowLabel}>{label}</span>
        {hint !== undefined && <span className={styles.rowHint}>{hint}</span>}
      </div>
      <div className={styles.rowControl}>{children}</div>
    </div>
  );
}

interface NoteProps {
  children: ReactNode;
}

/// A short explanation under a group of rows.
export function Note({ children }: NoteProps) {
  return <p className={styles.note}>{children}</p>;
}
