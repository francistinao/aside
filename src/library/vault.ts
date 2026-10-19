// Typed wrappers around the vault commands. Paths are relative to the scripts folder.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type EntryKind = 'folder' | 'script';

export interface Entry {
  path: string;
  name: string;
  kind: EntryKind;
  modified: number;
}

export interface Tree {
  root: string;
  entries: Entry[];
  pins: string[];
  recents: string[];
}

export interface VaultStatus {
  folder: string | null;
  missing: boolean;
}

export interface ChosenFolder {
  folder: string;
  entries: Entry[];
  open: string | null;
}

export interface FileData {
  content: string;
  modified: number;
}

export interface Hit {
  path: string;
  name: string;
  snippet: string;
}

export const vault = {
  status: () => invoke<VaultStatus>('vault_status'),
  chooseFolder: () => invoke<ChosenFolder | null>('vault_choose_folder'),
  tree: () => invoke<Tree>('vault_tree'),
  read: (path: string) => invoke<FileData>('vault_read', { path }),
  write: (path: string, content: string) => invoke<FileData>('vault_write', { path, content }),
  create: (dir: string) => invoke<string>('vault_create', { dir }),
  createFolder: (dir: string) => invoke<string>('vault_create_folder', { dir }),
  rename: (path: string, newName: string) => invoke<string>('vault_rename', { path, newName }),
  trash: async (path: string) => {
    await invoke('vault_trash', { path });
  },
  search: (query: string) => invoke<Hit[]>('vault_search', { query }),
  reveal: async (path: string) => {
    await invoke('vault_reveal', { path });
  },
  pin: (path: string, pinned: boolean) => invoke<string[]>('library_pin', { path, pinned }),
  contextMenu: async (path: string, kind: EntryKind, pinned: boolean, x: number, y: number) => {
    await invoke('library_context_menu', { path, target: { kind, pinned }, x, y });
  },
};

/// Fires with the changed relative paths after the folder changed on disk.
export function onVaultChanged(handler: (paths: string[]) => void): Promise<() => void> {
  return listen<{ paths: string[] }>('vault://changed', (event) => handler(event.payload.paths));
}

export function parentOf(path: string): string {
  const slash = path.lastIndexOf('/');
  return slash < 0 ? '' : path.slice(0, slash);
}

export function describeError(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}
