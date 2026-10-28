import '@fontsource-variable/plus-jakarta-sans';
import './settings.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { drawScene, CAT_PX_W, CAT_PX_H } from './cat-art.js';

const $ = (id) => document.getElementById(id);
const TOGGLES = ['flip', 'alwaysOnTop', 'clickThrough', 'hideOnHover', 'keyBubble'];

let settings = null;
let statusTimer = 0;

function showSaved() {
  $('status').textContent = 'Saved';
  clearTimeout(statusTimer);
  statusTimer = setTimeout(() => ($('status').textContent = ''), 1200);
}

async function patch(p) {
  render(await invoke('update_settings', { patch: p }));
  showSaved();
}

function render(s) {
  settings = s;
  for (const id of TOGGLES) $(id).checked = s[id];
  $('size').value = s.size;
  $('size-out').textContent = `${s.size}%`;
  $('opacity').value = s.opacity;
  $('opacity-out').textContent = `${s.opacity}%`;
  $('sleepAfterSecs').value = String(s.sleepAfterSecs);
}

for (const id of TOGGLES) {
  $(id).addEventListener('change', (e) => patch({ [id]: e.target.checked }));
}

// Labels follow the thumb live; size only commits on release since each
// commit resizes the cat window.
$('size').addEventListener('input', (e) => ($('size-out').textContent = `${e.target.value}%`));
$('size').addEventListener('change', (e) => patch({ size: Number(e.target.value) }));
$('opacity').addEventListener('input', (e) => {
  $('opacity-out').textContent = `${e.target.value}%`;
  patch({ opacity: Number(e.target.value) });
});
$('sleepAfterSecs').addEventListener('change', (e) => patch({ sleepAfterSecs: Number(e.target.value) }));

$('autostart').addEventListener('change', async (e) => {
  e.target.checked = await invoke('set_autostart_cmd', { on: e.target.checked });
  showSaved();
});

$('reset').addEventListener('click', async () => {
  render(await invoke('reset_settings'));
  showSaved();
});

// Mini cat in the header: watches the cursor and taps when you click.
const preview = $('preview');
preview.width = CAT_PX_W;
preview.height = CAT_PX_H;
const pctx = preview.getContext('2d');
const plotter = {
  dot(x, y, c) { pctx.fillStyle = c; pctx.fillRect(Math.round(x), Math.round(y), 1, 1); },
  row(x1, x2, y, c) { pctx.fillStyle = c; pctx.fillRect(x1, y, x2 - x1 + 1, 1); },
  rect(x, y, w, h, c) { pctx.fillStyle = c; pctx.fillRect(x, y, w, h); },
};
const look = { x: 0, y: 0 };
let tapUntil = 0;
let tapSide = 'left';
let blinkUntil = 0;
let nextBlink = Date.now() + 2000;

window.addEventListener('mousemove', (e) => {
  const r = preview.getBoundingClientRect();
  look.x = Math.max(-1, Math.min(1, (e.clientX - (r.left + r.width / 2)) / 200));
  look.y = Math.max(-1, Math.min(1, (e.clientY - (r.top + r.height * 0.3)) / 200));
});
window.addEventListener('mousedown', () => {
  tapSide = tapSide === 'left' ? 'right' : 'left';
  tapUntil = Date.now() + 120;
});

function draw() {
  const now = Date.now();
  if (now >= nextBlink) {
    blinkUntil = now + 140;
    nextBlink = now + 2500 + Math.random() * 3500;
  }
  const flip = settings?.flip ?? false;
  const tapping = now < tapUntil;
  pctx.setTransform(1, 0, 0, 1, 0, 0);
  pctx.clearRect(0, 0, CAT_PX_W, CAT_PX_H);
  if (flip) pctx.setTransform(-1, 0, 0, 1, CAT_PX_W, 0);
  drawScene(plotter, {
    state: 'IDLE',
    time: now,
    squash: 0,
    blink: now < blinkUntil,
    eyeOffsetX: flip ? -look.x : look.x,
    eyeOffsetY: look.y,
    leftPaw: tapping && tapSide === (flip ? 'right' : 'left') ? 'down' : 'up',
    rightPaw: tapping && tapSide === (flip ? 'left' : 'right') ? 'down' : 'up',
  });
  preview.style.opacity = settings ? Math.max(0.35, settings.opacity / 100) : 1;
  requestAnimationFrame(draw);
}

async function init() {
  render(await invoke('get_settings'));
  $('autostart').checked = await invoke('get_autostart');
  await listen('settings-changed', ({ payload }) => render(payload));
  requestAnimationFrame(draw);
}

init();
