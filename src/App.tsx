import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import styles from './App.module.css';

interface Settings {
  sidebarWidth: number;
  sidebarVisible: boolean;
}

export function App() {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    let active = true;
    invoke<Settings>('settings_get')
      .then((value) => {
        if (active) setSettings(value);
      })
      .catch(() => {
        if (active) setSettings({ sidebarWidth: 240, sidebarVisible: true });
      });
    return () => {
      active = false;
    };
  }, []);

  const width = settings?.sidebarVisible === false ? 0 : (settings?.sidebarWidth ?? 240);

  return (
    <div className={styles.shell} style={{ '--sidebar-width': `${width}px` } as React.CSSProperties}>
      <aside className={styles.sidebar} aria-label="Library">
        <div className={styles.sidebarTop} data-tauri-drag-region />
      </aside>
      <main className={styles.main}>
        <header className={styles.toolbar} data-tauri-drag-region />
        <section className={styles.content} />
      </main>
    </div>
  );
}
