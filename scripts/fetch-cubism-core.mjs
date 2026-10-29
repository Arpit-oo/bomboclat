// Downloads Live2D Cubism Core for Web into public/. The core is proprietary
// (Live2D Proprietary Software License), so it is fetched locally rather than
// committed. Only needed for Live2D models; the pixel cat works without it.
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const URL = 'https://cubism.live2d.com/sdk-web/cubismcore/live2dcubismcore.min.js';
const out = join(dirname(fileURLToPath(import.meta.url)), '..', 'public', 'live2dcubismcore.min.js');

if (existsSync(out) && !process.argv.includes('--force')) {
  console.log('Cubism Core already present.');
  process.exit(0);
}
const res = await fetch(URL);
if (!res.ok) {
  console.error(`Cubism Core download failed: HTTP ${res.status}`);
  process.exit(1);
}
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, Buffer.from(await res.arrayBuffer()));
console.log(`Cubism Core saved to ${out}`);
console.log('By using it you accept the Live2D Proprietary Software License: https://www.live2d.com/eula/live2d-proprietary-software-license-agreement_en.html');
