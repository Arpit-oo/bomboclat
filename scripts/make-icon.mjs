// Renders the pixel cat into a 512x512 PNG used as the source for `tauri icon`.
// Usage: node scripts/make-icon.mjs <out.png>   (needs `sharp` resolvable)
import { createRequire } from 'module';
import { drawCatArt, CAT_PX_W, CAT_PX_H } from '../src/cat-art.js';

const require = createRequire(import.meta.url);
const sharp = require(process.env.SHARP_PATH || 'sharp');

const buf = Buffer.alloc(CAT_PX_W * CAT_PX_H * 4);
const hex = (c) => {
  let h = c.slice(1);
  if (h.length === 3) h = [...h].map((ch) => ch + ch).join('');
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
};
const set = (x, y, c) => {
  x = Math.round(x); y = Math.round(y);
  if (x < 0 || y < 0 || x >= CAT_PX_W || y >= CAT_PX_H) return;
  const [r, g, b] = hex(c);
  buf.set([r, g, b, 255], (y * CAT_PX_W + x) * 4);
};
const p = {
  dot: set,
  row(x1, x2, y, c) { for (let x = x1; x <= x2; x++) set(x, y, c); },
  rect(x, y, w, h, c) { for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) set(i, j, c); },
};
drawCatArt(p, { state: 'IDLE', eyeOffsetX: 0, eyeOffsetY: 0, time: 0 });

// Crop to the art's vertical extent (rows 2..32) and pad to a square.
await sharp(buf, { raw: { width: CAT_PX_W, height: CAT_PX_H, channels: 4 } })
  .extract({ left: 3, top: 1, width: 34, height: 34 })
  .resize(512, 512, { kernel: 'nearest' })
  .png()
  .toFile(process.argv[2] || 'app-icon.png');
