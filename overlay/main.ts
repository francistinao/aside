import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import '../assets/tokens.css';
import './bubble.css';
import type { ScriptPayload } from './script';
import { View } from './view';
import type { CaptureStatus, Corner, Position, StateEvent, Style } from './view';

const root = document.getElementById('root') ?? document.body;
const view = new View(root, {
  onLines: (ranges) => void invoke('session_set_lines', { ranges }),
  onResize: (width, height) => void invoke('prompter_resize', { width, height }),
});

void listen<ScriptPayload>('session://script', (event) => view.setScript(event.payload));
void listen<Position>('session://position', (event) => view.setPosition(event.payload));
void listen<StateEvent>('session://state', (event) => view.setState(event.payload));
void listen<Style>('prompter://style', (event) => view.setStyle(event.payload));
void listen<{ corner: Corner }>('prompter://corner', (event) => view.setCorner(event.payload.corner));
void listen<CaptureStatus>('capture://status', (event) => view.setStatus(event.payload));
