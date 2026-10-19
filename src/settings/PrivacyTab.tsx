import { Switch } from '../controls/Switch';
import { isMac } from '../platform';
import type { CaptureStatus, Settings, SettingsPatch } from './api';
import { Note, Row } from './rows';
import styles from './settings.module.css';

interface Props {
  settings: Settings;
  capture: CaptureStatus;
  onPatch: (patch: SettingsPatch) => void;
}

// Copy from docs/INVISIBILITY.md.
const STATUS_LABEL: Record<CaptureStatus, string> = {
  hidden: 'Hidden',
  mayBeVisible: 'May be visible',
  visible: 'Visible',
  unsupported: 'Visible',
  failed: 'Visible',
};

const STATUS_DOT: Partial<Record<CaptureStatus, string>> = {
  hidden: 'success',
  mayBeVisible: 'warning',
};

const STATUS_NOTE: Partial<Record<CaptureStatus, string>> = {
  unsupported: "This version of Windows can't hide the Prompter from capture.",
  failed: "Couldn't hide the Prompter from capture.",
};

export function PrivacyTab({ settings, capture, onPatch }: Props) {
  return (
    <>
      <Row label="Hide from screen sharing">
        <Switch checked={settings.hideFromCapture} label="Hide from screen sharing" onChange={(hideFromCapture) => onPatch({ hideFromCapture })} />
      </Row>
      <Row label="Capture status for this computer" hint={STATUS_NOTE[capture]}>
        <span className={styles.status}>
          <span className={styles.dot} data-tone={STATUS_DOT[capture] ?? ''} />
          {STATUS_LABEL[capture]}
        </span>
      </Row>
      {isMac && <Note>macOS may include the Prompter when you share your entire screen. Share a single window or tab to keep it out.</Note>}
    </>
  );
}
