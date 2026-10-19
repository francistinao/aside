import { Switch } from '../controls/Switch';
import { api } from './api';
import type { Settings, SettingsPatch } from './api';
import { Row } from './rows';
import styles from './settings.module.css';

interface Props {
  settings: Settings;
  onPatch: (patch: SettingsPatch) => void;
  onError: (reason: unknown) => void;
}

export function GeneralTab({ settings, onPatch, onError }: Props) {
  return (
    <>
      <Row label="Scripts folder" hint={settings.scriptsFolder ?? 'Not chosen'}>
        <button type="button" className={styles.secondary} onClick={() => api.chooseFolder().catch(onError)}>
          Change…
        </button>
      </Row>
      <Row label="Open at login">
        <Switch checked={settings.openAtLogin} label="Open at login" onChange={(openAtLogin) => onPatch({ openAtLogin })} />
      </Row>
    </>
  );
}
