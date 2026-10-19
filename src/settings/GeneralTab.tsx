import { api } from './api';
import type { Settings } from './api';
import { Row } from './rows';
import styles from './settings.module.css';

interface Props {
  settings: Settings;
  onError: (reason: unknown) => void;
}

export function GeneralTab({ settings, onError }: Props) {
  return (
    <>
      <Row label="Scripts folder" hint={settings.scriptsFolder ?? 'Not chosen'}>
        <button type="button" className={styles.secondary} onClick={() => api.chooseFolder().catch(onError)}>
          Change…
        </button>
      </Row>
    </>
  );
}
