//! Optional Claude Code hook for precise "needs you" alerts.
//!
//! Permission prompts ("Allow Bash?") never reach the session transcript; Claude
//! Code reports them only to Notification hooks. When the user opts in, we add
//! one hook to `~/.claude/settings.json`:
//!
//! ```json
//! { "hooks": { "Notification": [ { "matcher": "", "hooks": [
//!     { "type": "command", "command": "\"C:/.../bomboclat.exe\" --hook-event" } ] } ] } }
//! ```
//!
//! Claude runs it with the notification JSON on stdin; `bomboclat.exe
//! --hook-event` appends one line to `claude-events.jsonl` in our data dir and
//! exits without starting the app. The watcher in `agents.rs` tails that file.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Runtime};

/// Must match `identifier` in tauri.conf.json: the hook process has no Tauri
/// context to ask for the data dir.
const APP_ID: &str = "com.arpit.bomboclat";
const EVENTS_FILE: &str = "claude-events.jsonl";
const HOOK_FLAG: &str = "--hook-event";
const MAX_STDIN: u64 = 256 * 1024;

fn events_path_standalone() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join(APP_ID).join(EVENTS_FILE))
}

fn events_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join(EVENTS_FILE))
}

/// True when this process was started by the Claude Code hook.
pub fn is_hook_invocation() -> bool {
    std::env::args().any(|a| a == HOOK_FLAG)
}

/// Entry point for `bomboclat.exe --hook-event`. Never fails loudly: a hook
/// error would show up in the user's Claude session.
pub fn hook_event_main() {
    let mut raw = Vec::new();
    let _ = std::io::stdin().take(MAX_STDIN).read_to_end(&mut raw);
    let Ok(v) = serde_json::from_slice::<Value>(&raw) else { return };
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let line = json!({
        "session_id": s("session_id"),
        "event": s("hook_event_name"),
        "type": s("notification_type"),
        "message": s("message"),
    });
    let Some(path) = events_path_standalone() else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

pub struct HookEvent {
    pub session_id: String,
    pub message: String,
    kind: String,
}

impl HookEvent {
    /// Claude's "still waiting for your input" nudge after a finished turn.
    pub fn is_idle_reminder(&self) -> bool {
        self.kind == "idle_prompt" || self.message.to_lowercase().contains("waiting for your input")
    }
}

/// Tails the hook events file. Starts at the end: old events are history.
#[derive(Default)]
pub struct EventTail {
    pos: Option<u64>,
    partial: Vec<u8>,
}

impl EventTail {
    pub fn read_new<R: Runtime>(&mut self, app: &AppHandle<R>) -> Vec<HookEvent> {
        let Some(path) = events_path(app) else { return Vec::new() };
        let len = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let pos = *self.pos.get_or_insert(len);
        if len < pos {
            self.pos = Some(0);
            self.partial.clear();
            return Vec::new();
        }
        if len == pos {
            return Vec::new();
        }
        let mut buf = Vec::new();
        let Ok(mut f) = File::open(&path) else { return Vec::new() };
        if f.seek(SeekFrom::Start(pos)).is_err() || f.take(len - pos).read_to_end(&mut buf).is_err() {
            return Vec::new();
        }
        self.pos = Some(pos + buf.len() as u64);
        self.partial.extend_from_slice(&buf);
        let mut out = Vec::new();
        while let Some(nl) = self.partial.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.partial.drain(..=nl).collect();
            let Ok(v) = serde_json::from_slice::<Value>(&line) else { continue };
            let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            if s("event") == "Notification" && !s("session_id").is_empty() && !s("message").is_empty() {
                out.push(HookEvent { session_id: s("session_id"), message: s("message"), kind: s("type") });
            }
        }
        out
    }
}

// ---- install / uninstall -------------------------------------------------

fn claude_settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".claude").join("settings.json"))
}

fn our_command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // Forward slashes work in both Git Bash and cmd, which Claude may use.
    Ok(format!("\"{}\" {HOOK_FLAG}", exe.display().to_string().replace('\\', "/")))
}

fn is_ours(hook: &Value) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| c.contains(HOOK_FLAG) && c.to_lowercase().contains("bomboclat"))
}

fn read_claude_settings() -> Result<(PathBuf, Value), String> {
    let path = claude_settings_path().ok_or("No home folder found.")?;
    let v = match fs::read_to_string(&path) {
        Ok(text) if text.trim().is_empty() => json!({}),
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("~/.claude/settings.json isn't valid JSON ({e}); not touching it."))?,
        Err(_) => json!({}),
    };
    if !v.is_object() {
        return Err("~/.claude/settings.json isn't a JSON object; not touching it.".into());
    }
    Ok((path, v))
}

/// Removes our hook entries; drops groups and the event key if left empty.
fn strip_ours(settings: &mut Value) {
    let Some(groups) = settings.pointer_mut("/hooks/Notification").and_then(Value::as_array_mut) else { return };
    for g in groups.iter_mut() {
        if let Some(hooks) = g.get_mut("hooks").and_then(Value::as_array_mut) {
            hooks.retain(|h| !is_ours(h));
        }
    }
    groups.retain(|g| g.get("hooks").and_then(Value::as_array).is_none_or(|h| !h.is_empty()));
    if groups.is_empty() {
        if let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) {
            hooks.remove("Notification");
            if hooks.is_empty() {
                settings.as_object_mut().unwrap().remove("hooks");
            }
        }
    }
}

fn add_ours(settings: &mut Value, command: &str) {
    strip_ours(settings);
    let root = settings.as_object_mut().unwrap();
    let hooks = root.entry("hooks").or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    let groups = hooks.as_object_mut().unwrap().entry("Notification").or_insert_with(|| json!([]));
    if !groups.is_array() {
        *groups = json!([]);
    }
    groups.as_array_mut().unwrap().push(json!({
        "matcher": "",
        "hooks": [{ "type": "command", "command": command }],
    }));
}

fn installed_command(settings: &Value) -> Option<String> {
    settings
        .pointer("/hooks/Notification")?
        .as_array()?
        .iter()
        .filter_map(|g| g.get("hooks").and_then(Value::as_array))
        .flatten()
        .find(|h| is_ours(h))
        .and_then(|h| h.get("command").and_then(Value::as_str).map(String::from))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookStatus {
    pub installed: bool,
    /// Installed but pointing at another copy of the app (moved/reinstalled).
    pub stale: bool,
}

fn status() -> HookStatus {
    let installed = read_claude_settings().ok().and_then(|(_, v)| installed_command(&v));
    let stale = match (&installed, our_command()) {
        (Some(cmd), Ok(ours)) => *cmd != ours,
        _ => false,
    };
    HookStatus { installed: installed.is_some(), stale }
}

#[tauri::command]
pub fn claude_hook_status() -> HookStatus {
    status()
}

#[tauri::command]
pub fn set_claude_hook(on: bool) -> Result<HookStatus, String> {
    let (path, mut settings) = read_claude_settings()?;
    if path.exists() {
        // One backup of the user's own file, taken before our first change.
        let backup = path.with_extension("json.bomboclat-backup");
        if !backup.exists() {
            fs::copy(&path, &backup).map_err(|e| format!("Backup failed: {e}"))?;
        }
    }
    if on {
        add_ours(&mut settings, &our_command()?);
    } else {
        strip_ours(&mut settings);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, text + "\n").map_err(|e| format!("Couldn't write settings: {e}"))?;
    Ok(status())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = "\"C:/x/bomboclat.exe\" --hook-event";

    #[test]
    fn add_keeps_other_hooks_and_is_idempotent() {
        let mut s = json!({
            "model": "opus",
            "hooks": { "Notification": [ { "matcher": "", "hooks": [ { "type": "command", "command": "notify-send hi" } ] } ],
                       "Stop": [] }
        });
        add_ours(&mut s, CMD);
        add_ours(&mut s, CMD);
        let groups = s.pointer("/hooks/Notification").unwrap().as_array().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(installed_command(&s).as_deref(), Some(CMD));
        assert_eq!(s["model"], "opus");
        assert!(s["hooks"]["Stop"].is_array());
    }

    #[test]
    fn strip_removes_only_ours_and_cleans_up() {
        let mut s = json!({ "model": "opus" });
        add_ours(&mut s, CMD);
        strip_ours(&mut s);
        assert_eq!(s, json!({ "model": "opus" }));

        let mut s = json!({ "hooks": { "Notification": [ { "matcher": "", "hooks": [
            { "type": "command", "command": "other" }, { "type": "command", "command": CMD } ] } ] } });
        strip_ours(&mut s);
        assert_eq!(s, json!({ "hooks": { "Notification": [ { "matcher": "", "hooks": [
            { "type": "command", "command": "other" } ] } ] } }));
    }
}
