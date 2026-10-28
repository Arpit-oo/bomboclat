// Maps rdev key names (Debug format of rdev::Key) to keyboard side and display labels.

const LEFT_KEYS = new Set([
  'Escape', 'BackQuote', 'Num1', 'Num2', 'Num3', 'Num4', 'Num5',
  'Tab', 'KeyQ', 'KeyW', 'KeyE', 'KeyR', 'KeyT',
  'CapsLock', 'KeyA', 'KeyS', 'KeyD', 'KeyF', 'KeyG',
  'ShiftLeft', 'KeyZ', 'KeyX', 'KeyC', 'KeyV', 'KeyB',
  'ControlLeft', 'MetaLeft', 'Alt',
  'F1', 'F2', 'F3', 'F4', 'F5', 'F6',
]);

const MODIFIERS = {
  ControlLeft: 'Ctrl', ControlRight: 'Ctrl',
  ShiftLeft: 'Shift', ShiftRight: 'Shift',
  Alt: 'Alt', AltGr: 'AltGr',
  MetaLeft: 'Win', MetaRight: 'Win',
};

const LABELS = {
  Return: 'Enter', Space: 'Space', Backspace: 'Bksp', Tab: 'Tab', Escape: 'Esc',
  CapsLock: 'Caps', Delete: 'Del', Insert: 'Ins', Home: 'Home', End: 'End',
  PageUp: 'PgUp', PageDown: 'PgDn', UpArrow: '↑', DownArrow: '↓',
  LeftArrow: '←', RightArrow: '→', BackQuote: '`', Minus: '-', Equal: '=',
  LeftBracket: '[', RightBracket: ']', BackSlash: '\\', IntlBackslash: '\\',
  SemiColon: ';', Quote: "'", Comma: ',', Dot: '.', Slash: '/',
  PrintScreen: 'PrtSc', ScrollLock: 'ScrLk', Pause: 'Pause', NumLock: 'NumLk',
  KpReturn: 'Enter', KpMinus: '-', KpPlus: '+', KpMultiply: '*', KpDivide: '/', KpDelete: '.',
};

/** 'left' | 'right' | 'both' (space bar alternates in the caller). */
export function keySide(key) {
  if (key === 'Space') return 'both';
  return LEFT_KEYS.has(key) ? 'left' : 'right';
}

export function isModifier(key) {
  return key in MODIFIERS;
}

export function keyLabel(key) {
  if (MODIFIERS[key]) return MODIFIERS[key];
  if (LABELS[key]) return LABELS[key];
  let m = key.match(/^Key([A-Z])$/);
  if (m) return m[1];
  m = key.match(/^(?:Num|Kp)(\d)$/);
  if (m) return m[1];
  if (/^F\d+$/.test(key)) return key;
  if (key.startsWith('Unknown')) return '?';
  return key;
}

/** Combo label from held modifiers plus the key just pressed, e.g. "Ctrl + Shift + S". */
export function comboLabel(heldKeys, key) {
  const order = ['Ctrl', 'Shift', 'Alt', 'AltGr', 'Win'];
  const mods = new Set();
  for (const k of heldKeys) if (MODIFIERS[k]) mods.add(MODIFIERS[k]);
  if (isModifier(key)) return order.filter((m) => mods.has(m)).join(' + ') || keyLabel(key);
  const parts = order.filter((m) => mods.has(m));
  parts.push(keyLabel(key));
  return parts.join(' + ');
}

// rdev names that differ from the key image names BongoCat models ship.
const MODEL_KEY_NAMES = {
  Alt: 'AltLeft', AltGr: 'AltRight', Return: 'Enter', KpReturn: 'Enter',
  MetaLeft: 'Meta', MetaRight: 'Meta', Function: 'Fn',
};

/** Key image name (file stem in a model's left-keys/right-keys) for an rdev key. */
export function modelKeyName(key) {
  if (MODEL_KEY_NAMES[key]) return MODEL_KEY_NAMES[key];
  const kp = key.match(/^Kp(\d)$/);
  return kp ? `Num${kp[1]}` : key;
}
