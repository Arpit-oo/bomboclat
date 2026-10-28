# Bomboclat

Open-source pixel bongo cat desktop companion for Windows, built with Tauri 2.

A tiny black pixel cat sits on your desktop, follows your cursor with its eyes and reacts to
every key and mouse button you press. Inspired by [BongoCat](https://github.com/ayangweb/BongoCat).

## Use

- **Drag** the cat anywhere. It remembers where you left it.
- **Tray icon**: left-click shows/hides the cat; right-click for size, opacity, flip,
  click-through, always-on-top, launch at startup, settings and quit.
- **Ctrl+Shift+Alt+C** shows/hides the cat from anywhere.
- **Settings** (tray, or `bomboclat --settings`): everything above plus fade-on-hover,
  sleep timer and the pressed-key bubble. The key bubble is off by default because it
  would show passwords during a screen share.

Settings live in `%APPDATA%\com.arpit.bomboclat\settings.json`.

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
- [x] Phase 2: Desk + slamming paws per key side, pressed-key bubble, sleep/wake
- [x] Phase 3: Tray, settings window, persistence, global shortcut, autostart
- [ ] Phase 4: Claude Code agent watcher, reminders, speech bubbles
- [ ] Phase 5: Packaging, CI, releases
