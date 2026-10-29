// Pixel cat art. Drawing goes through a plotter so the same art can render
// to a canvas (renderer) or a raw RGBA buffer (icon generation).

export const CAT_PX_W = 40;
export const CAT_PX_H = 40;

/** @param {{dot(x:number,y:number,c:string):void,row(x1:number,x2:number,y:number,c:string):void,rect(x:number,y:number,w:number,h:number,c:string):void}} p */
export function drawCatArt(p, state) {

  const cx = 20;
  const black = '#111111';
  const dark = '#1a1a1a';
  const white = '#ffffff';
  const offwhite = '#dddddd';
  const pupil = '#111111';
  const whisker = '#555555';
  const red = '#cc3333';
  const gold = '#ddaa33';
  const bellYellow = '#ffcc44';
  const bellDark = '#cc9922';

  const eox = Math.round(Math.max(-1, Math.min(1, (state.eyeOffsetX || 0))));
  const eoy = Math.round(Math.max(-1, Math.min(1, (state.eyeOffsetY || 0))));
  const isSleeping = state.state === 'SLEEP' || state.blink;

  // --- Tail (wraps right, curls up) ---
  const tb = Math.round(Math.sin((state.time ?? Date.now()) / 500));
  p.dot(cx + 10, 28, black);
  p.dot(cx + 11, 27, black);
  p.dot(cx + 12, 26 + tb, black);
  p.dot(cx + 13, 25 + tb, black);
  p.dot(cx + 13, 24 + tb, black);
  p.dot(cx + 12, 23 + tb, black);
  p.dot(cx + 11, 22 + tb, black);
  // Tail tip highlight
  p.dot(cx + 11, 22 + tb, dark);

  // --- Body (big chubby blob) ---
  p.row(cx - 7, cx + 7, 18, black);
  p.row(cx - 8, cx + 8, 19, black);
  p.row(cx - 9, cx + 9, 20, black);
  p.row(cx - 10, cx + 10, 21, black);
  p.row(cx - 10, cx + 10, 22, black);
  p.row(cx - 10, cx + 10, 23, black);
  p.row(cx - 10, cx + 10, 24, black);
  p.row(cx - 10, cx + 10, 25, black);
  p.row(cx - 10, cx + 10, 26, black);
  p.row(cx - 9, cx + 9, 27, black);
  p.row(cx - 9, cx + 9, 28, black);
  p.row(cx - 8, cx + 8, 29, black);
  p.row(cx - 7, cx + 7, 30, black);

  // Chest highlight (white patch)
  p.row(cx - 3, cx + 3, 24, dark);
  p.row(cx - 2, cx + 2, 25, dark);

  // --- Paws ---
  p.row(cx - 8, cx - 4, 31, black);
  p.row(cx + 4, cx + 8, 31, black);
  p.row(cx - 7, cx - 5, 32, black);
  p.row(cx + 5, cx + 7, 32, black);
  // Paw toe beans (white detail)
  p.dot(cx - 6, 32, offwhite);
  p.dot(cx + 6, 32, offwhite);

  // --- Head (big round) ---
  p.row(cx - 6, cx + 6, 7, black);
  p.row(cx - 8, cx + 8, 8, black);
  p.row(cx - 9, cx + 9, 9, black);
  p.row(cx - 10, cx + 10, 10, black);
  p.row(cx - 10, cx + 10, 11, black);
  p.row(cx - 10, cx + 10, 12, black);
  p.row(cx - 10, cx + 10, 13, black);
  p.row(cx - 10, cx + 10, 14, black);
  p.row(cx - 9, cx + 9, 15, black);
  p.row(cx - 8, cx + 8, 16, black);
  p.row(cx - 7, cx + 7, 17, black);

  // --- Ears (filled triangles with pink inner) ---
  const earPink = '#c0607a';
  for (const s of [-1, 1]) {
    const ex = (o) => cx + s * o;
    p.dot(ex(10), 2, black);
    p.row(Math.min(ex(11), ex(9)), Math.max(ex(11), ex(9)), 3, black);
    p.row(Math.min(ex(11), ex(8)), Math.max(ex(11), ex(8)), 4, black);
    p.row(Math.min(ex(11), ex(7)), Math.max(ex(11), ex(7)), 5, black);
    p.row(Math.min(ex(10), ex(6)), Math.max(ex(10), ex(6)), 6, black);
    p.row(Math.min(ex(9), ex(5)), Math.max(ex(9), ex(5)), 7, black);
    p.row(Math.min(ex(10), ex(9)), Math.max(ex(10), ex(9)), 4, earPink);
    p.row(Math.min(ex(10), ex(8)), Math.max(ex(10), ex(8)), 5, earPink);
    p.row(Math.min(ex(9), ex(8)), Math.max(ex(9), ex(8)), 6, earPink);
  }

  // --- Eyes (BIGGER — 4x4 whites) ---
  if (isSleeping) {
    p.row(cx - 7, cx - 3, 12, white);
    p.row(cx + 3, cx + 7, 12, white);
  } else {
    // Left eye white (4x4)
    p.rect(cx - 7, 10, 4, 4, white);
    // Right eye white (4x4)
    p.rect(cx + 3, 10, 4, 4, white);

    // Pupils (2x2, follow cursor)
    p.rect(cx - 6 + eox, 11 + eoy, 2, 2, pupil);
    p.rect(cx + 4 + eox, 11 + eoy, 2, 2, pupil);

    // Highlights (1px)
    p.dot(cx - 6 + eox, 10 + eoy, white);
    p.dot(cx + 4 + eox, 10 + eoy, white);
  }

  // --- Nose (small white/pink triangle) ---
  p.dot(cx, 14, '#6b4050');
  p.dot(cx - 1, 14, '#6b4050');

  // --- Mouth ---
  p.dot(cx - 1, 15, dark);
  p.dot(cx, 15, dark);
  p.dot(cx + 1, 15, dark);
  p.dot(cx - 2, 15, '#333');
  p.dot(cx + 2, 15, '#333');

  // --- Whiskers (extend FROM face edge outward, 3 per side) ---
  // Left whiskers — start at face edge (x-10), go outward
  p.dot(cx - 11, 11, whisker); p.dot(cx - 12, 11, whisker); p.dot(cx - 13, 10, whisker); p.dot(cx - 14, 10, whisker); p.dot(cx - 15, 9, whisker);
  p.dot(cx - 11, 12, whisker); p.dot(cx - 12, 12, whisker); p.dot(cx - 13, 12, whisker); p.dot(cx - 14, 12, whisker); p.dot(cx - 15, 12, whisker);
  p.dot(cx - 11, 13, whisker); p.dot(cx - 12, 13, whisker); p.dot(cx - 13, 14, whisker); p.dot(cx - 14, 14, whisker); p.dot(cx - 15, 15, whisker);
  // Right whiskers
  p.dot(cx + 11, 11, whisker); p.dot(cx + 12, 11, whisker); p.dot(cx + 13, 10, whisker); p.dot(cx + 14, 10, whisker); p.dot(cx + 15, 9, whisker);
  p.dot(cx + 11, 12, whisker); p.dot(cx + 12, 12, whisker); p.dot(cx + 13, 12, whisker); p.dot(cx + 14, 12, whisker); p.dot(cx + 15, 12, whisker);
  p.dot(cx + 11, 13, whisker); p.dot(cx + 12, 13, whisker); p.dot(cx + 13, 14, whisker); p.dot(cx + 14, 14, whisker); p.dot(cx + 15, 15, whisker);

  // --- Red collar ---
  p.row(cx - 7, cx + 7, 17, red);
  p.row(cx - 6, cx + 6, 18, red);

  // --- Bell on collar ---
  p.dot(cx, 19, bellYellow);
  p.dot(cx - 1, 19, bellYellow);
  p.dot(cx + 1, 19, bellYellow);
  p.dot(cx, 20, bellYellow);
  p.dot(cx - 1, 20, bellDark);
  p.dot(cx + 1, 20, bellDark);
  p.dot(cx, 21, bellDark);
  // Bell highlight
  p.dot(cx, 19, gold);

  // --- White detail: chest tuft ---
  p.dot(cx, 22, '#222');
  p.dot(cx - 1, 22, '#222');
  p.dot(cx + 1, 22, '#222');
  p.dot(cx, 23, '#222');
}

const DESK_TOP = '#d9a66b';
const DESK = '#c8935a';
const DESK_EDGE = '#7a5230';
const DESK_FRONT = '#a0703f';
const KB_FRAME = '#454c5a';
const KB_KEY = '#6a7080';
const KB_LIT = '#ffcc44';
const PAD_PINK = '#e890a8';

/** Desk across the bottom with a keyboard the cat sits behind. */
export function drawDesk(p, state) {
  const cx = 20;
  p.row(0, CAT_PX_W - 1, 30, DESK_TOP);
  p.row(0, CAT_PX_W - 1, 31, DESK);
  p.row(0, CAT_PX_W - 1, 32, DESK_EDGE);
  p.rect(0, 33, CAT_PX_W, 6, DESK_FRONT);

  p.row(cx - 12, cx + 12, 28, KB_FRAME);
  p.row(cx - 12, cx + 12, 29, KB_FRAME);
  p.row(cx - 12, cx + 12, 30, KB_FRAME);
  for (let x = cx - 11; x <= cx + 11; x += 2) {
    const lit = (state.leftPaw === 'down' && x < cx) || (state.rightPaw === 'down' && x > cx);
    p.dot(x, 29, lit ? KB_LIT : KB_KEY);
  }
}

/** One paw. side: -1 left, 1 right. Up paws show pink pads; down paws rest on the keys. */
function drawPaw(p, side, down) {
  const cx = 20;
  const span = (a, b, y, c) => {
    const x1 = cx + side * a;
    const x2 = cx + side * b;
    p.row(Math.min(x1, x2), Math.max(x1, x2), y, c);
  };
  const black = '#111111';
  if (down) {
    span(9, 5, 27, black);
    span(10, 4, 28, black);
    span(10, 4, 29, black);
    span(9, 5, 30, black);
    p.dot(cx + side * 9, 30, PAD_PINK);
    p.dot(cx + side * 7, 30, PAD_PINK);
    p.dot(cx + side * 5, 30, PAD_PINK);
  } else {
    span(11, 10, 22, black);
    span(12, 9, 23, black);
    span(12, 9, 24, black);
    span(12, 9, 25, black);
    span(11, 9, 26, black);
    p.dot(cx + side * 12, 23, PAD_PINK);
    p.dot(cx + side * 10, 23, PAD_PINK);
    span(11, 10, 25, PAD_PINK);
  }
}

/** Full bongo scene: cat behind a desk, paws following key presses. */
export function drawScene(p, state) {
  drawCatArt(p, state);
  drawDesk(p, state);
  drawPaw(p, -1, state.leftPaw === 'down');
  drawPaw(p, 1, state.rightPaw === 'down');
}
