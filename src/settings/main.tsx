import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '../styles/tokens.css';
import '../styles/settings.css';
import { applyPlatformClass } from '../platform';
import { SettingsApp } from './SettingsApp';

applyPlatformClass();

createRoot(document.getElementById('root') ?? document.body).render(
  <StrictMode>
    <SettingsApp />
  </StrictMode>,
);
