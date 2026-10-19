import { Play, Square } from 'lucide-react';
import { isMac } from '../platform';
import { CornerPicker } from './CornerPicker';
import type { Corner } from './CornerPicker';
import { Segmented } from './Segmented';
import { Slider } from './Slider';
import { Switch } from './Switch';
import styles from './controls.module.css';

export type Highlight = 'word' | 'line';
export type CaptureStatus = 'hidden' | 'mayBeVisible' | 'visible' | 'unsupported' | 'failed';

export interface PrompterSettings {
  highlight: Highlight;
  wpm: number;
  opacity: number;
  corner: Corner;
  hideFromCapture: boolean;
}

interface Props {
  settings: PrompterSettings;
  capture: CaptureStatus;
  running: boolean;
  onChange: (patch: Partial<PrompterSettings>) => void;
  onStart: () => void;
  onStop: () => void;
}

const CAPTURE_LABEL: Record<CaptureStatus, string> = {
  hidden: 'Hidden',
  mayBeVisible: 'May be visible',
  visible: 'Visible',
  unsupported: 'Visible',
  failed: 'Visible',
};

const ICON = { size: 14, strokeWidth: 1.5 } as const;

/// The floating bar under the editor: one row, a label above each control.
export function ControlBar({ settings, capture, running, onChange, onStart, onStop }: Props) {
  const shortcut = isMac ? '⌘↩' : 'Ctrl+Enter';
  return (
    <div className={styles.bar} role="toolbar" aria-label="Prompter">
      <div className={styles.group}>
        <span className={styles.label}>Highlight</span>
        <div className={styles.control}>
          <Segmented
            value={settings.highlight}
            label="Highlight"
            options={[
              { value: 'word', label: 'Word' },
              { value: 'line', label: 'Line' },
            ]}
            onChange={(highlight) => onChange({ highlight })}
          />
        </div>
      </div>
      <span className={styles.divider} />
      <div className={`${styles.group} ${styles.flexible}`}>
        <span className={styles.label}>Speed</span>
        <div className={styles.control}>
          <Slider value={settings.wpm} min={80} max={220} step={5} label="Speed" onChange={(wpm) => onChange({ wpm })} />
          <span className={styles.value}>{settings.wpm} wpm</span>
        </div>
      </div>
      <span className={styles.divider} />
      <div className={`${styles.group} ${styles.flexible}`}>
        <span className={styles.label}>Opacity</span>
        <div className={styles.control}>
          <Slider value={settings.opacity} min={30} max={100} step={5} label="Opacity" onChange={(opacity) => onChange({ opacity })} />
          <span className={styles.value}>{settings.opacity}%</span>
        </div>
      </div>
      <span className={styles.divider} />
      <div className={styles.group}>
        <span className={styles.label}>Position</span>
        <div className={styles.control}>
          <CornerPicker value={settings.corner} onChange={(corner) => onChange({ corner })} />
        </div>
      </div>
      <span className={styles.divider} />
      <div className={styles.group}>
        <span className={styles.label}>Hide from screen sharing</span>
        <div className={styles.control}>
          <Switch
            checked={settings.hideFromCapture}
            label="Hide from screen sharing"
            title={CAPTURE_LABEL[capture]}
            onChange={(hideFromCapture) => onChange({ hideFromCapture })}
          />
        </div>
      </div>
      <span className={styles.spacer} />
      {running ? (
        <button type="button" className={`${styles.start} ${styles.stop}`} title="Stop" onClick={onStop}>
          <Square {...ICON} aria-hidden="true" />
          Stop
        </button>
      ) : (
        <button type="button" className={styles.start} title={`Start ${shortcut}`} onClick={onStart}>
          <Play {...ICON} aria-hidden="true" />
          Start
        </button>
      )}
    </div>
  );
}
