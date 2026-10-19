import { useCallback, useEffect, useState } from 'react';
import { Segmented } from '../controls/Segmented';
import { api, describeError } from './api';
import type { CaptureStatus, Settings, SettingsPatch, ShortcutStatus } from './api';
import { GeneralTab } from './GeneralTab';
import { PrivacyTab } from './PrivacyTab';
import { ShortcutsTab } from './ShortcutsTab';
import { PrompterTab } from './PrompterTab';
import styles from './settings.module.css';

type Tab = 'general' | 'prompter' | 'shortcuts' | 'privacy';

const TABS: { value: Tab; label: string }[] = [
  { value: 'general', label: 'General' },
  { value: 'prompter', label: 'Prompter' },
  { value: 'shortcuts', label: 'Shortcuts' },
  { value: 'privacy', label: 'Privacy' },
];

/// Four tabs over the shared settings. Every control writes at once; the window shows
/// whatever Rust reports back, so a change made anywhere else appears here too.
export function SettingsApp() {
  const [tab, setTab] = useState<Tab>('general');
  const [settings, setSettings] = useState<Settings | null>(null);
  const [capture, setCapture] = useState<CaptureStatus>('visible');
  const [shortcuts, setShortcuts] = useState<ShortcutStatus>({ failed: [] });
  const [error, setError] = useState<string | null>(null);
  const report = useCallback((reason: unknown) => setError(describeError(reason)), []);

  useEffect(() => {
    let active = true;
    const stops: (() => void)[] = [];
    const keep = (stop: () => void) => {
      if (active) stops.push(stop);
      else stop();
    };
    api.settings().then(setSettings).catch(report);
    api.captureStatus().then(setCapture).catch(report);
    api.shortcutStatus().then(setShortcuts).catch(report);
    void api.onSettings(setSettings).then(keep);
    void api.onCapture(setCapture).then(keep);
    void api.onShortcuts(setShortcuts).then(keep);
    return () => {
      active = false;
      for (const stop of stops) stop();
    };
  }, [report]);

  const patch = useCallback(
    (change: SettingsPatch) => {
      setSettings((current) => (current === null ? current : { ...current, ...change }));
      api.patch(change).then(setSettings).catch(report);
    },
    [report],
  );

  return (
    <div className={styles.app}>
      <div className={styles.tabs}>
        <Segmented value={tab} label="Settings tabs" options={TABS} onChange={setTab} />
      </div>
      <div className={styles.content}>
        {settings !== null && tab === 'general' && <GeneralTab settings={settings} onPatch={patch} onError={report} />}
        {settings !== null && tab === 'prompter' && <PrompterTab settings={settings} onPatch={patch} />}
        {settings !== null && tab === 'shortcuts' && <ShortcutsTab settings={settings} status={shortcuts} onError={report} />}
        {settings !== null && tab === 'privacy' && <PrivacyTab settings={settings} capture={capture} onPatch={patch} />}
        {error !== null && <p className={styles.error}>{error}</p>}
      </div>
    </div>
  );
}
