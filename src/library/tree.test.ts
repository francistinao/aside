import { describe, expect, it } from 'vitest';
import { buildTree, moveSelection, parentRowId, sidebarRows } from './tree';
import type { Entry } from './vault';

const entry = (path: string, kind: Entry['kind'] = 'script'): Entry => ({
  path,
  name: path.split('/').pop()?.replace(/\.md$/, '') ?? path,
  kind,
  modified: 0,
});

const entries: Entry[] = [
  entry('Talks', 'folder'),
  entry('Talks/Deep', 'folder'),
  entry('Talks/Deep/Nested.md'),
  entry('Talks/Demo.md'),
  entry('Intro.md'),
];

describe('buildTree', () => {
  it('nests folders and counts scripts including nested ones', () => {
    const tree = buildTree(entries);
    expect(tree.count).toBe(3);
    expect(tree.scripts.map((s) => s.name)).toEqual(['Intro']);
    const talks = tree.folders[0];
    expect(talks?.name).toBe('Talks');
    expect(talks?.count).toBe(2);
    expect(talks?.folders[0]?.path).toBe('Talks/Deep');
    expect(talks?.folders[0]?.depth).toBe(2);
  });
});

describe('sidebarRows', () => {
  it('shows the Library section and the folder tree, collapsed by default', () => {
    const rows = sidebarRows(buildTree(entries), 'all', new Set(), { recents: [], pinned: ['Intro.md'] }, entries);
    expect(rows.map((r) => r.id)).toEqual([
      'header:library',
      'smart:all',
      'smart:recents',
      'smart:pinned',
      'header:folders',
      'folder:Talks',
      'script:Intro.md',
    ]);
    const pinned = rows[3];
    expect(pinned?.kind === 'smart' && pinned.count).toBe(1);
  });

  it('expands folders in place with increasing depth', () => {
    const rows = sidebarRows(buildTree(entries), 'all', new Set(['Talks', 'Talks/Deep']), { recents: [], pinned: [] }, entries);
    expect(rows.slice(5).map((r) => `${r.id}@${'depth' in r ? r.depth : '-'}`)).toEqual([
      'folder:Talks@0',
      'folder:Talks/Deep@1',
      'script:Talks/Deep/Nested.md@2',
      'script:Talks/Demo.md@1',
      'script:Intro.md@0',
    ]);
  });

  it('lists recents in order and drops unknown paths', () => {
    const rows = sidebarRows(buildTree(entries), 'recents', new Set(), { recents: ['Talks/Demo.md', 'missing.md', 'Intro.md'], pinned: [] }, entries);
    expect(rows.slice(4).map((r) => r.id)).toEqual(['header:recents', 'script:Talks/Demo.md', 'script:Intro.md']);
  });
});

describe('keyboard moves', () => {
  const rows = sidebarRows(buildTree(entries), 'all', new Set(['Talks']), { recents: [], pinned: [] }, entries);

  it('skips headers and stops at the ends', () => {
    expect(moveSelection(rows, null, 1)).toBe('smart:all');
    expect(moveSelection(rows, 'smart:pinned', 1)).toBe('folder:Talks');
    expect(moveSelection(rows, 'folder:Talks', -1)).toBe('smart:pinned');
    expect(moveSelection(rows, 'smart:all', -1)).toBe('smart:all');
    expect(moveSelection(rows, 'script:Intro.md', 1)).toBe('script:Intro.md');
  });

  it('finds the folder that holds a nested row', () => {
    expect(parentRowId(rows, 'script:Talks/Demo.md')).toBe('folder:Talks');
    expect(parentRowId(rows, 'folder:Talks/Deep')).toBe('folder:Talks');
    expect(parentRowId(rows, 'script:Intro.md')).toBeNull();
    expect(parentRowId(rows, 'smart:all')).toBeNull();
  });
});
