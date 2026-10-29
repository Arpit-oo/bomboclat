// Terminal-style list of the Claude Code / Codex sessions the cat can see.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './sessions.css';

const $ = (id) => document.getElementById(id);
const ORDER = { input: 0, working: 1, done: 2, idle: 3 };
const SPINNER = '⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏';

let sessions = [];
let showIdle = false;
let frame = 0;
// id -> { key, el }: cards are reused while their content is unchanged.
const cards = new Map();

function cardFor(s) {
  // updatedMs ticks on every log write; the timer refreshes that text in place.
  const { updatedMs, ...rest } = s;
  const key = JSON.stringify(rest);
  const hit = cards.get(s.id);
  if (hit && hit.key === key) {
    hit.el.querySelector('.when').textContent = ago(updatedMs);
    return hit.el;
  }
  const el = row(s);
  cards.set(s.id, { key, el });
  return el;
}

function ago(ms) {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 10) return 'just now';
  if (s < 60) return `${s}s ago`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m}m ago`;
  return `${Math.floor(m / 60)}h ${m % 60}m ago`;
}

function tagText(s) {
  if (s.state === 'input') return '[NEEDS YOU]';
  if (s.maybeWaiting) return '[WAITING?]';
  if (s.state === 'working') return `[${SPINNER[frame % SPINNER.length]} WORKING]`;
  if (s.state === 'done') return '[DONE]';
  return '[IDLE]';
}

function detailText(s) {
  if (s.maybeWaiting) {
    const silent = Math.round((Date.now() - s.updatedMs) / 1000);
    return `${s.detail} · silent ${silent}s, it may be waiting for your approval in its terminal`;
  }
  if (s.state === 'done') return s.detail ? `Code is done. ${s.detail}` : 'Code is done.';
  return s.detail || (s.state === 'idle' ? 'Nothing happening' : '…');
}

function toast(text) {
  const t = Object.assign(document.createElement('div'), { className: 'toast', textContent: text });
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 4000);
}

async function run(cmd, args) {
  try {
    await invoke(cmd, args);
  } catch (e) {
    toast(String(e));
  }
}

function row(s) {
  const el = document.createElement('article');
  el.className = 'session';
  el.dataset.state = s.state;
  el.dataset.maybe = String(!!s.maybeWaiting);
  el.dataset.id = s.id;

  const tag = Object.assign(document.createElement('div'), { className: 'tag', textContent: tagText(s) });
  const head = document.createElement('div');
  head.className = 'head';
  const title = Object.assign(document.createElement('span'), { className: 'title', textContent: s.title || s.project });
  title.title = s.title;
  const where = document.createElement('span');
  where.className = 'where';
  where.textContent = `${s.agent === 'codex' ? 'codex · ' : ''}${s.project}`;
  if (s.branch) {
    const b = Object.assign(document.createElement('span'), { className: 'branch', textContent: s.branch });
    where.append(' ', b);
  }
  head.append(title, where);
  const when = Object.assign(document.createElement('div'), { className: 'when', textContent: ago(s.updatedMs) });
  const detail = Object.assign(document.createElement('div'), { className: 'detail', textContent: detailText(s) });

  const buttons = document.createElement('div');
  buttons.className = 'buttons';
  if (s.cwd) {
    const open = Object.assign(document.createElement('button'), { type: 'button', textContent: 'Open terminal here' });
    open.addEventListener('click', () => run('open_terminal', { cwd: s.cwd }));
    buttons.append(open);
  }
  // A running session already has its terminal; resuming it twice would fork it.
  if (s.agent === 'claude' && s.state !== 'working' && s.cwd) {
    const resume = Object.assign(document.createElement('button'), { type: 'button', textContent: 'Resume in new terminal' });
    resume.title = `claude --resume ${s.id}`;
    resume.addEventListener('click', () => run('resume_session', { id: s.id }));
    buttons.append(resume);
  }

  el.append(tag, head, when, detail, buttons);
  return el;
}

function render() {
  const visible = sessions
    .filter((s) => showIdle || s.state !== 'idle')
    .sort((a, b) => ORDER[a.state] - ORDER[b.state] || b.updatedMs - a.updatedMs);

  const count = (st) => sessions.filter((s) => s.state === st).length;
  const summary = $('summary');
  summary.replaceChildren();
  for (const [st, label] of [['input', 'need you'], ['working', 'working'], ['done', 'done']]) {
    const span = document.createElement('span');
    span.className = st;
    span.innerHTML = '<b></b> ';
    span.querySelector('b').textContent = count(st);
    span.append(label);
    summary.append(span);
  }

  const list = $('list');
  if (!visible.length) {
    list.replaceChildren();
    const empty = document.createElement('div');
    empty.className = 'empty';
    empty.innerHTML = sessions.length
      ? 'Only idle sessions right now. Tick <code>show idle</code> to see them.'
      : 'No Claude Code or Codex sessions in the last 2 hours.<br/>Run <code>claude</code> in a terminal and it shows up here.';
    list.append(empty);
    return;
  }
  for (const id of cards.keys()) if (!sessions.some((s) => s.id === id)) cards.delete(id);
  const els = visible.map(cardFor);
  // Same cards in the same order: leave the DOM alone so clicks aren't lost.
  const same = els.length === list.children.length && els.every((el, i) => list.children[i] === el);
  if (!same) list.replaceChildren(...els);
}

async function refreshHook() {
  try {
    const st = await invoke('claude_hook_status');
    $('hookHint').hidden = st.installed && !st.stale;
    $('enableHook').textContent = st.stale ? 'Repair precise alerts' : 'Enable precise alerts';
  } catch {
    $('hookHint').hidden = true;
  }
}

$('showIdle').addEventListener('change', (e) => {
  showIdle = e.target.checked;
  render();
});
$('openLatest').addEventListener('click', () => run('open_terminal', {}));
$('enableHook').addEventListener('click', async () => {
  if (!confirm('Add a Notification hook to ~/.claude/settings.json so Bomboclat sees permission prompts?\n\nA backup is saved next to it, and you can remove it any time in Settings.')) return;
  try {
    await invoke('set_claude_hook', { on: true });
    toast('Precise alerts on. New Claude sessions will report permission prompts.');
  } catch (e) {
    toast(String(e));
  }
  refreshHook();
});

async function init() {
  sessions = await invoke('list_sessions');
  render();
  await listen('sessions', ({ payload }) => {
    sessions = payload;
    render();
  });
  await refreshHook();
  // Spinner and "x ago" labels change in place, so buttons never get swapped
  // out from under a click.
  setInterval(() => {
    frame++;
    for (const s of sessions) {
      if (s.state !== 'working' || s.maybeWaiting) continue;
      const tag = document.querySelector(`.session[data-id="${CSS.escape(s.id)}"] .tag`);
      if (tag) tag.textContent = tagText(s);
    }
  }, 120);
  setInterval(() => {
    for (const s of sessions) {
      const card = document.querySelector(`.session[data-id="${CSS.escape(s.id)}"]`);
      if (!card) continue;
      card.querySelector('.when').textContent = ago(s.updatedMs);
      card.querySelector('.detail').textContent = detailText(s);
    }
  }, 5_000);
}

init();
