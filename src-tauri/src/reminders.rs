//! Stretch / water nudges, daily reminders at a set time, and the pomodoro
//! timer. Messages go out as `bubble` events; while a pomodoro runs a
//! `pomodoro` tick goes out every second.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::settings::{self, Settings};

#[derive(Clone, Serialize)]
pub struct Bubble {
    pub text: String,
    /// "reminder" | "pomodoro" | "test"
    pub kind: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Focus,
    Break,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroTick {
    /// None = stopped.
    pub phase: Option<Phase>,
    pub remaining_secs: u64,
}

/// Running pomodoro phase and when it ends.
#[derive(Default)]
pub struct PomodoroState(pub Mutex<Option<(Phase, Instant)>>);

/// "Time to stretch" + name -> "Time to stretch, Arpit!"
pub fn greet(msg: &str, name: &str) -> String {
    let name = name.trim();
    if name.is_empty() { format!("{msg}!") } else { format!("{msg}, {name}!") }
}

fn bubble<R: Runtime>(app: &AppHandle<R>, text: String, kind: &'static str) {
    let _ = app.emit("bubble", Bubble { text, kind });
}

fn mins(n: u32) -> Duration {
    Duration::from_secs(n as u64 * 60)
}

/// Repeating nudge that restarts its countdown when its interval changes.
struct Nudge {
    every: u32,
    since: Instant,
}

impl Nudge {
    fn new() -> Self {
        Self { every: 0, since: Instant::now() }
    }

    fn due(&mut self, every: u32, now: Instant) -> bool {
        if every != self.every {
            self.every = every;
            self.since = now;
        }
        if every == 0 || now.duration_since(self.since) < mins(every) {
            return false;
        }
        self.since = now;
        true
    }
}

pub fn start_focus<R: Runtime>(app: &AppHandle<R>) {
    let s = settings::current(app);
    *app.state::<PomodoroState>().0.lock().unwrap() = Some((Phase::Focus, Instant::now() + mins(s.pomodoro_focus_mins)));
    bubble(app, greet(&format!("Focus time, {} minutes. Let's go", s.pomodoro_focus_mins), ""), "pomodoro");
    crate::tray::set_focus_running(app, true);
    tick(app);
}

pub fn stop_focus<R: Runtime>(app: &AppHandle<R>) {
    *app.state::<PomodoroState>().0.lock().unwrap() = None;
    crate::tray::set_focus_running(app, false);
    tick(app);
}

pub fn focus_running<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<PomodoroState>().0.lock().unwrap().is_some()
}

fn tick<R: Runtime>(app: &AppHandle<R>) {
    let cur = *app.state::<PomodoroState>().0.lock().unwrap();
    let payload = match cur {
        Some((phase, ends)) => PomodoroTick {
            phase: Some(phase),
            remaining_secs: ends.saturating_duration_since(Instant::now()).as_secs(),
        },
        None => PomodoroTick { phase: None, remaining_secs: 0 },
    };
    let _ = app.emit("pomodoro", payload);
}

/// Advances the pomodoro when a phase runs out.
fn step_pomodoro<R: Runtime>(app: &AppHandle<R>, s: &Settings, now: Instant) {
    let next = {
        let state = app.state::<PomodoroState>();
        let mut cur = state.0.lock().unwrap();
        let Some((phase, ends)) = *cur else { return };
        if now < ends {
            None
        } else {
            let (p, len) = match phase {
                Phase::Focus => (Phase::Break, s.pomodoro_break_mins),
                Phase::Break => (Phase::Focus, s.pomodoro_focus_mins),
            };
            *cur = Some((p, now + mins(len)));
            Some((p, len))
        }
    };
    match next {
        Some((Phase::Break, len)) => bubble(app, greet(&format!("Nice focus! Take {len} minutes off"), &s.user_name), "pomodoro"),
        Some((Phase::Focus, len)) => bubble(app, greet(&format!("Break's over. Focus for {len} minutes"), &s.user_name), "pomodoro"),
        None => {}
    }
    tick(app);
}

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        let mut stretch = Nudge::new();
        let mut water = Nudge::new();
        // Reminder key -> the day it last fired, so each fires once per day.
        let mut fired: HashMap<String, NaiveDate> = HashMap::new();
        loop {
            let s = settings::current(&app);
            let now = Instant::now();
            if stretch.due(s.stretch_mins, now) {
                bubble(&app, greet("Time to stretch", &s.user_name), "reminder");
            }
            if water.due(s.water_mins, now) {
                bubble(&app, greet("Drink some water", &s.user_name), "reminder");
            }

            let local = Local::now();
            let hhmm = local.format("%H:%M").to_string();
            let today = local.date_naive();
            for r in &s.reminders {
                let key = format!("{}|{}", r.time, r.message);
                if r.time == hhmm && fired.get(&key) != Some(&today) && !r.message.trim().is_empty() {
                    fired.insert(key, today);
                    bubble(&app, r.message.trim().to_string(), "reminder");
                }
            }

            step_pomodoro(&app, &s, now);
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

#[tauri::command]
pub fn start_focus_cmd(app: AppHandle) {
    start_focus(&app);
}

#[tauri::command]
pub fn stop_focus_cmd(app: AppHandle) {
    stop_focus(&app);
}

/// Shows a sample bubble so settings changes can be previewed.
#[tauri::command]
pub fn test_bubble(app: AppHandle) {
    let s = settings::current(&app);
    bubble(&app, greet("Hi, this is how messages look", &s.user_name), "test");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_with_and_without_name() {
        assert_eq!(greet("Time to stretch", ""), "Time to stretch!");
        assert_eq!(greet("Time to stretch", " Arpit "), "Time to stretch, Arpit!");
    }

    #[test]
    fn nudge_fires_after_interval_and_resets_on_change() {
        let t0 = Instant::now();
        let mut n = Nudge::new();
        assert!(!n.due(1, t0));
        assert!(!n.due(1, t0 + Duration::from_secs(59)));
        assert!(n.due(1, t0 + Duration::from_secs(60)));
        assert!(!n.due(1, t0 + Duration::from_secs(61)));
        // Interval change restarts the countdown.
        assert!(!n.due(2, t0 + Duration::from_secs(200)));
        assert!(!n.due(0, t0 + Duration::from_secs(10_000)));
    }
}
