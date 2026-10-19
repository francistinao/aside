import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import styles from './App.module.css';

interface FollowStats {
  hz: number;
  settled: boolean;
}

const STATS_INTERVAL_MS = 500;

export function App() {
  const [shown, setShown] = useState(false);
  const [stats, setStats] = useState<FollowStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function toggle() {
    try {
      await invoke('prompter_set_visible', { visible: !shown });
      setShown(!shown);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  useEffect(() => {
    if (!import.meta.env.DEV || !shown) return;
    const timer = setInterval(() => {
      invoke<FollowStats>('follow_stats')
        .then(setStats)
        .catch(() => setStats(null));
    }, STATS_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [shown]);

  return (
    <main className={styles.app}>
      <button type="button" className={styles.primary} onClick={() => void toggle()}>
        {shown ? 'Hide Bubble' : 'Show Bubble'}
      </button>
      {import.meta.env.DEV && shown && stats !== null && (
        <p className={styles.debug}>
          Follow: {stats.hz} Hz{stats.settled ? ', settled' : ''}
        </p>
      )}
      {error !== null && <p className={styles.error}>{error}</p>}
    </main>
  );
}
