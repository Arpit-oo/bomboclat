//! Watches AI coding agents you run yourself by tailing their session logs:
//! - Claude Code: `~/.claude/projects/<project>/<session>.jsonl`
//! - Codex CLI:   `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
//!
//! Emits `agents` (status summary) whenever it changes and `agent-done` when a
//! session finishes a turn. Nothing is launched or modified.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

const POLL: Duration = Duration::from_millis(700);
/// Only logs written this recently are tailed.
const ACTIVE_WINDOW: Duration = Duration::from_secs(30 * 60);
/// A session marked working with no log writes for this long is assumed
/// abandoned (killed terminal, crash) and goes back to idle quietly.
const STALE_AFTER: Duration = Duration::from_secs(10 * 60);
/// Cap on bytes read per poll so a huge first write can't stall the thread.
const MAX_READ: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Working,
}

/// What one log line says about its session.
#[derive(Debug, PartialEq, Eq)]
enum Signal {
    Working,
    Done,
}

struct Tail {
    agent: Agent,
    pos: u64,
    partial: Vec<u8>,
    state: State,
    last_write: Instant,
    project: String,
}

#[derive(Clone, Serialize, PartialEq)]
pub struct AgentsStatus {
    pub working: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDone {
    pub agent: Agent,
    pub project: String,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from)
}

fn recent(path: &Path, now: SystemTime) -> bool {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| now.duration_since(t).ok())
        .is_some_and(|age| age < ACTIVE_WINDOW)
}

fn jsonl_in(dir: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
}

fn subdirs(dir: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
}

/// Session logs touched within [`ACTIVE_WINDOW`]. Claude subagent logs live a
/// level deeper (`<session>/subagents/`) and are skipped: the parent session
/// already reports the turn.
fn candidates(claude: bool, codex: bool) -> Vec<(Agent, PathBuf)> {
    let Some(home) = home() else { return Vec::new() };
    let now = SystemTime::now();
    let mut out = Vec::new();
    if claude {
        for project in subdirs(&home.join(".claude").join("projects")) {
            out.extend(jsonl_in(&project).filter(|p| recent(p, now)).map(|p| (Agent::Claude, p)));
        }
    }
    if codex {
        // Only today's and yesterday's folders, by mtime; the tree grows forever.
        for year in subdirs(&home.join(".codex").join("sessions")) {
            for month in subdirs(&year) {
                for day in subdirs(&month).filter(|d| recent(d, now)) {
                    out.extend(jsonl_in(&day).filter(|p| recent(p, now)).map(|p| (Agent::Codex, p)));
                }
            }
        }
    }
    out
}

fn claude_signal(v: &Value) -> Option<Signal> {
    match v.get("type")?.as_str()? {
        // A prompt or a tool result: the agent has work to do.
        "user" => Some(Signal::Working),
        "assistant" => match v.pointer("/message/stop_reason").and_then(Value::as_str) {
            Some("end_turn") | Some("stop_sequence") | Some("max_tokens") => Some(Signal::Done),
            _ => Some(Signal::Working),
        },
        // Written once per finished turn.
        "system" if v.get("subtype").and_then(Value::as_str) == Some("turn_duration") => Some(Signal::Done),
        _ => None,
    }
}

fn codex_signal(v: &Value) -> Option<Signal> {
    if v.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    match v.pointer("/payload/type")?.as_str()? {
        "task_started" | "user_message" => Some(Signal::Working),
        "task_complete" | "turn_aborted" => Some(Signal::Done),
        _ => None,
    }
}

fn project_of(agent: Agent, v: &Value) -> Option<String> {
    let cwd = match agent {
        Agent::Claude => v.get("cwd"),
        Agent::Codex => v.pointer("/payload/cwd"),
    }?
    .as_str()?;
    Path::new(cwd).file_name().map(|n| n.to_string_lossy().into_owned())
}

/// Reads whatever was appended since the last poll; returns (signals, wrote).
fn read_new(path: &Path, tail: &mut Tail) -> (Vec<Signal>, bool) {
    let Ok(len) = fs::metadata(path).map(|m| m.len()) else { return (Vec::new(), false) };
    if len < tail.pos {
        // Rewritten or truncated: start over.
        tail.pos = 0;
        tail.partial.clear();
    }
    if len == tail.pos {
        return (Vec::new(), false);
    }
    let Ok(mut f) = File::open(path) else { return (Vec::new(), false) };
    let want = (len - tail.pos).min(MAX_READ);
    let mut buf = Vec::with_capacity(want as usize);
    if f.seek(SeekFrom::Start(tail.pos)).is_err() || f.take(want).read_to_end(&mut buf).is_err() {
        return (Vec::new(), false);
    }
    tail.pos += buf.len() as u64;
    tail.partial.extend_from_slice(&buf);

    let mut signals = Vec::new();
    // Only complete lines; a half-written line waits for the next poll.
    while let Some(nl) = tail.partial.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = tail.partial.drain(..=nl).collect();
        let Ok(v) = serde_json::from_slice::<Value>(&line) else { continue };
        if let Some(p) = project_of(tail.agent, &v) {
            tail.project = p;
        }
        let sig = match tail.agent {
            Agent::Claude => claude_signal(&v),
            Agent::Codex => codex_signal(&v),
        };
        signals.extend(sig);
    }
    (signals, true)
}

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        let mut tails: HashMap<PathBuf, Tail> = HashMap::new();
        let mut first_scan = true;
        let mut last_status: Option<AgentsStatus> = None;
        loop {
            let s = crate::settings::current(&app);
            let found = if s.agent_watch { candidates(true, true) } else { Vec::new() };
            if !s.agent_watch {
                tails.clear();
            }
            let now = Instant::now();
            for (agent, path) in found {
                let tail = tails.entry(path.clone()).or_insert_with(|| Tail {
                    agent,
                    // Existing history is skipped; a log that appears later is
                    // a new session and is read from the top.
                    pos: if first_scan { fs::metadata(&path).map(|m| m.len()).unwrap_or(0) } else { 0 },
                    partial: Vec::new(),
                    state: State::Idle,
                    last_write: now,
                    project: String::new(),
                });
                let (signals, wrote) = read_new(&path, tail);
                if wrote {
                    tail.last_write = now;
                }
                for sig in signals {
                    match (sig, tail.state) {
                        (Signal::Working, _) => tail.state = State::Working,
                        (Signal::Done, State::Working) => {
                            tail.state = State::Idle;
                            let _ = app.emit(
                                "agent-done",
                                AgentDone { agent: tail.agent, project: tail.project.clone() },
                            );
                        }
                        (Signal::Done, State::Idle) => {}
                    }
                }
                if tail.state == State::Working && now.duration_since(tail.last_write) > STALE_AFTER {
                    tail.state = State::Idle;
                }
            }
            first_scan = false;
            // Forget logs that went quiet long ago.
            tails.retain(|_, t| now.duration_since(t.last_write) < ACTIVE_WINDOW);

            let status = AgentsStatus { working: tails.values().filter(|t| t.state == State::Working).count() };
            if last_status.as_ref() != Some(&status) {
                let _ = app.emit("agents", &status);
                crate::tray::set_agent_tooltip(&app, status.working);
                last_status = Some(status);
            }
            std::thread::sleep(POLL);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn claude_turn_lifecycle() {
        assert_eq!(claude_signal(&json!({"type": "user", "message": {"content": "hi"}})), Some(Signal::Working));
        assert_eq!(
            claude_signal(&json!({"type": "assistant", "message": {"stop_reason": "tool_use"}})),
            Some(Signal::Working)
        );
        assert_eq!(
            claude_signal(&json!({"type": "assistant", "message": {"stop_reason": "end_turn"}})),
            Some(Signal::Done)
        );
        assert_eq!(claude_signal(&json!({"type": "system", "subtype": "turn_duration"})), Some(Signal::Done));
        assert_eq!(claude_signal(&json!({"type": "attachment"})), None);
    }

    #[test]
    fn codex_turn_lifecycle() {
        let ev = |t: &str| json!({"type": "event_msg", "payload": {"type": t}});
        assert_eq!(codex_signal(&ev("task_started")), Some(Signal::Working));
        assert_eq!(codex_signal(&ev("task_complete")), Some(Signal::Done));
        assert_eq!(codex_signal(&ev("token_count")), None);
    }

    #[test]
    fn partial_lines_wait_for_newline() {
        let dir = std::env::temp_dir().join(format!("bomboclat-agents-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.jsonl");
        let mut tail = Tail {
            agent: Agent::Claude,
            pos: 0,
            partial: Vec::new(),
            state: State::Idle,
            last_write: Instant::now(),
            project: String::new(),
        };
        fs::write(&path, b"{\"type\":\"user\",\"cwd\":\"C:\\\\code\\\\demo\"}\n{\"type\":\"assis").unwrap();
        let (sigs, _) = read_new(&path, &mut tail);
        assert_eq!(sigs, vec![Signal::Working]);
        assert_eq!(tail.project, "demo");
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        std::io::Write::write_all(&mut f, b"tant\",\"message\":{\"stop_reason\":\"end_turn\"}}\n").unwrap();
        let (sigs, _) = read_new(&path, &mut tail);
        assert_eq!(sigs, vec![Signal::Done]);
        let _ = fs::remove_dir_all(dir);
    }
}
