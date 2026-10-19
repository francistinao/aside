// The scripts folder as React state: first run, the tree, the open script, live refreshes.

import { useCallback, useEffect, useRef, useState } from 'react';
import { describeError, onVaultChanged, vault } from './vault';
import type { Tree } from './vault';

export type Phase = 'loading' | 'noFolder' | 'missing' | 'ready';

export interface VaultState {
  phase: Phase;
  tree: Tree | null;
  error: string | null;
}

export function useVault() {
  const [state, setState] = useState<VaultState>({ phase: 'loading', tree: null, error: null });
  const [openPath, setOpenPath] = useState<string | null>(null);
  const started = useRef(false);

  const refresh = useCallback(async () => {
    try {
      const tree = await vault.tree();
      setState({ phase: 'ready', tree, error: null });
    } catch (reason: unknown) {
      const status = await vault.status().catch(() => ({ folder: null, missing: false }));
      setState({ phase: status.folder === null ? 'noFolder' : 'missing', tree: null, error: describeError(reason) });
    }
  }, []);

  const chooseFolder = useCallback(async () => {
    try {
      const chosen = await vault.chooseFolder();
      if (chosen === null) {
        setState((current) => (current.phase === 'ready' ? current : { ...current, phase: 'noFolder' }));
        return;
      }
      await refresh();
      if (chosen.open !== null) setOpenPath(chosen.open);
    } catch (reason: unknown) {
      setState((current) => ({ ...current, error: describeError(reason) }));
    }
  }, [refresh]);

  useEffect(() => {
    // StrictMode runs effects twice; the folder picker must open once.
    if (started.current) return;
    started.current = true;
    vault
      .status()
      .then((status) => (status.folder === null ? chooseFolder() : refresh()))
      .catch((reason: unknown) => setState({ phase: 'noFolder', tree: null, error: describeError(reason) }));
  }, [chooseFolder, refresh]);

  useEffect(() => {
    let active = true;
    let stop: (() => void) | null = null;
    void onVaultChanged(() => void refresh()).then((unlisten) => {
      if (active) stop = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      stop?.();
    };
  }, [refresh]);

  const createScript = useCallback(
    async (dir: string) => {
      const path = await vault.create(dir);
      await refresh();
      setOpenPath(path);
      return path;
    },
    [refresh],
  );

  const createFolder = useCallback(
    async (dir: string) => {
      const path = await vault.createFolder(dir);
      await refresh();
      return path;
    },
    [refresh],
  );

  return { ...state, openPath, open: setOpenPath, refresh, chooseFolder, createScript, createFolder };
}
