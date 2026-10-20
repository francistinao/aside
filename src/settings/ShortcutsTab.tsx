import { useState } from 'react';
import { api, describeError } from './api';
import type { Settings, ShortcutAction, ShortcutStatus } from './api';
import { ShortcutRecorder } from './ShortcutRecorder';
import { ACTIONS, NOT_ALLOWED, isAllowed } from './shortcuts';
import styles from './settings.module.css';

interface Props {
  settings: Settings;
  status: ShortcutStatus;
  onError: (reason: unknown) => void;
}

export function ShortcutsTab({ settings, status, onError }: Props) {
  // What this window refused or the OS rejected on a rebind, shown under that row.
  const [refused, setRefused] = useState<Partial<Record<ShortcutAction, string>>>({});
  const set = (action: ShortcutAction, accelerator: string) => {
    if (!isAllowed(accelerator)) {
      setRefused((current) => ({ ...current, [action]: NOT_ALLOWED }));
      return;
    }
    api
      .setShortcut(action, accelerator)
      .then(() => setRefused((current) => ({ ...current, [action]: undefined })))
      .catch((reason: unknown) => setRefused((current) => ({ ...current, [action]: describeError(reason) })));
  };
  const reset = (action: ShortcutAction) => {
    setRefused((current) => ({ ...current, [action]: undefined }));
    api.resetShortcut(action).catch(onError);
  };
  return (
    <>
      {ACTIONS.map(({ action, label }) => {
        const failure = refused[action] ?? status.failed.find((entry) => entry.action === action)?.message;
        return (
          <div key={action} className={styles.shortcutRow}>
            <div className={styles.row}>
              <div className={styles.rowText}>
                <span className={styles.rowLabel}>{label}</span>
                {failure !== undefined && <span className={styles.failure}>{failure}</span>}
              </div>
              <div className={styles.rowControl}>
                <ShortcutRecorder value={settings.shortcuts[action]} label={label} onChange={(value) => void set(action, value)} />
                <button type="button" className={styles.plain} onClick={() => void reset(action)}>
                  Reset
                </button>
              </div>
            </div>
          </div>
        );
      })}
    </>
  );
}
