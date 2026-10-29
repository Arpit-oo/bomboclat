import '@fontsource-variable/plus-jakarta-sans';
import './settings.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { drawScene, CAT_PX_W, CAT_PX_H } from './cat-art.js';

const $ = (id) => document.getElementById(id);
const TOGGLES = ['flip', 'alwaysOnTop', 'clickThrough', 'hideOnHover', 'keyBubble', 'agentWatch', 'agentBubbles', 'sounds'];
const TEXTS = ['userName', 'pinnedMessage'];
const SELECTS = ['sleepAfterSecs', 'stretchMins', 'waterMins'];
const NUMBERS = ['pomodoroFocusMins', 'pomodoroBreakMins'];

let settings = null;
let models = [];
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

function modelUrl(id, rel) {
  const path = rel.split('/').map(encodeURIComponent).join('/');
  return `http://bcmodel.localhost/${encodeURIComponent(id)}/${path}`;
}

function pixelThumb() {
  const c = document.createElement('canvas');
  c.width = CAT_PX_W;
  c.height = CAT_PX_H;
  const ctx = c.getContext('2d');
  drawScene(
    {
      dot(x, y, col) { ctx.fillStyle = col; ctx.fillRect(Math.round(x), Math.round(y), 1, 1); },
      row(x1, x2, y, col) { ctx.fillStyle = col; ctx.fillRect(x1, y, x2 - x1 + 1, 1); },
      rect(x, y, w, h, col) { ctx.fillStyle = col; ctx.fillRect(x, y, w, h); },
    },
    { state: 'IDLE', time: 0, squash: 0, blink: false, eyeOffsetX: 0, eyeOffsetY: 0, leftPaw: 'up', rightPaw: 'up' },
  );
  return c;
}

function modelCard(id, name, src, thumb, removable) {
  const card = document.createElement('button');
  card.type = 'button';
  card.className = 'model';
  card.setAttribute('role', 'radio');
  card.dataset.id = id;
  card.append(thumb);
  const label = document.createElement('div');
  label.innerHTML = '<div class="name"></div><div class="src"></div>';
  label.querySelector('.name').textContent = name;
  label.querySelector('.name').title = name;
  label.querySelector('.src').textContent = src;
  card.append(label);
  card.addEventListener('click', () => patch({ model: id }));
  if (removable) {
    const x = document.createElement('span');
    x.className = 'remove';
    x.setAttribute('role', 'button');
    x.title = 'Remove this model';
    x.textContent = '×';
    x.style.cssText += 'display:grid;place-items:center;border:1px solid var(--line);cursor:pointer;';
    x.addEventListener('click', async (e) => {
      e.stopPropagation();
      if (!confirm(`Remove "${name}"? Its imported files will be deleted.`)) return;
      await invoke('remove_model', { id });
      await loadModels();
  await refreshHook();
    });
    card.append(x);
  }
  return card;
}

function renderModels() {
  const box = $('models');
  box.replaceChildren();
  const pix = document.createElement('div');
  pix.className = 'thumb pixel';
  pix.append(pixelThumb());
  box.append(modelCard('pixel', 'Pixel cat', 'Built in', pix, false));
  for (const m of models) {
    const t = document.createElement('div');
    t.className = 'thumb';
    const img = m.hasCover ? 'resources/cover.png' : m.hasBackground ? 'resources/background.png' : null;
    if (img) t.style.backgroundImage = `url("${modelUrl(m.id, img)}")`;
    const src = m.source === 'bongocat' ? 'From BongoCat' : 'Imported';
    box.append(modelCard(m.id, m.name, src, t, m.source === 'imported'));
  }
  $('models-hint').textContent = models.some((m) => m.source === 'bongocat')
    ? 'BongoCat models are read from your BongoCat install. Keys with no image in a model do not move its paws.'
    : 'Install BongoCat to use its models here, or import any Live2D (Cubism 3/4) model folder.';
  markModel();
}

function markModel() {
  for (const card of $('models').children) {
    card.setAttribute('aria-checked', String(card.dataset.id === settings?.model));
  }
}

async function loadModels() {
  models = await invoke('list_models');
  renderModels();
}

$('import').addEventListener('click', async () => {
  try {
    const m = await invoke('import_model');
    if (!m) return;
    await loadModels();
    await patch({ model: m.id });
  } catch (e) {
    alert(String(e));
  }
});

function render(s) {
  settings = s;
  markModel();
  for (const id of TOGGLES) $(id).checked = s[id];
  $('size').value = s.size;
  $('size-out').textContent = `${s.size}%`;
  $('opacity').value = s.opacity;
  $('opacity-out').textContent = `${s.opacity}%`;
  for (const id of SELECTS) $(id).value = String(s[id]);
  // Don't clobber a field while the user is typing in it.
  for (const id of [...TEXTS, ...NUMBERS]) if (document.activeElement !== $(id)) $(id).value = s[id];
  if (!$('reminders').contains(document.activeElement)) renderReminders(s.reminders);
}

function renderReminders(list) {
  const box = $('reminders');
  box.replaceChildren();
  list.forEach((r, i) => {
    const row = document.createElement('div');
    row.className = 'reminder';
    const time = Object.assign(document.createElement('input'), { type: 'time', value: r.time });
    time.setAttribute('aria-label', 'Time');
    const text = Object.assign(document.createElement('input'), {
      type: 'text', className: 'text', value: r.message, maxLength: 80, placeholder: 'Message',
    });
    text.setAttribute('aria-label', 'Message');
    const del = Object.assign(document.createElement('button'), { type: 'button', textContent: 'Remove' });
    const save = () => {
      const next = settings.reminders.map((x) => ({ ...x }));
      next[i] = { time: time.value || r.time, message: text.value };
      patch({ reminders: next });
    };
    time.addEventListener('change', save);
    text.addEventListener('change', save);
    del.addEventListener('click', () => patch({ reminders: settings.reminders.filter((_, j) => j !== i) }));
    row.append(time, text, del);
    box.append(row);
  });
}

$('addReminder').addEventListener('click', async () => {
  await patch({ reminders: [...settings.reminders, { time: '12:00', message: 'Lunch time!' }] });
  $('reminders').lastElementChild?.querySelector('input.text')?.focus();
});

for (const id of TEXTS) $(id).addEventListener('change', (e) => patch({ [id]: e.target.value }));
for (const id of NUMBERS) {
  $(id).addEventListener('change', (e) => patch({ [id]: Number(e.target.value) || settings[id] }));
}
$('stretchMins').addEventListener('change', (e) => patch({ stretchMins: Number(e.target.value) }));
$('waterMins').addEventListener('change', (e) => patch({ waterMins: Number(e.target.value) }));
$('testBubble').addEventListener('click', () => invoke('test_bubble'));

async function refreshHook() {
  try {
    const st = await invoke('claude_hook_status');
    $('claudeHook').checked = st.installed;
    if (st.stale) $('claudeHookHint').textContent = 'The hook points at another copy of Bomboclat. Toggle off and on to repair it.';
  } catch {
    $('claudeHook').disabled = true;
  }
}

$('claudeHook').addEventListener('change', async (e) => {
  try {
    await invoke('set_claude_hook', { on: e.target.checked });
    showSaved();
  } catch (err) {
    alert(String(err));
  }
  refreshHook();
});
$('openSessions').addEventListener('click', () => invoke('open_sessions_window'));

let focusRunning = false;
$('focus').addEventListener('click', () => invoke(focusRunning ? 'stop_focus_cmd' : 'start_focus_cmd'));

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
  await loadModels();
  $('autostart').checked = await invoke('get_autostart');
  await listen('settings-changed', ({ payload }) => render(payload));
  await listen('pomodoro', ({ payload }) => {
    focusRunning = !!payload.phase;
    $('focus').textContent = focusRunning ? 'Stop' : 'Start';
  });
  requestAnimationFrame(draw);
}

init();
