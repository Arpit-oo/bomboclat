# Architecture

Bomboclat is a Tauri 2 app: Rust owns state, system integration and file watching, and three
small vanilla-JS pages render the UI. There's no framework and no state library: Rust emits
events and the pages redraw.

```
┌──────────────────────────── Rust (src-tauri/src) ────────────────────────────┐
│ input.rs      rdev global hook ─────────────► "input", "cursor"              │
│ agents.rs     tails Claude/Codex logs ──────► "sessions", "agents",          │
│   └ hooks.rs  + Notification hook file        "agent-input", "agent-done"    │
│ reminders.rs  water/stretch/daily/pomodoro ─► "bubble", "pomodoro"           │
│ settings.rs   settings.json, window apply ──► "settings-changed"             │
│ models.rs     model catalogue + bcmodel:// protocol                          │
│ tray.rs       tray + right-click menu, window openers                        │
│ terminal.rs   open terminal / resume session                                 │
└──────────────────────────────────────────────────────────────────────────────┘
          │ events                                   ▲ invoke(commands)
          ▼                                          │
┌─ index.html (cat) ─────────┐ ┌─ settings.html ─┐ ┌─ sessions.html ─┐
│ main.js   picks renderer   │ │ settings.js     │ │ sessions.js     │
│ pixel.js  canvas pixel cat │ └─────────────────┘ └─────────────────┘
│ live2d.js pixi + Live2D    │
│ bubble.js shared bubbles   │
└────────────────────────────┘
```

## Rust modules

| File | Responsibility |
|---|---|
| `lib.rs` | Builder: plugins (single-instance, window-state, autostart, global-shortcut, dialog), `bcmodel` protocol, commands, background threads. `--settings` / `--sessions` / `--hook-event` handling. |
| `input.rs` | `rdev::listen` on its own thread. Key/button events → `input`; mouse moves throttled to ~60 Hz → `cursor`. Tracks `LAST_INPUT_MS` for away detection. |
| `settings.rs` | `Settings` struct (serde, camelCase, defaults), load/save, `apply()` for window-level settings (click-through, on-top, size with bottom-centre anchoring and screen clamping), `update()` merges a JSON patch, saves, applies, syncs the tray and broadcasts. |
| `agents.rs` | Session watcher, polled every 700 ms (see below). |
| `hooks.rs` | Install/remove the Claude Code Notification hook in `~/.claude/settings.json`; the `--hook-event` mode; tailing `claude-events.jsonl`. |
| `reminders.rs` | 1 s scheduler thread: interval nudges (paused while away), daily `HH:MM` reminders (once per day), pomodoro state machine. |
| `models.rs` | Lists BongoCat's installed models and imported models; serves their files over `http://bcmodel.localhost/<id>/<path>` with canonical-path checks so nothing outside a model root is readable; import (folder picker, copy) and remove. |
| `tray.rs` | Tray icon and the shared menu (also used as the cat's right-click popup); keeps check items in sync with settings; opens Settings / Sessions windows. |
| `terminal.rs` | `wt.exe -d <dir>` with a `cmd` fallback. Directories and session ids must come from tracked sessions and are passed as args / working dir, never through a shell string. |

## Session detection (`agents.rs`)

Sources: `~/.claude/projects/<project>/<session>.jsonl` (subagent logs one level deeper are
skipped) and `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`. Only logs touched in the last two
hours are tracked.

**Startup:** existing logs are read *quietly* from their last 512 KB, so the list shows current
states without announcing history. Logs that appear later are read from the start, with
events.

**Per line → signal → state:**

| Claude transcript line | Signal |
|---|---|
| `user` with text (not `isMeta` / `isSidechain`) | Working: *Working on: …* |
| `user` with `tool_result` | Working: *Thinking…* |
| `assistant` `tool_use` `AskUserQuestion` | **Input**: `questions[0].question` |
| `assistant` `tool_use` `ExitPlanMode` | **Input**: plan approval |
| `assistant` `tool_use` (other) | Working: *Running: …* / *Editing x* / … |
| `assistant` text with `stop_reason: end_turn` | **Done** (summary = first line), or **Input** if the last line ends with `?` |
| `system` `turn_duration` | **Done** |
| `ai-title` | session title |
| any line | `cwd` → project, `gitBranch` |
| hook event (`claude-events.jsonl`) | **Input**: hook `message` (idle reminders after Done are ignored) |

Transitions into Input or Done emit `agent-input` / `agent-done`. Working with no writes for 10
minutes becomes Idle. A pending tool call silent for 30 s sets `maybeWaiting`.

Half-written lines wait for the next poll; truncated files restart from zero; reads are capped
per poll.

## Frontend

- **`main.js`** reads settings and loads `pixel.js` or `live2d.js` (lazy chunk with pixi). If a
  model fails it falls back to the pixel cat. Changing the model reloads the page. Right-click
  → `show_menu`. Webview errors are mirrored to stderr via `frontend_log`.
- **`pixel.js`** draws the 40×40 pixel scene (`cat-art.js`) to an offscreen canvas and scales
  it up with smoothing off.
- **`live2d.js`** builds background, then the Live2D canvas, then the key-overlay `<img>`
  layers. Parameters are written into the core model in `beforeModelUpdate` so motions,
  breath and blink can't overwrite them.
- **`bubble.js`** is the DOM bubble layer shared by both renderers: a message queue, pinned
  note, thought dots / `?`, pomodoro pill, and a WebAudio chime.

## Files on disk

| Path | What |
|---|---|
| `%APPDATA%\com.arpit.bomboclat\settings.json` | Settings |
| `%APPDATA%\com.arpit.bomboclat\models\` | Imported Live2D models |
| `%APPDATA%\com.arpit.bomboclat\claude-events.jsonl` | Hook events (only with precise alerts on) |
| `~/.claude/settings.json.bomboclat-backup` | Your Claude settings before the hook was first added |

## Tests

`cargo test` in `src-tauri`: transcript and Codex parsing, state transitions and
announcements, partial-line handling, unicode-safe truncation, hook add/remove keeping other
hooks, reminder timing including the away pause.
