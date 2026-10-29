//! Watches AI coding agents you run yourself by tailing their session logs:
//! - Claude Code: `~/.claude/projects/<project>/<session>.jsonl`
//! - Codex CLI:   `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
//!
//! Each session is Working, waiting on you (Input), Done or Idle. Permission
//! prompts never reach the transcript, so an optional Claude Code
//! Notification hook (see `hooks.rs`) feeds those in.
//!
//! Events:
//! - `sessions`     full list, whenever any session changes
//! - `agents`       `{ working, input }` counts for the cat
//! - `agent-input`  a session started waiting on you (with the question)
//! - `agent-done`   a session finished its turn
//!
//! Nothing is launched or modified.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};

const POLL: Duration = Duration::from_millis(700);
/// Sessions written this recently are tracked and listed.
const ACTIVE_WINDOW: Duration = Duration::from_secs(2 * 60 * 60);
/// Working with no log writes for this long = killed terminal or crash.
const STALE_AFTER: Duration = Duration::from_secs(10 * 60);
/// A tool call with no output for this long may be a permission prompt.
const MAYBE_WAITING_AFTER: Duration = Duration::from_secs(30);
/// How much of an existing log is read at startup to learn its state.
const BOOTSTRAP_BYTES: u64 = 512 * 1024;
/// Cap on bytes read per poll so a huge write can't stall the thread.
const MAX_READ: u64 = 4 * 1024 * 1024;
const DETAIL_MAX: usize = 160;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Idle,
    Working,
    Input,
    Done,
}

/// What one log line says about its session.
#[derive(Debug, PartialEq, Eq)]
enum Signal {
    /// Busy, with a short description of what it's doing.
    Working(String),
    /// Waiting on the user, with what it's asking.
    Input(String),
    /// Turn finished, with a one-line summary (may be empty).
    Done(String),
    /// Session title (Claude's `ai-title`).
    Title(String),
}

struct Session {
    agent: Agent,
    id: String,
    pos: u64,
    partial: Vec<u8>,
    state: State,
    detail: String,
    title: String,
    project: String,
    cwd: String,
    branch: String,
    first_prompt: String,
    changed_ms: u64,
    last_write: Instant,
    last_write_ms: u64,
    /// A tool call that hasn't produced a result yet.
    pending_tool: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub id: String,
    pub agent: Agent,
    pub project: String,
    pub cwd: String,
    pub title: String,
    pub branch: String,
    pub state: State,
    pub detail: String,
    /// Unix ms of the last state change.
    pub changed_ms: u64,
    /// Unix ms of the last log write.
    pub updated_ms: u64,
    /// Working on a tool call that has been silent a while: maybe a
    /// permission prompt the transcript can't show.
    pub maybe_waiting: bool,
}

#[derive(Clone, Serialize, PartialEq)]
pub struct AgentsStatus {
    pub working: usize,
    pub input: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub agent: Agent,
    pub project: String,
    pub title: String,
    pub detail: String,
}

/// Latest session list, for the `list_sessions` command.
#[derive(Default)]
pub struct SessionsState(pub Mutex<Vec<SessionView>>);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
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
fn candidates() -> Vec<(Agent, PathBuf)> {
    let Some(home) = home() else { return Vec::new() };
    let now = SystemTime::now();
    let mut out = Vec::new();
    for project in subdirs(&home.join(".claude").join("projects")) {
        out.extend(jsonl_in(&project).filter(|p| recent(p, now)).map(|p| (Agent::Claude, p)));
    }
    // Only recently touched day folders; the tree grows forever.
    for year in subdirs(&home.join(".codex").join("sessions")) {
        for month in subdirs(&year) {
            for day in subdirs(&month).filter(|d| recent(d, now)) {
                out.extend(jsonl_in(&day).filter(|p| recent(p, now)).map(|p| (Agent::Codex, p)));
            }
        }
    }
    out
}

/// First non-empty line, whitespace squashed, cut to `max` chars on a char
/// boundary.
pub fn short(s: &str, max: usize) -> String {
    let line = s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let squashed: String = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if squashed.chars().count() <= max {
        squashed
    } else {
        let cut: String = squashed.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", cut.trim_end())
    }
}

fn file_name(p: &str) -> String {
    Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string())
}

/// What a Claude Code tool call means, in a few words.
fn tool_activity(name: &str, input: &Value) -> String {
    let s = |k: &str| input.get(k).and_then(Value::as_str).unwrap_or("");
    match name {
        "Bash" | "PowerShell" => {
            let what = if s("description").is_empty() { s("command") } else { s("description") };
            format!("Running: {}", short(what, 80))
        }
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => {
            let p = if s("file_path").is_empty() { s("notebook_path") } else { s("file_path") };
            format!("Editing {}", file_name(p))
        }
        "Read" => format!("Reading {}", file_name(s("file_path"))),
        "Grep" | "Glob" => format!("Searching for {}", short(s("pattern"), 60)),
        "Task" | "Agent" => format!("Running a subagent: {}", short(s("description"), 60)),
        "WebFetch" => format!("Reading {}", short(s("url"), 60)),
        "WebSearch" => format!("Searching the web: {}", short(s("query"), 60)),
        "TodoWrite" => "Updating its todo list".into(),
        other => format!("Using {other}"),
    }
}

fn claude_signal(v: &Value) -> Option<Signal> {
    if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    match v.get("type")?.as_str()? {
        "ai-title" => v.get("aiTitle").and_then(Value::as_str).map(|t| Signal::Title(short(t, 80))),
        "user" => {
            if v.get("isMeta").and_then(Value::as_bool) == Some(true) {
                return None;
            }
            match v.pointer("/message/content")? {
                Value::String(text) => Some(Signal::Working(format!("Working on: {}", short(text, 100)))),
                Value::Array(blocks) => {
                    let is_tool_result =
                        blocks.iter().any(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"));
                    if is_tool_result {
                        return Some(Signal::Working("Thinking…".into()));
                    }
                    let text = blocks.iter().find_map(|b| b.get("text").and_then(Value::as_str)).unwrap_or("");
                    Some(Signal::Working(format!("Working on: {}", short(text, 100))))
                }
                _ => None,
            }
        }
        "assistant" => {
            let stop = v.pointer("/message/stop_reason").and_then(Value::as_str);
            let blocks = v.pointer("/message/content")?.as_array()?;
            for b in blocks {
                match b.get("type").and_then(Value::as_str) {
                    Some("tool_use") => {
                        let name = b.get("name").and_then(Value::as_str).unwrap_or("");
                        let input = b.get("input").cloned().unwrap_or(Value::Null);
                        return Some(match name {
                            "AskUserQuestion" => {
                                let q = input
                                    .pointer("/questions/0/question")
                                    .and_then(Value::as_str)
                                    .unwrap_or("It has a question for you");
                                Signal::Input(short(q, DETAIL_MAX))
                            }
                            "ExitPlanMode" => Signal::Input("Plan ready: approve or change it".into()),
                            _ => Signal::Working(tool_activity(name, &input)),
                        });
                    }
                    Some("text") if matches!(stop, Some("end_turn" | "stop_sequence" | "max_tokens")) => {
                        let text = b.get("text").and_then(Value::as_str).unwrap_or("").trim();
                        // A turn that ends on a question is waiting for an answer.
                        let last = text.lines().map(str::trim).rfind(|l| !l.is_empty()).unwrap_or("");
                        return Some(if last.ends_with('?') {
                            Signal::Input(short(last, DETAIL_MAX))
                        } else {
                            Signal::Done(short(text, DETAIL_MAX))
                        });
                    }
                    Some("thinking") if stop.is_none() || stop == Some("tool_use") => {
                        return Some(Signal::Working("Thinking…".into()));
                    }
                    _ => {}
                }
            }
            None
        }
        // Written once per finished turn.
        "system" if v.get("subtype").and_then(Value::as_str) == Some("turn_duration") => {
            Some(Signal::Done(String::new()))
        }
        _ => None,
    }
}

fn codex_signal(v: &Value) -> Option<Signal> {
    if v.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let p = v.get("payload")?;
    let s = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("");
    match p.get("type")?.as_str()? {
        "task_started" => Some(Signal::Working("Thinking…".into())),
        "user_message" => Some(Signal::Working(format!("Working on: {}", short(s("message"), 100)))),
        "exec_command_begin" => Some(Signal::Working("Running a command".into())),
        "exec_approval_request" | "apply_patch_approval_request" => Some(Signal::Input("Approve its next step".into())),
        "task_complete" => Some(Signal::Done(short(s("last_agent_message"), DETAIL_MAX))),
        "turn_aborted" => Some(Signal::Done("Stopped".into())),
        _ => None,
    }
}

fn meta_of(session: &mut Session, v: &Value) {
    let (cwd, branch) = match session.agent {
        Agent::Claude => (v.get("cwd"), v.get("gitBranch")),
        Agent::Codex => (v.pointer("/payload/cwd"), v.pointer("/payload/git/branch")),
    };
    if let Some(cwd) = cwd.and_then(Value::as_str) {
        if cwd != session.cwd {
            session.cwd = cwd.to_string();
            session.project = file_name(cwd);
        }
    }
    if let Some(b) = branch.and_then(Value::as_str).filter(|b| !b.is_empty()) {
        session.branch = b.to_string();
    }
    if session.agent == Agent::Claude && session.first_prompt.is_empty() {
        let is_prompt = v.get("type").and_then(Value::as_str) == Some("user")
            && v.get("isMeta").and_then(Value::as_bool) != Some(true);
        if let (true, Some(Value::String(t))) = (is_prompt, v.pointer("/message/content")) {
            session.first_prompt = short(t, 80);
        }
    }
}

/// Reads whatever was appended since the last poll; returns (signals, wrote).
fn read_new(path: &Path, s: &mut Session) -> (Vec<Signal>, bool) {
    let Ok(len) = fs::metadata(path).map(|m| m.len()) else { return (Vec::new(), false) };
    if len < s.pos {
        // Rewritten or truncated: start over.
        s.pos = 0;
        s.partial.clear();
    }
    if len == s.pos {
        return (Vec::new(), false);
    }
    let Ok(mut f) = File::open(path) else { return (Vec::new(), false) };
    let want = (len - s.pos).min(MAX_READ);
    let mut buf = Vec::with_capacity(want as usize);
    if f.seek(SeekFrom::Start(s.pos)).is_err() || f.take(want).read_to_end(&mut buf).is_err() {
        return (Vec::new(), false);
    }
    s.pos += buf.len() as u64;
    s.partial.extend_from_slice(&buf);

    let mut signals = Vec::new();
    // Only complete lines; a half-written line waits for the next poll. When
    // bootstrapping mid-file the first piece fails to parse and is skipped.
    while let Some(nl) = s.partial.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = s.partial.drain(..=nl).collect();
        let Ok(v) = serde_json::from_slice::<Value>(&line) else { continue };
        meta_of(s, &v);
        let sig = match s.agent {
            Agent::Claude => claude_signal(&v),
            Agent::Codex => codex_signal(&v),
        };
        signals.extend(sig);
    }
    (signals, true)
}

fn new_session(agent: Agent, path: &Path, now: Instant) -> Session {
    let id = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let modified_ms = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or_else(now_ms);
    let age = Duration::from_millis(now_ms().saturating_sub(modified_ms));
    Session {
        agent,
        id,
        pos: 0,
        partial: Vec::new(),
        state: State::Idle,
        detail: String::new(),
        title: String::new(),
        project: String::new(),
        cwd: String::new(),
        branch: String::new(),
        first_prompt: String::new(),
        changed_ms: modified_ms,
        last_write: now.checked_sub(age).unwrap_or(now),
        last_write_ms: modified_ms,
        pending_tool: false,
    }
}

/// Applies signals; returns the event to announce when the session newly
/// waits on the user or finishes.
fn apply(s: &mut Session, signals: Vec<Signal>) -> Option<(&'static str, String)> {
    let mut announce = None;
    for sig in signals {
        let (state, detail, pending) = match sig {
            Signal::Title(t) => {
                s.title = t;
                continue;
            }
            Signal::Working(d) => {
                let pending = !d.starts_with("Thinking") && !d.starts_with("Working on");
                (State::Working, d, pending)
            }
            Signal::Input(d) => (State::Input, d, false),
            Signal::Done(d) => {
                // turn_duration carries no text; keep the summary we have.
                let d = if d.is_empty() && s.state == State::Done { s.detail.clone() } else { d };
                (State::Done, d, false)
            }
        };
        let was = s.state;
        if state != was {
            s.changed_ms = now_ms();
        }
        s.state = state;
        s.detail = detail;
        s.pending_tool = pending;
        announce = match (was, state) {
            (State::Working | State::Idle | State::Done, State::Input) => Some(("agent-input", s.detail.clone())),
            (State::Working | State::Input, State::Done) => Some(("agent-done", s.detail.clone())),
            (_, State::Working) => None,
            _ => announce,
        };
    }
    announce
}

fn view(s: &Session, now: Instant) -> SessionView {
    let title = [&s.title, &s.first_prompt, &s.project]
        .into_iter()
        .find(|t| !t.is_empty())
        .cloned()
        .unwrap_or_default();
    SessionView {
        id: s.id.clone(),
        agent: s.agent,
        project: s.project.clone(),
        cwd: s.cwd.clone(),
        title,
        branch: s.branch.clone(),
        state: s.state,
        detail: s.detail.clone(),
        changed_ms: s.changed_ms,
        updated_ms: s.last_write_ms,
        maybe_waiting: s.state == State::Working
            && s.pending_tool
            && now.duration_since(s.last_write) > MAYBE_WAITING_AFTER,
    }
}

fn emit_event<R: Runtime>(app: &AppHandle<R>, name: &str, s: &Session, detail: String) {
    let title = if s.title.is_empty() { s.first_prompt.clone() } else { s.title.clone() };
    let _ = app.emit(name, AgentEvent { agent: s.agent, project: s.project.clone(), title, detail });
}

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        let mut sessions: HashMap<PathBuf, Session> = HashMap::new();
        let mut hook_events = crate::hooks::EventTail::default();
        let mut last_views: Vec<SessionView> = Vec::new();
        let mut last_status: Option<AgentsStatus> = None;
        let mut first_scan = true;
        loop {
            let settings = crate::settings::current(&app);
            let now = Instant::now();
            if settings.agent_watch {
                for (agent, path) in candidates() {
                    // Logs already there at startup are read quietly (tail
                    // only) to learn their state without announcing history.
                    let quiet = first_scan && !sessions.contains_key(&path);
                    let s = sessions.entry(path.clone()).or_insert_with(|| {
                        let mut s = new_session(agent, &path, now);
                        if quiet {
                            let len = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                            s.pos = len.saturating_sub(BOOTSTRAP_BYTES);
                        }
                        s
                    });
                    let (signals, wrote) = read_new(&path, s);
                    if wrote && !quiet {
                        s.last_write = now;
                        s.last_write_ms = now_ms();
                    }
                    if let Some((event, detail)) = apply(s, signals) {
                        if !quiet {
                            emit_event(&app, event, s, detail);
                        }
                    }
                }
                // Permission prompts and other notifications from the hook.
                for ev in hook_events.read_new(&app) {
                    let Some(s) = sessions.values_mut().find(|s| s.id == ev.session_id) else { continue };
                    if s.state == State::Done && ev.is_idle_reminder() {
                        continue;
                    }
                    if let Some((event, detail)) = apply(s, vec![Signal::Input(short(&ev.message, DETAIL_MAX))]) {
                        emit_event(&app, event, s, detail);
                    }
                }
            } else {
                sessions.clear();
            }
            first_scan = false;

            for s in sessions.values_mut() {
                if s.state == State::Working && now.duration_since(s.last_write) > STALE_AFTER {
                    s.state = State::Idle;
                    s.detail = "No activity for a while".into();
                    s.changed_ms = now_ms();
                }
            }
            sessions.retain(|path, s| now.duration_since(s.last_write) < ACTIVE_WINDOW && path.exists());

            let mut views: Vec<SessionView> = sessions.values().map(|s| view(s, now)).collect();
            views.sort_by_key(|v| std::cmp::Reverse(v.updated_ms));
            if views != last_views {
                let _ = app.emit("sessions", &views);
                app.state::<SessionsState>().0.lock().unwrap().clone_from(&views);
                last_views = views;
            }
            let status = AgentsStatus {
                working: last_views.iter().filter(|v| v.state == State::Working).count(),
                input: last_views.iter().filter(|v| v.state == State::Input).count(),
            };
            if last_status.as_ref() != Some(&status) {
                let _ = app.emit("agents", &status);
                crate::tray::set_agent_tooltip(&app, status.working, status.input);
                last_status = Some(status);
            }
            std::thread::sleep(POLL);
        }
    });
}

#[tauri::command]
pub fn list_sessions(app: AppHandle) -> Vec<SessionView> {
    app.state::<SessionsState>().0.lock().unwrap().clone()
}

/// Folder of the most recently active session, for "Open terminal".
pub fn latest_cwd<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    app.state::<SessionsState>().0.lock().unwrap().iter().map(|v| v.cwd.clone()).find(|c| !c.is_empty())
}

/// Whether `cwd` belongs to a tracked session (terminals only open there).
pub fn is_session_cwd<R: Runtime>(app: &AppHandle<R>, cwd: &str) -> bool {
    app.state::<SessionsState>().0.lock().unwrap().iter().any(|v| v.cwd == cwd)
}

/// Whether `id` is a tracked session id.
pub fn is_session_id<R: Runtime>(app: &AppHandle<R>, id: &str) -> Option<SessionView> {
    app.state::<SessionsState>().0.lock().unwrap().iter().find(|v| v.id == id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn working(s: &str) -> Option<Signal> {
        Some(Signal::Working(s.into()))
    }

    #[test]
    fn claude_prompt_tool_and_done() {
        assert_eq!(
            claude_signal(&json!({"type": "user", "message": {"content": "fix the login bug"}})),
            working("Working on: fix the login bug")
        );
        assert_eq!(
            claude_signal(&json!({"type": "assistant", "message": {"stop_reason": "tool_use",
                "content": [{"type": "tool_use", "name": "Edit", "input": {"file_path": "C:\\x\\auth.rs"}}]}})),
            working("Editing auth.rs")
        );
        assert_eq!(
            claude_signal(&json!({"type": "assistant", "message": {"stop_reason": "end_turn",
                "content": [{"type": "text", "text": "Fixed it.\nAll tests pass."}]}})),
            Some(Signal::Done("Fixed it.".into()))
        );
        assert_eq!(
            claude_signal(&json!({"type": "system", "subtype": "turn_duration"})),
            Some(Signal::Done(String::new()))
        );
        assert_eq!(claude_signal(&json!({"type": "attachment"})), None);
    }

    #[test]
    fn claude_questions_are_input() {
        assert_eq!(
            claude_signal(&json!({"type": "assistant", "message": {"stop_reason": "tool_use",
                "content": [{"type": "tool_use", "name": "AskUserQuestion",
                    "input": {"questions": [{"question": "Which database?"}]}}]}})),
            Some(Signal::Input("Which database?".into()))
        );
        assert_eq!(
            claude_signal(&json!({"type": "assistant", "message": {"stop_reason": "end_turn",
                "content": [{"type": "text", "text": "Done the refactor.\n\nShould I also update the docs?"}]}})),
            Some(Signal::Input("Should I also update the docs?".into()))
        );
    }

    #[test]
    fn meta_and_sidechain_lines_ignored() {
        assert_eq!(claude_signal(&json!({"type": "user", "isMeta": true, "message": {"content": "x"}})), None);
        assert_eq!(claude_signal(&json!({"type": "user", "isSidechain": true, "message": {"content": "x"}})), None);
    }

    #[test]
    fn codex_lifecycle() {
        let ev = |t: &str| json!({"type": "event_msg", "payload": {"type": t}});
        assert_eq!(codex_signal(&ev("task_started")), working("Thinking…"));
        assert_eq!(codex_signal(&ev("exec_approval_request")), Some(Signal::Input("Approve its next step".into())));
        assert_eq!(codex_signal(&ev("task_complete")), Some(Signal::Done(String::new())));
        assert_eq!(codex_signal(&ev("token_count")), None);
    }

    #[test]
    fn transitions_announce_input_and_done_once() {
        let mut s = new_session(Agent::Claude, Path::new("x.jsonl"), Instant::now());
        assert_eq!(apply(&mut s, vec![Signal::Working("a".into())]), None);
        assert_eq!(apply(&mut s, vec![Signal::Input("q?".into())]), Some(("agent-input", "q?".into())));
        assert_eq!(apply(&mut s, vec![Signal::Working("b".into())]), None);
        assert_eq!(apply(&mut s, vec![Signal::Done("ok".into())]), Some(("agent-done", "ok".into())));
        // turn_duration after the final text keeps the summary, no second event.
        assert_eq!(apply(&mut s, vec![Signal::Done(String::new())]), None);
        assert_eq!(s.detail, "ok");
    }

    #[test]
    fn short_cuts_on_char_boundary() {
        assert_eq!(short("  hello   world \nsecond", 50), "hello world");
        assert_eq!(short("abcdefghij", 5), "abcd…");
        assert_eq!(short("小熊猫昊昊小熊猫", 4), "小熊猫…");
    }

    #[test]
    fn partial_lines_wait_for_newline() {
        let dir = std::env::temp_dir().join(format!("bomboclat-agents-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.jsonl");
        let mut s = new_session(Agent::Claude, &path, Instant::now());
        fs::write(&path, b"{\"type\":\"user\",\"cwd\":\"C:\\\\code\\\\demo\",\"message\":{\"content\":\"go\"}}\n{\"type\":\"assis").unwrap();
        let (sigs, _) = read_new(&path, &mut s);
        assert_eq!(sigs, vec![Signal::Working("Working on: go".into())]);
        assert_eq!(s.project, "demo");
        assert_eq!(s.first_prompt, "go");
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        std::io::Write::write_all(
            &mut f,
            b"tant\",\"message\":{\"stop_reason\":\"end_turn\",\"content\":[{\"type\":\"text\",\"text\":\"ok\"}]}}\n",
        )
        .unwrap();
        let (sigs, _) = read_new(&path, &mut s);
        assert_eq!(sigs, vec![Signal::Done("ok".into())]);
        let _ = fs::remove_dir_all(dir);
    }
}
