import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { drawScene, CAT_PX_W, CAT_PX_H } from './cat-art.js';
import { keySide, comboLabel } from './keys.js';

const PIXEL_SCALE = 4;
const CAT_W = CAT_PX_W * PIXEL_SCALE;
const CAT_H = CAT_PX_H * PIXEL_SCALE;
const SLEEP_AFTER_MS = 60_000;
// Eye row in art pixels, used as the origin for cursor tracking.
const EYE_ROW = 12;
// Top of the ears in art pixels; the key bubble floats above this.
const HEAD_TOP_ROW = 2;
// Quick taps still show the paw down for at least this long.
const MIN_PAW_DOWN_MS = 90;
// Held keys whose release we never saw (e.g. Win+L) are dropped after this.
const STUCK_KEY_MS = 30_000;
const BUBBLE_MS = 1200;
const BUBBLE_FADE_MS = 300;

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
  blinkUntil: 0,
  nextBlink: Date.now() + 3000,
};

// key/button id -> { side, at }. Side is fixed at press time so space can alternate.
const held = new Map();
const pawUntil = { left: 0, right: 0 };
let spaceSide = 'left';
const bubble = { text: '', until: 0 };

// Window geometry in physical pixels, kept current for cursor math.
const win = { x: 0, y: 0, scale: 1 };

function catOrigin() {
  return { x: window.innerWidth / 2, y: window.innerHeight };
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

function press(id, side) {
  if (held.has(id)) return; // OS auto-repeat
  if (side === 'both') {
    side = spaceSide;
    spaceSide = side === 'left' ? 'right' : 'left';
  }
  const now = Date.now();
  held.set(id, { side, at: now });
  pawUntil[side] = now + MIN_PAW_DOWN_MS;
  cat.squash = 0.5;
}

function onInput(e) {
  wake();
  switch (e.kind) {
    case 'keyDown': {
      const repeat = held.has(e.key);
      press(e.key, keySide(e.key));
      if (!repeat) {
        bubble.text = comboLabel(held.keys(), e.key);
        bubble.until = Date.now() + BUBBLE_MS;
      }
      break;
    }
    case 'keyUp':
      held.delete(e.key);
      break;
    case 'mouseDown':
      press(`mouse:${e.button}`, 'right');
      break;
    case 'mouseUp':
      held.delete(`mouse:${e.button}`);
      break;
  }
}

function pawState(side, now) {
  if (now < pawUntil[side]) return 'down';
  for (const [id, h] of held) {
    if (now - h.at > STUCK_KEY_MS) held.delete(id);
    else if (h.side === side) return 'down';
  }
  return 'up';
}

function tickBlink(now) {
  if (now >= cat.nextBlink) {
    cat.blinkUntil = now + 140;
    cat.nextBlink = now + 2500 + Math.random() * 3500;
  }
}

function drawBubble(now) {
  if (now >= bubble.until || !bubble.text) return;
  const o = catOrigin();
  const left = bubble.until - now;
  ctx.globalAlpha = left < BUBBLE_FADE_MS ? left / BUBBLE_FADE_MS : 1;
  ctx.font = 'bold 13px Consolas, "Cascadia Mono", monospace';
  const w = Math.ceil(ctx.measureText(bubble.text).width) + 16;
  const h = 22;
  const x = Math.round(o.x - w / 2);
  const y = Math.round(o.y - CAT_H + HEAD_TOP_ROW * PIXEL_SCALE - h - 8);
  // Pixel-style box: dark border drawn as a 2px frame, no rounded corners.
  ctx.fillStyle = '#111111';
  ctx.fillRect(x - 2, y - 2, w + 4, h + 4);
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = '#111111';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(bubble.text, o.x, y + h / 2 + 1);
  ctx.globalAlpha = 1;
}

function render() {
  const now = Date.now();
  if (cat.state !== 'SLEEP' && now - cat.lastInput > SLEEP_AFTER_MS) cat.state = 'SLEEP';
  cat.squash = Math.max(0, cat.squash - 0.08);
  tickBlink(now);

  px.clearRect(0, 0, CAT_PX_W, CAT_PX_H);
  drawScene(plotter, {
    ...cat,
    time: now,
    blink: now < cat.blinkUntil,
    leftPaw: pawState('left', now),
    rightPaw: pawState('right', now),
  });

  ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
  const o = catOrigin();
  ctx.save();
  ctx.translate(o.x, o.y);
  // Squash only the cat's height a touch; the desk is part of the image so keep it subtle.
  ctx.scale(1 + cat.squash * 0.02, 1 - cat.squash * 0.03);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(pxCanvas, -CAT_W / 2, -CAT_H, CAT_W, CAT_H);
  ctx.restore();

  if (cat.state === 'SLEEP') {
    const count = 1 + (Math.floor(now / 600) % 3);
    ctx.font = `bold ${12 + count * 2}px Consolas, monospace`;
    ctx.fillStyle = 'rgba(120,120,255,0.9)';
    ctx.textAlign = 'center';
    ctx.fillText('z'.repeat(count), o.x + 52, o.y - CAT_H + 10 - count * 4);
  }

  drawBubble(now);
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
