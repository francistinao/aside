// Draws what Rust sends. Position events change classes on at most three lines and two
// words and move one container with a transform. Layout is read only when a script loads
// or the text size changes.

import { formatProgress } from './format';
import { linesFromRects, measure } from './lines';
import { buildScript, fillLines, fillParagraphs } from './script';
import type { BuiltScript, ScriptPayload } from './script';

export type CaptureStatus = 'hidden' | 'mayBeVisible' | 'visible' | 'unsupported' | 'failed';
export type Corner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
export type Highlight = 'word' | 'line';
export type TextSize = 'small' | 'medium' | 'large';
export type SessionState = 'idle' | 'countdown' | 'running' | 'paused' | 'ended';
export type Visibility = 'visible' | 'peeking' | 'fading' | 'hidden';

export interface Position {
  word: number | null;
  line: number;
  section: number;
  elapsed: number;
  total: number;
  pause: number | null;
}

export interface StateEvent {
  state: SessionState;
  count?: number;
}

export interface Style {
  corner: Corner;
  opacity: number;
  highlight: Highlight;
  textSize: TextSize;
}

export interface ViewCallbacks {
  onLines(ranges: [number, number][]): void;
  onResize(width: number, height: number): void;
}

// Copy deck in docs/PRODUCT.md. Unsupported and Failed read as Visible; Settings explains why.
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

/// The window is this much larger than the bubble on every side (see bubble.css).
const WINDOW_INSET = 4;
const WINDOW_WIDTH = 368;
const END_OF_SCRIPT = 'End of script';
const PAUSED = 'Paused';

function element(tag: string, className: string, text = ''): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

export class View {
  private readonly bubble: HTMLElement;
  private readonly label: HTMLElement;
  private readonly lines: HTMLElement;
  private readonly message: HTMLElement;
  private readonly progressFill: HTMLElement;
  private readonly time: HTMLElement;
  private readonly dot: HTMLElement;
  private readonly statusLabel: HTMLElement;
  private readonly reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

  private script: BuiltScript | null = null;
  private lineElements: HTMLElement[] = [];
  private lineHeight = 0;
  private shown: HTMLElement[] = [];
  private currentWord: HTMLElement | null = null;
  private currentLine = -1;
  private section = -1;
  private state: SessionState = 'idle';
  private textSize: TextSize = 'small';
  private lastHeight = 0;

  constructor(
    root: HTMLElement,
    private readonly callbacks: ViewCallbacks,
  ) {
    this.bubble = element('div', 'bubble');
    this.bubble.dataset.corner = 'bottom-right';
    this.bubble.dataset.mode = 'word';
    this.bubble.dataset.size = 'small';
    this.bubble.dataset.state = 'idle';
    this.bubble.dataset.visibility = 'visible';
    this.bubble.setAttribute('aria-hidden', 'true');

    this.label = element('div', 'label');
    const viewport = element('div', 'viewport');
    this.lines = element('div', 'lines');
    this.message = element('div', 'message');
    viewport.append(this.lines, this.message);

    const footer = element('div', 'footer');
    const progress = element('div', 'progress');
    this.progressFill = element('div', 'progress-fill');
    progress.append(this.progressFill);
    const meta = element('div', 'meta');
    this.time = element('span', 'time');
    const status = element('span', 'status');
    this.dot = element('span', 'dot');
    this.statusLabel = element('span', 'status-label', STATUS_LABEL.visible);
    status.append(this.dot, this.statusLabel);
    meta.append(this.time, status);
    footer.append(progress, meta);

    this.bubble.append(this.label, viewport, footer);
    root.replaceChildren(this.bubble);
  }

  setScript(payload: ScriptPayload): void {
    this.script = buildScript(payload);
    this.currentWord = null;
    this.currentLine = -1;
    this.section = -1;
    this.shown = [];
    this.label.textContent = '';
    this.layout();
  }

  /// Wraps the words at the real width and font, reports the lines, and regroups the DOM.
  private layout(): void {
    if (this.script === null) return;
    fillParagraphs(this.lines, this.script);
    const ranges = linesFromRects(measure(this.script.words, this.script.paragraphOf));
    this.lineElements = fillLines(this.lines, this.script, ranges);
    this.lineHeight = this.lineElements[0]?.offsetHeight ?? 0;
    this.lines.style.transition = 'none';
    this.moveTo(Math.max(this.currentLine, 0));
    this.lines.style.transition = '';
    this.callbacks.onLines(ranges);
    this.fitWindow();
  }

  /// Asks Rust to resize the window when the bubble's height changed.
  private fitWindow(): void {
    const height = this.bubble.offsetHeight + WINDOW_INSET * 2;
    if (height !== this.lastHeight && height > 0) {
      this.lastHeight = height;
      this.callbacks.onResize(WINDOW_WIDTH, height);
    }
  }

  setPosition(position: Position): void {
    if (position.line !== this.currentLine) {
      this.currentLine = position.line;
      for (const line of this.shown) line.classList.remove('previous', 'current', 'next');
      const previous = this.lineElements[position.line - 1];
      const current = this.lineElements[position.line];
      const next = this.lineElements[position.line + 1];
      this.shown = [previous, current, next].filter((line): line is HTMLElement => line !== undefined);
      previous?.classList.add('previous');
      current?.classList.add('current');
      next?.classList.add('next');
      this.moveTo(position.line);
      if (this.reducedMotion.matches) {
        this.lines.animate([{ opacity: 0.3 }, { opacity: 1 }], { duration: 140, easing: 'ease-out' });
      }
    }

    const word = position.word === null ? null : (this.script?.words[position.word] ?? null);
    if (word !== this.currentWord) {
      this.currentWord?.classList.remove('current');
      word?.classList.add('current');
      this.currentWord = word;
    }

    if (position.section !== this.section) {
      this.section = position.section;
      this.label.textContent = this.script?.headings[position.section - 1] ?? '';
    }

    const fraction = position.total > 0 ? Math.min(position.elapsed / position.total, 1) : 0;
    this.progressFill.style.transform = `scaleX(${fraction})`;
    this.time.textContent =
      position.pause !== null
        ? `Pause ${position.pause}s`
        : this.state === 'paused'
          ? PAUSED
          : formatProgress(position.elapsed, position.total);
  }

  setState(event: StateEvent): void {
    this.state = event.state;
    this.bubble.dataset.state = event.state;
    if (event.state === 'countdown') {
      this.message.textContent = String(event.count ?? '');
    } else if (event.state === 'ended') {
      this.message.textContent = END_OF_SCRIPT;
    } else {
      this.message.textContent = '';
    }
    if (event.state === 'paused') this.time.textContent = PAUSED;
  }

  setStyle(style: Style): void {
    this.bubble.dataset.mode = style.highlight;
    this.bubble.style.setProperty('--prompter-opacity', String(style.opacity / 100));
    if (style.textSize !== this.textSize) {
      this.textSize = style.textSize;
      this.bubble.dataset.size = style.textSize;
      this.layout();
    }
  }

  setCorner(corner: Corner): void {
    this.bubble.dataset.corner = corner;
  }

  /// Peek, fade while clicking and the hide toggle. The CSS fades between the states.
  setVisibility(state: Visibility): void {
    this.bubble.dataset.visibility = state;
  }

  setStatus(status: CaptureStatus): void {
    this.statusLabel.textContent = STATUS_LABEL[status];
    this.dot.dataset.tone = STATUS_DOT[status] ?? '';
  }

  /// Keeps the current line in the middle slot of the three line viewport.
  private moveTo(line: number): void {
    this.lines.style.transform = `translateY(${(1 - line) * this.lineHeight}px)`;
  }
}
