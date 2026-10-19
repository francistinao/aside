import { listen } from '@tauri-apps/api/event';
import '../assets/tokens.css';
import './bubble.css';
import sample from '../assets/sample-script.md?raw';
import { breakLines, pickLines } from './sample';

type CaptureStatus = 'hidden' | 'mayBeVisible' | 'visible' | 'unsupported' | 'failed';
type Corner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

// Copy deck in docs/PRODUCT.md. Unsupported and Failed read as Visible here;
// Settings explains why.
const STATUS_LABEL: Record<CaptureStatus, string> = {
  hidden: 'Hidden',
  mayBeVisible: 'May be visible',
  visible: 'Visible',
  unsupported: 'Visible',
  failed: 'Visible',
};

const STATUS_DOT: Partial<Record<CaptureStatus, string>> = {
  hidden: 'success',
  mayBeVisible: 'warning',
};

function element(tag: string, className: string, text = ''): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

// Measures with the real font so the three lines are visual lines, not sentences.
function makeFitTest(reference: HTMLElement, width: number): (line: string) => boolean {
  const context = document.createElement('canvas').getContext('2d');
  if (context === null) return (line) => line.length <= 36;
  context.font = getComputedStyle(reference).font;
  return (line) => context.measureText(line).width <= width;
}

function build(root: HTMLElement) {
  const bubble = element('div', 'bubble');
  bubble.dataset.corner = 'bottom-right';
  bubble.setAttribute('aria-hidden', 'true');

  const lines = element('div', 'lines');
  const previous = element('p', 'line previous');
  const current = element('p', 'line current');
  const next = element('p', 'line next');
  lines.append(previous, current, next);

  const footer = element('div', 'footer');
  const status = element('span', 'status');
  const dot = element('span', 'dot');
  const label = element('span', 'status-label', STATUS_LABEL.visible);
  status.append(dot, label);
  footer.append(status);

  bubble.append(lines, footer);
  root.replaceChildren(bubble);

  const fits = makeFitTest(current, lines.clientWidth);
  const visual = pickLines(sample).flatMap((paragraph) => breakLines(paragraph, fits));
  previous.textContent = visual[0] ?? '';
  current.textContent = visual[1] ?? '';
  next.textContent = visual[2] ?? '';

  return {
    setStatus(value: CaptureStatus) {
      label.textContent = STATUS_LABEL[value];
      dot.dataset.tone = STATUS_DOT[value] ?? '';
    },
    setCorner(corner: Corner) {
      bubble.dataset.corner = corner;
    },
  };
}

const root = document.getElementById('root') ?? document.body;
const view = build(root);

void listen<CaptureStatus>('capture://status', (event) => view.setStatus(event.payload));
void listen<{ corner: Corner }>('prompter://style', (event) => view.setCorner(event.payload.corner));
