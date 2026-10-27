# Bomboclat

Open-source pixel bongo cat desktop companion for Windows, built with Tauri 2.

A tiny black pixel cat sits on your desktop, follows your cursor with its eyes and reacts to
every key and mouse button you press. Inspired by [BongoCat](https://github.com/ayangweb/BongoCat).

## Develop

Requires Node 20+, Rust stable and the WebView2 runtime.

```sh
npm install
npm run tauri dev
```

Build an installer with `npm run tauri build`.

Regenerate the app icon from the pixel art:

```sh
node scripts/make-icon.mjs app-icon.png   # needs `sharp`
npx tauri icon app-icon.png
```

## Roadmap

- [x] Phase 1: Tauri scaffold, transparent draggable window, pixel cat, global input hook, eye tracking
- [ ] Phase 2: Desk + slamming paws per key side, pressed-key bubble, sleep/wake
- [ ] Phase 3: Tray, settings window, persistence, global shortcut, autostart
- [ ] Phase 4: Claude Code agent watcher, reminders, speech bubbles
- [ ] Phase 5: Packaging, CI, releases
