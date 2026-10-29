// Picks the renderer for the selected model: the built-in pixel cat or a
// Live2D model. Switching models reloads the page so each renderer starts clean.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Webview errors are invisible without devtools; mirror them to the app's stderr.
function report(msg) {
  invoke('frontend_log', { msg: String(msg) }).catch(() => {});
}
window.addEventListener('error', (e) => report(`${e.message} @ ${e.filename}:${e.lineno}`));
window.addEventListener('unhandledrejection', (e) => report(e.reason?.stack || e.reason));

// Right-click the cat for the tray menu (Settings, size, quit, ...) instead
// of the webview's own Reload/Inspect menu.
window.addEventListener('contextmenu', (e) => {
  e.preventDefault();
  invoke('show_menu').catch((err) => report(`[menu] ${err}`));
});

async function boot() {
  const settings = await invoke('get_settings');
  await listen('settings-changed', ({ payload }) => {
    if (payload.model !== settings.model) location.reload();
  });

  if (settings.model !== 'pixel') {
    const models = await invoke('list_models');
    const info = models.find((m) => m.id === settings.model);
    if (info) {
      try {
        const { start } = await import('./live2d.js');
        await start(settings, info);
        return;
      } catch (e) {
        report(`[live2d] failed, falling back to pixel cat: ${e?.stack || e}`);
      }
    }
  }
  const { start } = await import('./pixel.js');
  await start(settings);
}

boot();
