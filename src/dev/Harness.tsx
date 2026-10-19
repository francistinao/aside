// Temporary M2 dev harness. It drives the session from the Library window until the
// editor and the control bar arrive in M3. Delete this folder in M3.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import sample from '../../assets/sample-script.md?raw';
import styles from './Harness.module.css';

type Highlight = 'word' | 'line';
type TextSize = 'small' | 'medium' | 'large';
type CaptureStatus = 'hidden' | 'mayBeVisible' | 'visible' | 'unsupported' | 'failed';
type Action = 'pause' | 'resume' | 'restart' | 'stop' | 'prev' | 'next';

interface Settings {
  wpm: number;
  highlight: Highlight;
  opacity: number;
  corner: string;
  textSize: TextSize;
  hideFromCapture: boolean;
}

interface StateEvent {
  state: 'idle' | 'countdown' | 'running' | 'paused' | 'ended';
  count?: number;
}

interface Position {
  word: number | null;
  line: number;
  section: number;
  elapsed: number;
  total: number;
  pause: number | null;
}

interface Estimate {
  words: number;
  seconds: number;
}

interface FollowStats {
  hz: number;
  settled: boolean;
}

const WPM_STEP = 10;
const STATS_INTERVAL_MS = 500;
const ESTIMATE_DEBOUNCE_MS = 300;

function describe(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

export function Harness() {
  const [markdown, setMarkdown] = useState(sample);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [estimate, setEstimate] = useState<Estimate | null>(null);
  const [state, setState] = useState<StateEvent>({ state: 'idle' });
  const [position, setPosition] = useState<Position | null>(null);
  const [eventsPerSecond, setEventsPerSecond] = useState(0);
  const [follow, setFollow] = useState<FollowStats | null>(null);
  const [capture, setCapture] = useState<CaptureStatus | null>(null);
  const [startLatency, setStartLatency] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const startedAt = useRef<number | null>(null);
  const eventCount = useRef(0);

  useEffect(() => {
    let active = true;
    const unlisten: (() => void)[] = [];
    invoke<Settings>('settings_get')
      .then((value) => {
        if (active) setSettings(value);
      })
      .catch((reason: unknown) => setError(describe(reason)));
    void listen<StateEvent>('session://state', (event) => {
      eventCount.current += 1;
      setState(event.payload);
      if (event.payload.state === 'countdown' && event.payload.count === 3 && startedAt.current !== null) {
        setStartLatency(performance.now() - startedAt.current);
        startedAt.current = null;
      }
    }).then((stop) => unlisten.push(stop));
    void listen<Position>('session://position', (event) => {
      eventCount.current += 1;
      setPosition(event.payload);
    }).then((stop) => unlisten.push(stop));
    void listen<CaptureStatus>('capture://status', (event) => setCapture(event.payload)).then((stop) =>
      unlisten.push(stop),
    );
    const counter = setInterval(() => {
      setEventsPerSecond(eventCount.current);
      eventCount.current = 0;
    }, 1000);
    const stats = setInterval(() => {
      invoke<FollowStats>('follow_stats')
        .then(setFollow)
        .catch(() => setFollow(null));
    }, STATS_INTERVAL_MS);
    return () => {
      active = false;
      clearInterval(counter);
      clearInterval(stats);
      for (const stop of unlisten) stop();
    };
  }, []);

  useEffect(() => {
    if (settings === null) return;
    const timer = setTimeout(() => {
      invoke<Estimate>('session_estimate', { markdown, wpm: settings.wpm })
        .then(setEstimate)
        .catch((reason: unknown) => setError(describe(reason)));
    }, ESTIMATE_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [markdown, settings]);

  function run(task: Promise<unknown>) {
    task.then(() => setError(null)).catch((reason: unknown) => setError(describe(reason)));
  }

  function start() {
    startedAt.current = performance.now();
    setStartLatency(null);
    run(invoke('session_start', { markdown }));
  }

  function control(action: Action) {
    run(invoke('session_control', { action }));
  }

  function change(patch: Partial<Settings>) {
    run(invoke<Settings>('settings_set', { patch }).then(setSettings));
  }

  return (
    <section className={styles.harness}>
      <textarea
        className={styles.script}
        value={markdown}
        onChange={(event) => setMarkdown(event.target.value)}
        spellCheck={false}
        aria-label="Script"
      />

      <div className={styles.row}>
        <button type="button" className={styles.primary} onClick={start}>
          Start
        </button>
        <button type="button" className={styles.secondary} onClick={() => control('pause')}>
          Pause
        </button>
        <button type="button" className={styles.secondary} onClick={() => control('resume')}>
          Resume
        </button>
        <button type="button" className={styles.secondary} onClick={() => control('restart')}>
          Restart
        </button>
        <button type="button" className={styles.secondary} onClick={() => control('prev')}>
          Previous
        </button>
        <button type="button" className={styles.secondary} onClick={() => control('next')}>
          Next
        </button>
        <button
          type="button"
          className={styles.secondary}
          disabled={settings === null}
          onClick={() => settings && change({ wpm: settings.wpm + WPM_STEP })}
        >
          Faster
        </button>
        <button
          type="button"
          className={styles.secondary}
          disabled={settings === null}
          onClick={() => settings && change({ wpm: settings.wpm - WPM_STEP })}
        >
          Slower
        </button>
        <button type="button" className={styles.secondary} onClick={() => control('stop')}>
          Stop
        </button>
      </div>

      {settings !== null && (
        <div className={styles.row}>
          <div className={styles.segmented} role="group" aria-label="Highlight">
            <button
              type="button"
              aria-pressed={settings.highlight === 'word'}
              onClick={() => change({ highlight: 'word' })}
            >
              Word
            </button>
            <button
              type="button"
              aria-pressed={settings.highlight === 'line'}
              onClick={() => change({ highlight: 'line' })}
            >
              Line
            </button>
          </div>
          <label className={styles.control}>
            Opacity
            <input
              type="range"
              min={30}
              max={100}
              step={5}
              value={settings.opacity}
              onChange={(event) => change({ opacity: Number(event.target.value) })}
            />
            <span className={styles.value}>{settings.opacity}%</span>
          </label>
          <label className={styles.control}>
            Text size
            <select
              value={settings.textSize}
              onChange={(event) => change({ textSize: event.target.value as TextSize })}
            >
              <option value="small">Small</option>
              <option value="medium">Medium</option>
              <option value="large">Large</option>
            </select>
          </label>
          <span className={styles.value}>{settings.wpm} wpm</span>
        </div>
      )}

      <dl className={styles.readout}>
        <dt>State</dt>
        <dd>
          {state.state}
          {state.count !== undefined ? ` ${state.count}` : ''}
        </dd>
        <dt>Position</dt>
        <dd>
          {position === null
            ? 'none'
            : `word ${position.word ?? 'none'}, line ${position.line}, section ${position.section}, ${position.elapsed.toFixed(1)} of ${position.total.toFixed(1)} s${position.pause !== null ? `, pause ${position.pause} s` : ''}`}
        </dd>
        <dt>Estimate</dt>
        <dd>{estimate === null ? 'none' : `${estimate.words} words, ${estimate.seconds.toFixed(1)} s`}</dd>
        <dt>Events per second</dt>
        <dd>{eventsPerSecond}</dd>
        <dt>Follow</dt>
        <dd>{follow === null ? 'off' : `${follow.hz} Hz${follow.settled ? ', settled' : ''}`}</dd>
        <dt>Capture</dt>
        <dd>{capture ?? 'unknown'}</dd>
        <dt>Start to first digit</dt>
        <dd>{startLatency === null ? 'not measured' : `${startLatency.toFixed(0)} ms`}</dd>
      </dl>

      {error !== null && <p className={styles.error}>{error}</p>}
    </section>
  );
}
