import { defineConfig } from 'vite';

export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    target: 'es2021',
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: { input: { main: 'index.html', settings: 'settings.html', sessions: 'sessions.html' } },
  },
});
