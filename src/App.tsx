import { Suspense, lazy } from 'react';
import styles from './App.module.css';

// The Library gets its editor and control bar in M3. Until then dev builds show a harness.
const Harness = import.meta.env.DEV ? lazy(() => import('./dev/Harness').then((m) => ({ default: m.Harness }))) : null;

export function App() {
  return (
    <main className={styles.app}>
      {Harness !== null && (
        <Suspense>
          <Harness />
        </Suspense>
      )}
    </main>
  );
}
