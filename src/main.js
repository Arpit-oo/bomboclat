import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { drawCatArt, CAT_PX_W, CAT_PX_H } from './cat-art.js';

const PIXEL_SCALE = 4;
const CAT_W = CAT_PX_W * PIXEL_SCALE;
const CAT_H = CAT_PX_H * PIXEL_SCALE;
const SLEEP_AFTER_MS = 60_000;
// Eye row in art pixels, used as the origin for cursor tracking.
const EYE_ROW = 12;

const appWindow = getCurrentWindow();
const canvas = document.getElementById('cat');
const ctx = canvas.getContext('2d');

const pxCanvas = document.createElement('canvas');
pxCanvas.width = CAT_PX_W;
pxCanvas.height = CAT_PX_H;
const px = pxCanvas.getContext('2d');

const plotter = {
  dot(x, y, c) {
    px.fillStyle = c;
    px.fillRect(Math.round(x), Math.round(y), 1, 1);
  },
  row(x1, x2, y, c) {
    px.fillStyle = c;
    px.fillRect(x1, y, x2 - x1 + 1, 1);
  },
  rect(x, y, w, h, c) {
    px.fillStyle = c;
    px.fillRect(x, y, w, h);
  },
};

const cat = {
  state: 'IDLE',
  eyeOffsetX: 0,
  eyeOffsetY: 0,
  squash: 0,
  lastInput: Date.now(),
};

// Window geometry in physical pixels, kept current for cursor math.
const win = { x: 0, y: 0, scale: 1 };

function catOrigin() {
  const w = window.innerWidth;
  const h = window.innerHeight;
  return { x: w / 2, y: h - 8 };
}

function resizeCanvas() {
  const dpr = window.devicePixelRatio;
  canvas.width = window.innerWidth * dpr;
  canvas.height = window.innerHeight * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function wake() {
  cat.lastInput = Date.now();
  if (cat.state === 'SLEEP') cat.state = 'IDLE';
}

function onCursor({ x, y }) {
  const o = catOrigin();
  const eyeX = win.x + o.x * win.scale;
  const eyeY = win.y + (o.y - CAT_H + EYE_ROW * PIXEL_SCALE) * win.scale;
  const range = 200 * win.scale;
  cat.eyeOffsetX = Math.max(-1, Math.min(1, (x - eyeX) / range));
  cat.eyeOffsetY = Math.max(-1, Math.min(1, (y - eyeY) / range));
}

function onInput(e) {
  wake();
  if (e.kind === 'keyDown' || e.kind === 'mouseDown') cat.squash = 1;
}

function render() {
  const now = Date.now();
  if (cat.state !== 'SLEEP' && now - cat.lastInput > SLEEP_AFTER_MS) cat.state = 'SLEEP';
  cat.squash = Math.max(0, cat.squash - 0.12);

  px.clearRect(0, 0, CAT_PX_W, CAT_PX_H);
  drawCatArt(plotter, { ...cat, time: now });

  ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
  const o = catOrigin();
  const sy = 1 - cat.squash * 0.08;
  const sx = 1 + cat.squash * 0.05;
  ctx.save();
  ctx.translate(o.x, o.y);
  ctx.scale(sx, sy);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(pxCanvas, -CAT_W / 2, -CAT_H, CAT_W, CAT_H);
  ctx.restore();

  if (cat.state === 'SLEEP') {
    const count = 1 + (Math.floor(now / 600) % 3);
    ctx.font = `${12 + count * 2}px sans-serif`;
    ctx.fillStyle = 'rgba(120,120,255,0.8)';
    ctx.textAlign = 'center';
    ctx.fillText('z'.repeat(count), o.x + 50, o.y - CAT_H - count * 4 + 20);
  }

  requestAnimationFrame(render);
}

canvas.addEventListener('mousedown', (e) => {
  if (e.button === 0) appWindow.startDragging();
});

async function syncGeometry() {
  const pos = await appWindow.outerPosition();
  win.x = pos.x;
  win.y = pos.y;
  win.scale = await appWindow.scaleFactor();
}

async function init() {
  resizeCanvas();
  window.addEventListener('resize', resizeCanvas);
  await syncGeometry();
  await appWindow.onMoved(({ payload }) => {
    win.x = payload.x;
    win.y = payload.y;
  });
  await appWindow.onScaleChanged(({ payload }) => {
    win.scale = payload.scaleFactor;
  });
  await listen('cursor', ({ payload }) => onCursor(payload));
  await listen('input', ({ payload }) => onInput(payload));
  requestAnimationFrame(render);
}

init();
