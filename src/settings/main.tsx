import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '../styles/tokens.css';

createRoot(document.getElementById('root') ?? document.body).render(
  <StrictMode>
    <main />
  </StrictMode>,
);
