//! Global keyboard/mouse hook. Runs rdev on its own thread and forwards
//! events to the webview as Tauri events.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rdev::{listen, Button, EventType};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InputEvent {
    KeyDown { key: String },
    KeyUp { key: String },
    MouseDown { button: String },
    MouseUp { button: String },
}

#[derive(Clone, Serialize)]
pub struct CursorPos {
    pub x: f64,
    pub y: f64,
}

/// Minimum gap between cursor events sent to the webview (~60 Hz).
const CURSOR_INTERVAL_MS: u64 = 16;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn button_name(b: Button) -> String {
    match b {
        Button::Left => "left".into(),
        Button::Right => "right".into(),
        Button::Middle => "middle".into(),
        Button::Unknown(n) => format!("x{n}"),
    }
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let last_cursor = AtomicU64::new(0);
        let result = listen(move |event| {
            let payload = match event.event_type {
                EventType::KeyPress(k) => InputEvent::KeyDown { key: format!("{k:?}") },
                EventType::KeyRelease(k) => InputEvent::KeyUp { key: format!("{k:?}") },
                EventType::ButtonPress(b) => InputEvent::MouseDown { button: button_name(b) },
                EventType::ButtonRelease(b) => InputEvent::MouseUp { button: button_name(b) },
                EventType::MouseMove { x, y } => {
                    let now = now_ms();
                    if now - last_cursor.load(Ordering::Relaxed) >= CURSOR_INTERVAL_MS {
                        last_cursor.store(now, Ordering::Relaxed);
                        let _ = app.emit("cursor", CursorPos { x, y });
                    }
                    return;
                }
                EventType::Wheel { .. } => return,
            };
            let _ = app.emit("input", payload);
        });
        if let Err(e) = result {
            eprintln!("[input] global hook failed: {e:?}");
        }
    });
}
