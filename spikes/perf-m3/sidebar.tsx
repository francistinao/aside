// Times the sidebar with 1,000 scripts in 20 folders, all expanded: the first render,
// a view switch and the pure row building.
import { StrictMode, createRef } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { Sidebar } from '../../src/library/Sidebar';
import { buildTree, sidebarRows } from '../../src/library/tree';
import type { SmartId } from '../../src/library/tree';
import type { Entry, Tree } from '../../src/library/vault';
import '../../src/styles/tokens.css';
import '../../src/styles/library.css';

function corpus(): Tree {
  const entries: Entry[] = [];
  for (let folder = 0; folder < 20; folder += 1) {
    entries.push({ path: `Folder ${folder}`, name: `Folder ${folder}`, kind: 'folder', modified: 0 });
    for (let i = 0; i < 50; i += 1) {
      const n = folder * 50 + i;
      entries.push({ path: `Folder ${folder}/Script ${n}.md`, name: `Script ${n}`, kind: 'script', modified: n });
    }
  }
  return { root: '/tmp/corpus', entries, pins: ['Folder 0/Script 1.md'], recents: ['Folder 1/Script 60.md'] };
}

function nextFrame(): Promise<number> {
  return new Promise((done) => {
    requestAnimationFrame(() => {
      void document.body.offsetHeight;
      done(performance.now());
    });
  });
}

declare global {
  interface Window {
    measure: () => Promise<string>;
  }
}

window.measure = async () => {
  const tree = corpus();
  // Expanded folders are tracked by path.
  const expanded = new Set(tree.entries.filter((entry) => entry.kind === 'folder').map((entry) => entry.path));
  const noop = () => undefined;
  const searchRef = createRef<HTMLInputElement>();
  const container = document.getElementById('root');
  if (container === null) throw new Error('no root');
  const root = createRoot(container);

  const render = (view: SmartId) =>
    flushSync(() => {
      root.render(
        <StrictMode>
          <div style={{ height: '700px', display: 'grid', gridTemplateColumns: '240px' }}>
            <Sidebar
              tree={tree}
              view={view}
              expanded={expanded}
              selectedId={null}
              openPath={null}
              width={240}
              query=""
              hits={[]}
              searchRef={searchRef}
              renamingId={null}
              onQuery={noop}
              onRename={noop}
              onView={noop}
              onToggleFolder={noop}
              onSelect={noop}
              onOpen={noop}
              onContextMenu={noop}
              onWidth={noop}
              onNewScript={noop}
            />
          </div>
        </StrictMode>,
      );
    });

  const firstStart = performance.now();
  render('all');
  const firstMs = (await nextFrame()) - firstStart;
  const rows = document.querySelectorAll('[role="treeitem"], [role="row"], li, button').length;

  const switchStart = performance.now();
  render('recents');
  const switchMs = (await nextFrame()) - switchStart;

  const pureStart = performance.now();
  let count = 0;
  for (let i = 0; i < 20; i += 1) {
    count = sidebarRows(buildTree(tree.entries), 'all', expanded, { recents: tree.recents, pinned: tree.pins }, tree.entries).length;
  }
  const pureMs = (performance.now() - pureStart) / 20;

  root.unmount();
  return JSON.stringify({
    entries: tree.entries.length,
    rows: count,
    domNodes: rows,
    firstRenderMs: Math.round(firstMs * 10) / 10,
    viewSwitchMs: Math.round(switchMs * 10) / 10,
    rowBuildMs: Math.round(pureMs * 100) / 100,
  });
};
