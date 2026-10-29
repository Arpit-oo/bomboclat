// Live2D renderer for BongoCat-style models: background image, the Live2D
// model, then one overlay image per pressed key. Paws follow which key folder
// (left-keys / right-keys) a pressed key's image lives in, like BongoCat.
import * as PIXI from 'pixi.js';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
import { modelKeyName } from './keys.js';
import { createBubbles } from './bubble.js';

const CORE_SRC = '/live2dcubismcore.min.js';
// Quick taps still show the paw down for at least this long.
const MIN_PAW_DOWN_MS = 90;
// Held keys whose release we never saw (e.g. Win+L) are dropped after this.
const STUCK_KEY_MS = 30_000;
// Opacity while the cursor is over the cat with hide-on-hover on.
const HOVER_ALPHA = 0.12;

const appWindow = getCurrentWindow();

function loadCore() {
  if (window.Live2DCubismCore) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const s = document.createElement('script');
    s.src = CORE_SRC;
    s.onload = resolve;
    s.onerror = () => reject(new Error('Live2D Cubism Core missing. Run `npm run setup:live2d`.'));
    document.head.appendChild(s);
  });
}

function modelUrl(id, rel) {
  // Windows serves custom protocols as http://<scheme>.localhost/.
  const path = rel.split('/').map(encodeURIComponent).join('/');
  return `http://bcmodel.localhost/${encodeURIComponent(id)}/${path}`;
}

function el(tag, props) {
  return Object.assign(document.createElement(tag), props);
}

/** Runs a Live2D model described by `info` (from the list_models command). */
export async function start(initialSettings, info) {
  await loadCore();
  // pixi-live2d-display looks for PIXI on window.
  window.PIXI = PIXI;
  const { Live2DModel, SoundManager } = await import('pixi-live2d-display/cubism4');
  Live2DModel.registerTicker(PIXI.Ticker);

  let settings = initialSettings;
  const stage = el('div', { className: 'stage' });
  if (info.hasBackground) {
    stage.appendChild(el('img', { className: 'layer', src: modelUrl(info.id, 'resources/background.png') }));
  }
  const canvas = el('canvas', { className: 'layer' });
  const keyLayer = el('div', { className: 'layer' });
  stage.append(canvas, keyLayer);

  // One hidden <img> per key image, shown while that key is held.
  const keyImages = new Map();
  for (const [side, names] of [['left', info.leftKeys], ['right', info.rightKeys]]) {
    for (const name of names) {
      if (keyImages.has(name)) continue;
      const img = el('img', { className: 'layer key', src: modelUrl(info.id, `resources/${side}-keys/${name}.png`) });
      keyLayer.appendChild(img);
      keyImages.set(name, { side, img });
    }
  }

  const app = new PIXI.Application({
    view: canvas,
    resizeTo: window,
    backgroundAlpha: 0,
    antialias: true,
    autoDensity: true,
    resolution: window.devicePixelRatio,
  });
  let model;
  try {
    model = await Live2DModel.from(modelUrl(info.id, info.modelFile), { autoInteract: false });
  } catch (e) {
    app.destroy();
    throw e;
  }
  app.stage.addChild(model);
  // Only swap out the pixel canvas once the model loaded, so the caller can
  // still fall back to the pixel cat.
  document.getElementById('cat').remove();
  document.body.appendChild(stage);

  const natural = { w: model.width, h: model.height };
  function fit() {
    const s = Math.min(window.innerWidth / natural.w, window.innerHeight / natural.h);
    model.scale.set(s);
    model.x = (window.innerWidth - natural.w * s) / 2;
    model.y = window.innerHeight - natural.h * s;
  }
  fit();
  window.addEventListener('resize', fit);

  // Parameters we drive, written straight into the core model just before it
  // updates each frame so motions/breath/blink can't overwrite them.
  const raw = model.internalModel.coreModel.getModel();
  const paramIndex = new Map(raw.parameters.ids.map((id, i) => [id, i]));
  const params = new Map();
  function range(id) {
    const i = paramIndex.get(id);
    return i === undefined ? null : { min: raw.parameters.minimumValues[i], max: raw.parameters.maximumValues[i] };
  }
  model.internalModel.on('beforeModelUpdate', () => {
    for (const [id, v] of params) {
      const i = paramIndex.get(id);
      if (i !== undefined) raw.parameters.values[i] = v;
    }
  });

  // Input state.
  const held = new Map(); // id -> { side, name, at }
  const pawUntil = { left: 0, right: 0 };
  const mouseDown = { left: false, right: false };

  function press(id, side, name) {
    if (held.has(id)) return; // OS auto-repeat
    const now = Date.now();
    held.set(id, { side, name, at: now });
    if (side) pawUntil[side] = now + MIN_PAW_DOWN_MS;
  }

  function onInput(e) {
    switch (e.kind) {
      case 'keyDown': {
        const name = modelKeyName(e.key);
        const k = keyImages.get(name);
        // Like BongoCat: a key the model has no image for gets no reaction.
        if (k) press(e.key, k.side, name);
        break;
      }
      case 'keyUp':
        held.delete(e.key);
        break;
      case 'mouseDown':
        if (e.button === 'left' || e.button === 'right') mouseDown[e.button] = true;
        break;
      case 'mouseUp':
        if (e.button === 'left' || e.button === 'right') mouseDown[e.button] = false;
        break;
    }
  }

  // Cursor -> ParamMouseX/Y and head angle, relative to the monitor the cat is on.
  const win = { x: 0, y: 0, w: 0, h: 0 };
  let monitor = null;
  let hoverInside = false;
  async function syncGeometry() {
    const pos = await appWindow.outerPosition();
    const size = await appWindow.outerSize();
    Object.assign(win, { x: pos.x, y: pos.y, w: size.width, h: size.height });
    monitor = await currentMonitor();
  }
  function onCursor({ x, y }) {
    hoverInside = x >= win.x && x < win.x + win.w && y >= win.y && y < win.y + win.h;
    if (!monitor) return;
    const clamp01 = (v) => Math.max(0, Math.min(1, v));
    let rx = clamp01((x - monitor.position.x) / monitor.size.width);
    const ry = clamp01((y - monitor.position.y) / monitor.size.height);
    // The stage is mirrored when flipped, so mirror the input too.
    if (settings.flip) rx = 1 - rx;
    for (const id of ['ParamMouseX', 'ParamMouseY', 'ParamAngleX', 'ParamAngleY']) {
      const r = range(id);
      if (!r) continue;
      const ratio = id.endsWith('X') ? rx : ry;
      params.set(id, r.max - ratio * (r.max - r.min));
    }
  }

  let alpha = settings.opacity / 100;
  let bubbles = null;
  // Models ship their own happy motion (BongoCat: CAT_motion, with a sound).
  const motions = model.internalModel.settings.motions ?? {};
  const doneMotion = ['CAT_motion', 'Tap', 'tap_body', 'TapBody'].find((g) => motions[g]?.length);
  function react(kind) {
    if (kind === 'done' && doneMotion) model.motion(doneMotion);
  }
  function applySettings(next) {
    settings = next;
    SoundManager.volume = settings.sounds ? 0.5 : 0;
    bubbles?.applySettings(next);
    stage.style.transform = settings.flip ? 'scaleX(-1)' : '';
  }
  applySettings(settings);

  app.ticker.add(() => {
    const now = Date.now();
    const down = { left: now < pawUntil.left, right: now < pawUntil.right };
    const showing = new Set();
    for (const [id, h] of held) {
      if (now - h.at > STUCK_KEY_MS) {
        held.delete(id);
        continue;
      }
      if (h.side) down[h.side] = true;
      if (h.name) showing.add(h.name);
    }
    for (const [name, k] of keyImages) k.img.classList.toggle('on', showing.has(name));
    params.set('CatParamLeftHandDown', down.left ? 1 : 0);
    params.set('CatParamRightHandDown', down.right ? 1 : 0);
    params.set('ParamMouseLeftDown', mouseDown.left ? 1 : 0);
    params.set('ParamMouseRightDown', mouseDown.right ? 1 : 0);

    const base = settings.opacity / 100;
    const target = settings.hideOnHover && hoverInside ? Math.min(base, HOVER_ALPHA) : base;
    alpha += (target - alpha) * 0.2;
    if (Math.abs(target - alpha) < 0.005) alpha = target;
    stage.style.opacity = alpha.toFixed(3);
  });

  stage.addEventListener('mousedown', (e) => {
    if (e.button === 0) appWindow.startDragging();
  });

  await syncGeometry();
  await appWindow.onMoved(syncGeometry);
  await appWindow.onResized(syncGeometry);
  await listen('cursor', ({ payload }) => onCursor(payload));
  await listen('input', ({ payload }) => onInput(payload));
  await listen('settings-changed', ({ payload }) => applySettings(payload));
  bubbles = await createBubbles(settings, react);
}
