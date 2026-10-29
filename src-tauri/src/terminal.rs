//! Opens a terminal in a session's project folder, or resumes a Claude Code
//! session in a new terminal. Paths and ids must come from a tracked session,
//! and are passed as arguments / working directory, never through a shell
//! string.

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use tauri::{AppHandle, Runtime};

use crate::agents::{self, Agent};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn home() -> Option<String> {
    std::env::var("USERPROFILE").ok()
}

/// Windows Terminal if installed, else a classic console window. `args` run
/// inside it (empty = just a shell).
fn launch(dir: &Path, args: &[&str]) -> Result<(), String> {
    let mut wt = Command::new("wt.exe");
    wt.arg("-d").arg(dir);
    if !args.is_empty() {
        wt.arg("cmd").arg("/k").args(args);
    }
    if wt.creation_flags(CREATE_NO_WINDOW).spawn().is_ok() {
        return Ok(());
    }
    // `start` gives the console its own window; the directory goes through
    // current_dir, not the command line.
    let mut cmd = Command::new("cmd");
    cmd.args(["/c", "start", "", "cmd", "/k"]).args(args).current_dir(dir).creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().map(|_| ()).map_err(|e| format!("Couldn't open a terminal: {e}"))
}

/// Terminal in the most recent session's folder, else the home folder.
pub fn open_latest<R: Runtime>(app: &AppHandle<R>) {
    let dir = agents::latest_cwd(app).filter(|d| Path::new(d).is_dir()).or_else(home);
    if let Some(dir) = dir {
        if let Err(e) = launch(Path::new(&dir), &[]) {
            eprintln!("[terminal] {e}");
        }
    }
}

#[tauri::command]
pub fn open_terminal(app: AppHandle, cwd: Option<String>) -> Result<(), String> {
    let dir = match cwd {
        Some(c) if agents::is_session_cwd(&app, &c) => c,
        Some(_) => return Err("That folder isn't from a tracked session.".into()),
        None => agents::latest_cwd(&app).or_else(home).ok_or("No folder to open.")?,
    };
    if !Path::new(&dir).is_dir() {
        return Err(format!("Folder not found: {dir}"));
    }
    launch(Path::new(&dir), &[])
}

/// `claude --resume <id>` in a new terminal, in the session's folder.
#[tauri::command]
pub fn resume_session(app: AppHandle, id: String) -> Result<(), String> {
    let s = agents::is_session_id(&app, &id).ok_or("Session not found.")?;
    if s.agent != Agent::Claude {
        return Err("Only Claude Code sessions can be resumed from here.".into());
    }
    // Session ids are UUIDs; anything else never reaches the command line.
    if !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err("Unexpected session id.".into());
    }
    if !Path::new(&s.cwd).is_dir() {
        return Err(format!("Folder not found: {}", s.cwd));
    }
    launch(Path::new(&s.cwd), &["claude", "--resume", &id])
}
