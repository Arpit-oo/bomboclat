//! User settings: loaded from and saved to `settings.json` in the app config
//! dir. Rust owns the truth; windows get a `settings-changed` event on update.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Runtime};

use crate::models;

/// Pixel-cat window size at 100%, in logical pixels.
pub const BASE_W: f64 = 240.0;
pub const BASE_H: f64 = 220.0;

pub const SIZE_STEPS: [u32; 5] = [75, 100, 125, 150, 200];
pub const OPACITY_STEPS: [u32; 4] = [25, 50, 75, 100];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub click_through: bool,
    pub always_on_top: bool,
    /// Percent of the base window size.
    pub size: u32,
    /// Percent, 10-100.
    pub opacity: u32,
    pub flip: bool,
    /// Shows pressed keys above the cat. Off by default: it would show
    /// passwords to anyone watching a screen share.
    pub key_bubble: bool,
    /// Fade the cat out while the cursor is over it.
    pub hide_on_hover: bool,
    /// Idle seconds before the cat falls asleep; 0 = never.
    pub sleep_after_secs: u32,
    /// `pixel` for the built-in cat, otherwise a Live2D model id.
    pub model: String,
    /// Watch Claude Code / Codex session logs.
    pub agent_watch: bool,
    /// Say something when an agent finishes.
    pub agent_bubbles: bool,
    /// Used in messages ("Time to stretch, Arpit!").
    pub user_name: String,
    pub sounds: bool,
    /// Minutes between nudges; 0 = off.
    pub stretch_mins: u32,
    pub water_mins: u32,
    pub pomodoro_focus_mins: u32,
    pub pomodoro_break_mins: u32,
    /// Daily reminders.
    pub reminders: Vec<Reminder>,
    /// Always-visible note above the cat; empty = none.
    pub pinned_message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    /// Local time, "HH:MM" (24h).
    pub time: String,
    pub message: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            click_through: false,
            always_on_top: true,
            size: 100,
            opacity: 100,
            flip: false,
            key_bubble: false,
            hide_on_hover: false,
            sleep_after_secs: 60,
            model: models::PIXEL.into(),
            agent_watch: true,
            agent_bubbles: true,
            user_name: String::new(),
            sounds: true,
            stretch_mins: 0,
            water_mins: 0,
            pomodoro_focus_mins: 25,
            pomodoro_break_mins: 5,
            reminders: Vec::new(),
            pinned_message: String::new(),
        }
    }
}

impl Settings {
    fn clamped(mut self) -> Self {
        self.size = self.size.clamp(50, 300);
        self.opacity = self.opacity.clamp(10, 100);
        self.pomodoro_focus_mins = self.pomodoro_focus_mins.clamp(1, 180);
        self.pomodoro_break_mins = self.pomodoro_break_mins.clamp(1, 60);
        self.reminders.retain(|r| valid_hhmm(&r.time));
        self.reminders.truncate(50);
        self
    }
}

fn valid_hhmm(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && t[..2].parse::<u32>().is_ok_and(|h| h < 24)
        && t[3..].parse::<u32>().is_ok_and(|m| m < 60)
}

pub struct SettingsState(pub Mutex<Settings>);

fn path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default()
        .clamped()
}

fn save<R: Runtime>(app: &AppHandle<R>, s: &Settings) {
    let Some(p) = path(app) else { return };
    if let Some(dir) = p.parent() {
        let _ = fs::create_dir_all(dir);
    }
    match serde_json::to_string_pretty(s) {
        Ok(json) => {
            if let Err(e) = fs::write(&p, json) {
                eprintln!("[settings] save failed: {e}");
            }
        }
        Err(e) => eprintln!("[settings] encode failed: {e}"),
    }
}

pub fn current<R: Runtime>(app: &AppHandle<R>) -> Settings {
    app.state::<SettingsState>().0.lock().unwrap().clone()
}

/// Pushes window-level settings onto the main window. Everything else
/// (opacity, flip, bubble, ...) is drawn by the frontend.
pub fn apply<R: Runtime>(app: &AppHandle<R>, s: &Settings) {
    let Some(w) = app.get_webview_window("main") else { return };
    let _ = w.set_ignore_cursor_events(s.click_through);
    let _ = w.set_always_on_top(s.always_on_top);

    let scale = s.size as f64 / 100.0;
    let (base_w, base_h) = models::base_size(app, &s.model).unwrap_or((BASE_W, BASE_H));
    let target = LogicalSize::new(base_w * scale, base_h * scale);
    let (Ok(old), Ok(pos), Ok(dpr)) = (w.outer_size(), w.outer_position(), w.scale_factor()) else {
        let _ = w.set_size(target);
        return;
    };
    let new = target.to_physical::<i32>(dpr);
    if new.width as u32 == old.width && new.height as u32 == old.height {
        return;
    }
    // The cat sits bottom-centre, so keep that point fixed while resizing.
    let mut x = pos.x + (old.width as i32 - new.width) / 2;
    let mut y = pos.y + (old.height as i32 - new.height);
    // ...but never push it off the edge of the screen it's on.
    if let Ok(Some(m)) = w.current_monitor() {
        let area = m.work_area();
        let (ax, ay) = (area.position.x, area.position.y);
        x = x.clamp(ax, (ax + area.size.width as i32 - new.width).max(ax));
        y = y.clamp(ay, (ay + area.size.height as i32 - new.height).max(ay));
    }
    let _ = w.set_size(target);
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

/// Merges a partial settings object, then saves, applies and broadcasts.
pub fn update<R: Runtime>(app: &AppHandle<R>, patch: serde_json::Value) -> Settings {
    let next = {
        let state = app.state::<SettingsState>();
        let mut cur = state.0.lock().unwrap();
        let mut merged = serde_json::to_value(&*cur).unwrap_or_default();
        if let (Some(m), Some(p)) = (merged.as_object_mut(), patch.as_object()) {
            for (k, v) in p {
                m.insert(k.clone(), v.clone());
            }
        }
        if let Ok(s) = serde_json::from_value::<Settings>(merged) {
            *cur = s.clamped();
        }
        cur.clone()
    };
    save(app, &next);
    apply(app, &next);
    crate::tray::sync(app, &next);
    let _ = app.emit("settings-changed", &next);
    next
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    current(&app)
}

#[tauri::command]
pub fn update_settings(app: AppHandle, patch: serde_json::Value) -> Settings {
    update(&app, patch)
}

#[tauri::command]
pub fn reset_settings(app: AppHandle) -> Settings {
    update(&app, serde_json::to_value(Settings::default()).unwrap_or_default())
}
