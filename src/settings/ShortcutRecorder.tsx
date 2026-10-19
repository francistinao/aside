import { useState } from 'react';
import type { KeyboardEvent } from 'react';
import { isMac } from '../platform';
import { acceleratorFromPress, formatShortcut } from './shortcuts';
import styles from './settings.module.css';

interface Props {
  value: string;
  label: string;
  onChange: (accelerator: string) => void;
}

/// Click, press the keys, done. Escape or leaving the field keeps the old shortcut.
export function ShortcutRecorder({ value, label, onChange }: Props) {
  const [recording, setRecording] = useState(false);

  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (!recording) return;
    event.preventDefault();
    if (event.key === 'Escape') {
      setRecording(false);
      return;
    }
    const accelerator = acceleratorFromPress(event);
    if (accelerator === null) return;
    setRecording(false);
    if (accelerator !== value) onChange(accelerator);
  }

  return (
    <button
      type="button"
      className={styles.recorder}
      data-recording={recording ? 'true' : undefined}
      aria-label={`${label} shortcut`}
      onClick={() => setRecording(true)}
      onKeyDown={onKeyDown}
      onBlur={() => setRecording(false)}
    >
      {recording ? 'Press keys' : formatShortcut(value, isMac)}
    </button>
  );
}
