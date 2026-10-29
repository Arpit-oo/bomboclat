// Speech bubbles shared by both renderers: one-off messages (reminders, agent
// done), a pinned note, a "..." thought while an agent works, and the pomodoro
// countdown. Pure DOM on top of the scene, so it works for pixel and Live2D.
import { listen } from '@tauri-apps/api/event';
import './bubble.css';

const MESSAGE_MS = 6000;
const MAX_QUEUE = 4;
const AGENT_NAMES = { claude: 'Claude', codex: 'Codex' };

function el(cls, text = '') {
  const d = document.createElement('div');
  d.className = cls;
  d.textContent = text;
  return d;
}

let audio = null;
/** Two soft notes; no asset needed. */
function chime() {
  try {
    audio ??= new AudioContext();
    const t = audio.currentTime;
    for (const [i, freq] of [880, 1318.5].entries()) {
      const osc = audio.createOscillator();
      const gain = audio.createGain();
      osc.type = 'sine';
      osc.frequency.value = freq;
      const start = t + i * 0.12;
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.12, start + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.001, start + 0.35);
      osc.connect(gain).connect(audio.destination);
      osc.start(start);
      osc.stop(start + 0.4);
    }
  } catch {
    // Audio is a nicety; never let it break the cat.
  }
}

export function agentDoneText(e, name) {
  const who = AGENT_NAMES[e.agent] ?? 'Your agent';
  const where = e.project ? ` in ${e.project}` : '';
  return `${name ? `${name}, ` : ''}${who} finished${where}!`;
}

/**
 * @param {(kind: 'done'|'message'|'working', detail?: any) => void} onReact
 *        called so the renderer can animate (hop, wake up, play a motion).
 */
export async function createBubbles(initialSettings, onReact = () => {}) {
  let settings = initialSettings;
  const root = el('bubbles');
  const msg = el('bubble msg');
  const pinned = el('bubble pinned');
  const think = el('think');
  think.append(el('dot'), el('dot'), el('dot'));
  const pomo = el('pomo');
  root.append(pinned, msg, think, pomo);
  document.body.appendChild(root);

  const queue = [];
  let current = null;
  let hideTimer = 0;
  let working = 0;

  function next() {
    current = queue.shift() ?? null;
    msg.classList.toggle('on', !!current);
    if (current) {
      msg.textContent = current;
      clearTimeout(hideTimer);
      hideTimer = setTimeout(next, MESSAGE_MS);
    }
    refresh();
  }

  function say(text, sound = true) {
    if (queue.length >= MAX_QUEUE) queue.shift();
    queue.push(text);
    if (sound && settings.sounds) chime();
    onReact('message');
    if (!current) next();
  }

  function refresh() {
    const note = settings.pinnedMessage?.trim();
    pinned.textContent = note ?? '';
    pinned.classList.toggle('on', !!note && !current);
    think.classList.toggle('on', working > 0 && !current);
  }

  // Click a message to dismiss it early.
  msg.addEventListener('mousedown', (e) => {
    e.stopPropagation();
    next();
  });

  await listen('bubble', ({ payload }) => say(payload.text));
  await listen('agent-done', ({ payload }) => {
    onReact('done', payload);
    if (settings.agentBubbles) say(agentDoneText(payload, settings.userName?.trim()));
  });
  await listen('agents', ({ payload }) => {
    working = settings.agentWatch ? payload.working : 0;
    if (working > 0) onReact('working');
    refresh();
  });
  await listen('pomodoro', ({ payload }) => {
    if (!payload.phase) {
      pomo.classList.remove('on');
      return;
    }
    const m = Math.floor(payload.remainingSecs / 60);
    const s = String(payload.remainingSecs % 60).padStart(2, '0');
    pomo.textContent = `${payload.phase === 'focus' ? 'FOCUS' : 'BREAK'} ${m}:${s}`;
    pomo.dataset.phase = payload.phase;
    pomo.classList.add('on');
  });

  // Show the pinned note straight away, not only after the first event.
  refresh();

  return {
    applySettings(next) {
      settings = next;
      if (!settings.agentWatch) working = 0;
      refresh();
    },
    /** True while a message bubble is up (renderers hide their own text then). */
    get speaking() {
      return !!current;
    },
    get thinking() {
      return working > 0;
    },
  };
}
