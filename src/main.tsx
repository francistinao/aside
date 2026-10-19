import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './styles/tokens.css';
import './styles/library.css';
import { App } from './App';
import { applyBackdrop, applyPlatformClass } from './platform';

applyPlatformClass();
void applyBackdrop();

createRoot(document.getElementById('root') ?? document.body).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
