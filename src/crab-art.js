// Pixel Claude crab: a blocky orange crab whose claws bongo on the keyboard.
// Same 40x40 canvas, plotter and desk as the cat (see cat-art.js).

import { drawDesk } from './cat-art.js';

const SHELL = '#d97757';
const SHELL_LIT = '#e8946f';
const SHELL_DARK = '#b45f40';
const CLAW = '#c8653f';
const CLAW_EDGE = '#8a3f28';
const EYE = '#111111';
const BLUSH = '#f0a08a';

// Eye row and top of the shell in art pixels, for cursor tracking and bubbles.
export const CRAB_EYE_ROW = 16;
export const CRAB_HEAD_TOP_ROW = 10;

function drawShell(p, state) {
  const cx = 20;
  const eox = Math.round(Math.max(-1, Math.min(1, state.eyeOffsetX || 0)));
  const eoy = Math.round(Math.max(-1, Math.min(1, state.eyeOffsetY || 0)));
  const closed = state.state === 'SLEEP' || state.blink;

  // Little legs either side of the keyboard; the desk hides the rest.
  for (const s of [-1, 1]) {
    p.dot(cx + s * 12, 26, SHELL_DARK);
    p.dot(cx + s * 13, 27, SHELL_DARK);
    p.dot(cx + s * 13, 28, SHELL_DARK);
    p.dot(cx + s * 14, 29, SHELL_DARK);
  }

  // Blocky shell with a lit top edge and shaded sides.
  p.row(cx - 9, cx + 9, 10, SHELL_LIT);
  p.rect(cx - 10, 11, 21, 17, SHELL);
  p.row(cx - 9, cx + 9, 11, SHELL_LIT);
  p.rect(cx - 10, 11, 1, 17, SHELL_DARK);
  p.rect(cx + 10, 11, 1, 17, SHELL_DARK);
  p.row(cx - 9, cx + 9, 27, SHELL_DARK);

  // Eyes: tall blocks that slide toward the cursor, a line when closed.
  for (const ex of [cx - 6, cx + 5]) {
    if (closed) p.row(ex, ex + 1, 17, EYE);
    else p.rect(ex + eox, 15 + eoy, 2, 3, EYE);
  }

  // Cheeks.
  p.dot(cx - 8, 19, BLUSH);
  p.dot(cx + 8, 19, BLUSH);
}

/** One claw. side: -1 left, 1 right. Up claws wave at the side; down claws tap the keys. */
function drawClaw(p, side, down) {
  const cx = 20;
  const span = (a, b, y, c) => {
    const x1 = cx + side * a;
    const x2 = cx + side * b;
    p.row(Math.min(x1, x2), Math.max(x1, x2), y, c);
  };
  if (down) {
    // Pincer resting on the keyboard, open toward the keys.
    // Dark outline so it reads against the shell behind it.
    span(10, 9, 24, CLAW_EDGE);
    span(6, 5, 24, CLAW_EDGE);
    for (let y = 25; y <= 28; y++) span(10, 5, y, CLAW_EDGE);
    span(9, 9, 25, SHELL_LIT);
    span(6, 6, 25, SHELL_LIT);
    span(9, 6, 26, SHELL_LIT);
    span(9, 6, 27, SHELL_LIT);
    span(10, 5, 29, CLAW_EDGE);
  } else {
    // Arm out of the shell, pincer raised.
    span(11, 12, 21, CLAW_EDGE);
    span(11, 12, 20, CLAW);
    span(12, 15, 19, CLAW_EDGE);
    span(12, 15, 18, CLAW);
    span(12, 15, 17, CLAW);
    span(12, 13, 16, CLAW);
    span(15, 15, 16, CLAW);
    span(12, 12, 15, CLAW);
    span(15, 15, 15, CLAW);
  }
}

/** Full bongo scene: crab behind the desk, claws following key presses. */
export function drawCrabScene(p, state) {
  drawShell(p, state);
  drawDesk(p, state);
  drawClaw(p, -1, state.leftPaw === 'down');
  drawClaw(p, 1, state.rightPaw === 'down');
}
