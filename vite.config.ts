import { resolve } from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

// The Tauri CLI sets TAURI_ENV_DEBUG for `tauri dev` and debug builds.
const debug = process.env.TAURI_ENV_DEBUG === 'true';
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  input: {
    library: resolve(import.meta.dirname, 'index.html'),
    settings: resolve(import.meta.dirname, 'settings.html'),
    overlay: resolve(import.meta.dirname, 'overlay.html'),
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  server: {
    port: 1420,
    strictPort: true,
    host: host ?? false,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    // WKWebView follows the OS (minimum macOS 14, Safari 17). WebView2 is evergreen.
    target: ['safari17', 'chrome120'],
    minify: !debug,
    sourcemap: debug,
  },
  test: {
    include: ['src/**/*.test.{ts,tsx}', 'overlay/**/*.test.ts'],
  },
});
