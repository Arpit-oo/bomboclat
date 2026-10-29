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

## AI agents, reminders, focus timer

- **Agent watcher**: while Claude Code or Codex works in any terminal, a little `...` thought
  bubble shows; when a turn finishes the cat hops (Live2D models play their motion) and says
  e.g. *"Arpit, Claude finished in bomboclat!"*. It only reads the session logs in
  `~/.claude/projects` and `~/.codex/sessions`; nothing is sent anywhere or launched.
- **Reminders**: water every 30 minutes and a stretch every hour by default (both adjustable or off),
  paused while you are away from the PC, plus daily reminders at a set time.
- **Focus timer**: pomodoro (25/5 by default) from the tray or Settings, with a countdown on the cat.
- **Pinned note** that always sits above the cat, your name in messages, optional chime.

## Live2D models (BongoCat style)

Besides the pixel cat, Bomboclat runs BongoCat-format Live2D models: a background,
the Live2D model, and one overlay image per key in `resources/left-keys` /
`resources/right-keys`. Pressing a key slams the paw on the side its image lives in;
keys with no image don't react. The mouse paw and head follow your cursor.

- **BongoCat models**: if [BongoCat](https://github.com/ayangweb/BongoCat) is installed,
  its Standard / Keyboard / Gamepad models show up in Settings automatically. They are
  read from BongoCat's install folder, never copied. Gamepad input is not supported yet.
- **Your own models**: Settings → Character → *Import Live2D model…* and pick the folder.
  Any Cubism 3/4 model works; the BongoCat folder layout adds the desk and key overlays.
  Imported models are copied to `%APPDATA%\com.arpit.bomboclat\models`.

Live2D needs Live2D Cubism Core, which is proprietary and not in this repo.
`run.bat` (or `npm run setup:live2d`) downloads it into `public/`; by using it you accept the
[Live2D Proprietary Software License](https://www.live2d.com/eula/live2d-proprietary-software-license-agreement_en.html).

## Develop

Requires Node 20+, Rust stable and the WebView2 runtime.

Double-click **`run.bat`**: it installs dependencies and Cubism Core on first run, then starts
the app. Or by hand:

```sh
npm install
npm run setup:live2d
npm run tauri dev
```

## Build & release

- **Local installer**: double-click **`build.bat`** (or `npm run tauri build`). Output:
  `src-tauri	argeteleaseundle
sis\Bomboclat_<version>_x64-setup.exe`. Installs per user,
  no admin prompt.
- **Release**: set the version, commit, tag and push. GitHub Actions builds the installer and
  opens a **draft** release with it attached; review and publish it on GitHub.

  ```sh
  npm run version:set -- 0.2.0
  git commit -am "release: v0.2.0"
  git tag v0.2.0
  git push && git push --tags
  ```

- **CI** runs on every PR and push to `master`: frontend build, `cargo clippy -D warnings`, `cargo test`.

The installer isn't code-signed, so SmartScreen warns on first run (*More info → Run anyway*).
Release builds include Live2D Cubism Core, fetched during the build; distributing it means
accepting Live2D's license terms (free for individuals and small businesses).

Regenerate the app icon from the pixel art:

```sh
node scripts/make-icon.mjs app-icon.png   # needs `sharp`
npx tauri icon app-icon.png
```

## Roadmap

- [x] Phase 1: Tauri scaffold, transparent draggable window, pixel cat, global input hook, eye tracking
- [x] Phase 2: Desk + slamming paws per key side, pressed-key bubble, sleep/wake
- [x] Phase 3: Tray, settings window, persistence, global shortcut, autostart
- [x] Phase 4: BongoCat-style Live2D models (BongoCat's own + imported), `run.bat`
- [x] Phase 5: Claude Code / Codex watcher, reminders, focus timer, speech bubbles
- [x] Phase 6: Installer metadata, `build.bat`, version script, CI + tagged draft releases
