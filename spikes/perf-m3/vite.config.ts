import { resolve } from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// Builds the two timing pages into spikes/perf-m3/dist for a plain WKWebView.
export default defineConfig({
  root: import.meta.dirname,
  base: './',
  plugins: [react()],
  input: {
    typing: resolve(import.meta.dirname, 'index.html'),
    sidebar: resolve(import.meta.dirname, 'sidebar.html'),
  },
  build: {
    target: ['safari17'],
    outDir: resolve(import.meta.dirname, 'dist'),
    emptyOutDir: true,
    minify: false,
  },
});
