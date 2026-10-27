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
  const isSleeping = state.state === 'SLEEP';

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

  // --- Ears (bigger, well-defined triangles) ---
  // Left ear outer
  p.dot(cx - 8, 7, black);
  p.dot(cx - 9, 6, black);
  p.dot(cx - 8, 6, black);
  p.dot(cx - 10, 5, black);
  p.dot(cx - 9, 5, black);
  p.dot(cx - 8, 5, black);
  p.dot(cx - 11, 4, black);
  p.dot(cx - 10, 4, black);
  p.dot(cx - 9, 4, black);
  p.dot(cx - 12, 3, black);
  p.dot(cx - 11, 3, black);
  p.dot(cx - 10, 3, black);
  p.dot(cx - 12, 2, black);
  p.dot(cx - 11, 2, black);
  // Left ear inner pink
  p.dot(cx - 10, 4, '#4a2a3a');
  p.dot(cx - 9, 4, '#4a2a3a');
  p.dot(cx - 11, 3, '#4a2a3a');
  p.dot(cx - 10, 3, '#4a2a3a');

  // Right ear outer
  p.dot(cx + 8, 7, black);
  p.dot(cx + 9, 6, black);
  p.dot(cx + 8, 6, black);
  p.dot(cx + 10, 5, black);
  p.dot(cx + 9, 5, black);
  p.dot(cx + 8, 5, black);
  p.dot(cx + 11, 4, black);
  p.dot(cx + 10, 4, black);
  p.dot(cx + 9, 4, black);
  p.dot(cx + 12, 3, black);
  p.dot(cx + 11, 3, black);
  p.dot(cx + 10, 3, black);
  p.dot(cx + 12, 2, black);
  p.dot(cx + 11, 2, black);
  // Right ear inner pink
  p.dot(cx + 10, 4, '#4a2a3a');
  p.dot(cx + 9, 4, '#4a2a3a');
  p.dot(cx + 11, 3, '#4a2a3a');
  p.dot(cx + 10, 3, '#4a2a3a');

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
