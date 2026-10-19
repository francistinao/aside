import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

createRoot(document.getElementById('root') ?? document.body).render(
  <StrictMode>
    <main />
  </StrictMode>,
);
