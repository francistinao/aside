import { Minus, Plus } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { MAX_PAUSE, MIN_PAUSE, PAUSE_STEP, formatPause } from './pause';
import styles from './editor.module.css';

interface Props {
  seconds: number;
  /// Position inside the editor container, in CSS pixels.
  left: number;
  top: number;
  onChange: (seconds: number) => void;
  onClose: () => void;
}

/// A stepper for one pause chip. Escape or a click elsewhere closes it.
export function PausePopover({ seconds, left, top, onChange, onClose }: Props) {
  const root = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose();
    }
    function onPointer(event: PointerEvent) {
      if (root.current !== null && !root.current.contains(event.target as Node)) onClose();
    }
    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onPointer, true);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onPointer, true);
    };
  }, [onClose]);

  const step = (delta: number) => onChange(Math.min(MAX_PAUSE, Math.max(MIN_PAUSE, seconds + delta)));

  return (
    <div ref={root} className={styles.popover} style={{ left, top }} role="dialog" aria-label="Pause length">
      <button type="button" className={styles.stepButton} aria-label="Shorter" disabled={seconds <= MIN_PAUSE} onClick={() => step(-PAUSE_STEP)}>
        <Minus size={16} strokeWidth={1.5} aria-hidden="true" />
      </button>
      <span className={styles.stepValue}>{formatPause(seconds)} s</span>
      <button type="button" className={styles.stepButton} aria-label="Longer" disabled={seconds >= MAX_PAUSE} onClick={() => step(PAUSE_STEP)}>
        <Plus size={16} strokeWidth={1.5} aria-hidden="true" />
      </button>
    </div>
  );
}
