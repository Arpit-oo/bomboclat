// Sets the app version everywhere it lives: package.json, tauri.conf.json and
// Cargo.toml (Cargo.lock follows on the next cargo build).
//   node scripts/bump-version.mjs 0.2.0
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version ?? '')) {
  console.error('Usage: node scripts/bump-version.mjs <x.y.z>');
  process.exit(1);
}
const root = join(dirname(fileURLToPath(import.meta.url)), '..');

function edit(rel, pattern) {
  const path = join(root, rel);
  const text = readFileSync(path, 'utf8');
  if (!pattern.test(text)) throw new Error(`No version found in ${rel}`);
  writeFileSync(path, text.replace(pattern, (_, pre, post) => `${pre}${version}${post}`));
  console.log(`${rel} -> ${version}`);
}

const jsonVersion = /("version"\s*:\s*")[^"]+(")/;
edit('package.json', jsonVersion);
edit('src-tauri/tauri.conf.json', jsonVersion);
// Only the [package] version, which is the first `version =` line.
edit('src-tauri/Cargo.toml', /^(version = ")[^"]+(")/m);
