import { api } from './api';
import type { Settings, ShortcutAction, ShortcutStatus } from './api';
import { ShortcutRecorder } from './ShortcutRecorder';
import { ACTIONS } from './shortcuts';
import styles from './settings.module.css';

interface Props {
  settings: Settings;
  status: ShortcutStatus;
  onError: (reason: unknown) => void;
}

export function ShortcutsTab({ settings, status, onError }: Props) {
  const set = (action: ShortcutAction, accelerator: string) => api.setShortcut(action, accelerator).catch(onError);
  const reset = (action: ShortcutAction) => api.resetShortcut(action).catch(onError);
  return (
    <>
      {ACTIONS.map(({ action, label }) => {
        const failure = status.failed.find((entry) => entry.action === action);
        return (
          <div key={action} className={styles.shortcutRow}>
            <div className={styles.row}>
              <div className={styles.rowText}>
                <span className={styles.rowLabel}>{label}</span>
                {failure !== undefined && <span className={styles.failure}>{failure.message}</span>}
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
