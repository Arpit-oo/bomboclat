//! Live2D model catalogue. Two sources:
//! - BongoCat's bundled models, read in place from its install dir (never copied).
//! - Models the user imports, copied into `<app data>/models/<name>`.
//!
//! Files reach the webview through the `bcmodel` protocol
//! (`http://bcmodel.localhost/<id>/<path>`), which only serves files inside a
//! known model root.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Runtime};

/// The built-in pixel cat; not a Live2D model.
pub const PIXEL: &str = "pixel";
/// The built-in pixel Claude crab; drawn by the same renderer as the cat.
pub const CRAB: &str = "crab";

/// True for the pixel characters that ship with the app.
pub fn is_builtin(id: &str) -> bool {
    id == PIXEL || id == CRAB
}

/// Live2D window size relative to the model's background image at 100%.
const MODEL_WINDOW_SCALE: f64 = 0.5;
const BONGOCAT_PREFIX: &str = "bongocat-";
const USER_PREFIX: &str = "user-";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    /// "bongocat" or "imported".
    pub source: &'static str,
    /// Path of the `.model3.json`, relative to the model root.
    pub model_file: String,
    pub has_background: bool,
    pub has_cover: bool,
    /// Background image size; the window keeps this aspect ratio.
    pub width: u32,
    pub height: u32,
    /// Key image names (file stems) in `resources/left-keys` and `right-keys`.
    pub left_keys: Vec<String>,
    pub right_keys: Vec<String>,
}

fn bongocat_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
        .join("BongoCat")
        .join("resources")
        .join("models");
    dir.is_dir().then_some(dir)
}

fn user_dir<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("models"))
}

fn png_size(path: &Path) -> Option<(u32, u32)> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < 24 || &bytes[1..4] != b"PNG" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

fn key_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
}

/// The `.model3.json` in `root`, preferring BongoCat's `cat.model3.json`.
fn model_file(root: &Path) -> Option<String> {
    if root.join("cat.model3.json").is_file() {
        return Some("cat.model3.json".into());
    }
    fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.ends_with(".model3.json"))
}

fn describe(id: String, name: String, root: &Path, source: &'static str) -> Option<ModelInfo> {
    let model_file = model_file(root)?;
    let res = root.join("resources");
    let background = res.join("background.png");
    let (width, height) = png_size(&background).unwrap_or((612, 354));
    Some(ModelInfo {
        id,
        name,
        source,
        model_file,
        has_background: background.is_file(),
        has_cover: res.join("cover.png").is_file(),
        width,
        height,
        left_keys: key_names(&res.join("left-keys")),
        right_keys: key_names(&res.join("right-keys")),
    })
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

fn subdirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut dirs: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    dirs.sort();
    dirs
}

pub fn list<R: Runtime>(app: &AppHandle<R>) -> Vec<ModelInfo> {
    let mut out = Vec::new();
    if let Some(dir) = bongocat_dir() {
        for (name, path) in subdirs(&dir) {
            let label = format!("BongoCat · {}", title_case(&name));
            out.extend(describe(format!("{BONGOCAT_PREFIX}{name}"), label, &path, "bongocat"));
        }
    }
    if let Some(dir) = user_dir(app) {
        for (name, path) in subdirs(&dir) {
            out.extend(describe(format!("{USER_PREFIX}{name}"), name, &path, "imported"));
        }
    }
    out
}

/// Root directory of a model id. Ids map to a single path component, so an id
/// can never escape its source directory.
fn root<R: Runtime>(app: &AppHandle<R>, id: &str) -> Option<PathBuf> {
    let (base, name) = if let Some(n) = id.strip_prefix(BONGOCAT_PREFIX) {
        (bongocat_dir()?, n)
    } else {
        (user_dir(app)?, id.strip_prefix(USER_PREFIX)?)
    };
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return None;
    }
    let dir = base.join(name);
    dir.is_dir().then_some(dir)
}

pub fn find<R: Runtime>(app: &AppHandle<R>, id: &str) -> Option<ModelInfo> {
    list(app).into_iter().find(|m| m.id == id)
}

/// Window size at 100% for a model, in logical pixels.
pub fn base_size<R: Runtime>(app: &AppHandle<R>, id: &str) -> Option<(f64, f64)> {
    if is_builtin(id) {
        return None;
    }
    let m = find(app, id)?;
    Some((m.width as f64 * MODEL_WINDOW_SCALE, m.height as f64 * MODEL_WINDOW_SCALE))
}

fn percent_decode(s: &str) -> String {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(hi), Some(lo)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("flac") => "audio/flac",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

fn respond(status: StatusCode, mime: &str, body: Vec<u8>) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        // The page lives on another origin (dev server / tauri.localhost) and
        // WebGL textures need CORS-clean images.
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Cow::Owned(body))
        .unwrap()
}

/// Handler for `bcmodel://localhost/<id>/<relative path>`.
pub fn serve<R: Runtime>(app: &AppHandle<R>, req: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let not_found = || respond(StatusCode::NOT_FOUND, "text/plain", b"not found".to_vec());
    let path = percent_decode(req.uri().path());
    let mut parts = path.trim_start_matches('/').splitn(2, '/');
    let (Some(id), Some(rel)) = (parts.next(), parts.next()) else { return not_found() };
    let Some(root) = root(app, id) else { return not_found() };
    let (Ok(root), Ok(file)) = (root.canonicalize(), root.join(rel).canonicalize()) else {
        return not_found();
    };
    if !file.starts_with(&root) || !file.is_file() {
        return not_found();
    }
    match fs::read(&file) {
        Ok(bytes) => respond(StatusCode::OK, mime(&file), bytes),
        Err(_) => not_found(),
    }
}

/// Finds the folder holding a `.model3.json` at most three levels down, since
/// downloaded models are often wrapped in an extra folder.
fn locate_model_root(dir: &Path, depth: u32) -> Option<PathBuf> {
    if model_file(dir).is_some() {
        return Some(dir.to_path_buf());
    }
    if depth == 0 {
        return None;
    }
    subdirs(dir).into_iter().find_map(|(_, p)| locate_model_root(&p, depth - 1))
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".DS_Store" {
            continue;
        }
        let src = entry.path();
        if src.is_dir() {
            copy_dir(&src, &to.join(&name))?;
        } else {
            fs::copy(&src, to.join(&name))?;
        }
    }
    Ok(())
}

fn import_from<R: Runtime>(app: &AppHandle<R>, picked: &Path) -> Result<ModelInfo, String> {
    let src = locate_model_root(picked, 3).ok_or("No .model3.json found in that folder.")?;
    let dest_root = user_dir(app).ok_or("No app data folder.")?;
    let base = picked
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "model".into());
    let mut name = base.clone();
    let mut n = 2;
    while dest_root.join(&name).exists() {
        name = format!("{base} ({n})");
        n += 1;
    }
    let dest = dest_root.join(&name);
    if let Err(e) = copy_dir(&src, &dest) {
        let _ = fs::remove_dir_all(&dest);
        return Err(format!("Copy failed: {e}"));
    }
    describe(format!("{USER_PREFIX}{name}"), name, &dest, "imported").ok_or("Import failed.".into())
}

#[tauri::command]
pub fn list_models(app: AppHandle) -> Vec<ModelInfo> {
    list(&app)
}

/// Opens a folder picker and imports the chosen model. `Ok(None)` = cancelled.
#[tauri::command]
pub async fn import_model(app: AppHandle) -> Result<Option<ModelInfo>, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(picked) = app.dialog().file().set_title("Choose a Live2D model folder").blocking_pick_folder()
    else {
        return Ok(None);
    };
    let picked = picked.into_path().map_err(|e| e.to_string())?;
    import_from(&app, &picked).map(Some)
}

#[tauri::command]
pub fn remove_model(app: AppHandle, id: String) -> Result<(), String> {
    if !id.starts_with(USER_PREFIX) {
        return Err("Only imported models can be removed.".into());
    }
    let dir = root(&app, &id).ok_or("Model not found.")?;
    fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    if crate::settings::current(&app).model == id {
        crate::settings::update(&app, serde_json::json!({ "model": PIXEL }));
    }
    Ok(())
}
