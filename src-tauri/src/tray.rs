//! System tray icon and menu, plus the window/autostart helpers it drives.

use serde_json::json;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

use crate::settings::{self, Settings, OPACITY_STEPS, SIZE_STEPS};

pub struct TrayItems<R: Runtime> {
    show: CheckMenuItem<R>,
    click_through: CheckMenuItem<R>,
    on_top: CheckMenuItem<R>,
    flip: CheckMenuItem<R>,
    autostart: CheckMenuItem<R>,
    focus: MenuItem<R>,
    sizes: Vec<(u32, CheckMenuItem<R>)>,
    opacities: Vec<(u32, CheckMenuItem<R>)>,
    /// Shared by the tray icon and the right-click menu on the cat; the
    /// tray's menu handler is global, so both route through `on_menu`.
    menu: Menu<R>,
}

/// Menu items before the menu exists; becomes [`TrayItems`].
struct Items<R: Runtime> {
    show: CheckMenuItem<R>,
    click_through: CheckMenuItem<R>,
    on_top: CheckMenuItem<R>,
    flip: CheckMenuItem<R>,
    autostart: CheckMenuItem<R>,
    focus: MenuItem<R>,
    sizes: Vec<(u32, CheckMenuItem<R>)>,
    opacities: Vec<(u32, CheckMenuItem<R>)>,
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let s = settings::current(app);
    let check = |id: &str, text: &str, on: bool| CheckMenuItem::with_id(app, id, text, true, on, None::<&str>);

    let items = Items {
        show: check("show", "Show cat", true)?,
        click_through: check("click_through", "Click-through", s.click_through)?,
        on_top: check("on_top", "Always on top", s.always_on_top)?,
        flip: check("flip", "Flip", s.flip)?,
        autostart: check("autostart", "Launch at startup", autostart_enabled(app))?,
        focus: MenuItem::with_id(app, "focus", FOCUS_START, true, None::<&str>)?,
        sizes: SIZE_STEPS
            .iter()
            .map(|&n| Ok((n, check(&format!("size:{n}"), &format!("{n}%"), s.size == n)?)))
            .collect::<tauri::Result<_>>()?,
        opacities: OPACITY_STEPS
            .iter()
            .map(|&n| Ok((n, check(&format!("opacity:{n}"), &format!("{n}%"), s.opacity == n)?)))
            .collect::<tauri::Result<_>>()?,
    };

    let size_items: Vec<_> = items.sizes.iter().map(|(_, i)| i as &dyn tauri::menu::IsMenuItem<R>).collect();
    let opacity_items: Vec<_> = items.opacities.iter().map(|(_, i)| i as &dyn tauri::menu::IsMenuItem<R>).collect();
    let size_menu = Submenu::with_items(app, "Size", true, &size_items)?;
    let opacity_menu = Submenu::with_items(app, "Opacity", true, &opacity_items)?;
    let sessions_item = MenuItem::with_id(app, "sessions", "Claude sessions…", true, None::<&str>)?;
    let terminal_item = MenuItem::with_id(app, "terminal", "Open terminal", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Bomboclat", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);

    let menu = Menu::with_items(
        app,
        &[
            &sessions_item,
            &terminal_item,
            &sep()?,
            &settings_item,
            &sep()?,
            &items.show,
            &sep()?,
            &items.click_through,
            &items.on_top,
            &items.flip,
            &size_menu,
            &opacity_menu,
            &sep()?,
            &items.focus,
            &sep()?,
            &items.autostart,
            &sep()?,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Bomboclat")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_visible(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    app.manage(TrayItems {
        show: items.show,
        click_through: items.click_through,
        on_top: items.on_top,
        flip: items.flip,
        autostart: items.autostart,
        focus: items.focus,
        sizes: items.sizes,
        opacities: items.opacities,
        menu,
    });
    Ok(())
}

fn on_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let s = settings::current(app);
    let patch = match id {
        "show" => return toggle_visible(app),
        "autostart" => return set_autostart(app, !autostart_enabled(app)),
        "settings" => return open_settings(app),
        "sessions" => return open_sessions(app),
        "terminal" => return crate::terminal::open_latest(app),
        "quit" => return app.exit(0),
        "focus" => {
            return if crate::reminders::focus_running(app) {
                crate::reminders::stop_focus(app)
            } else {
                crate::reminders::start_focus(app)
            }
        }
        "click_through" => json!({ "clickThrough": !s.click_through }),
        "on_top" => json!({ "alwaysOnTop": !s.always_on_top }),
        "flip" => json!({ "flip": !s.flip }),
        _ => {
            if let Some(n) = id.strip_prefix("size:").and_then(|n| n.parse::<u32>().ok()) {
                json!({ "size": n })
            } else if let Some(n) = id.strip_prefix("opacity:").and_then(|n| n.parse::<u32>().ok()) {
                json!({ "opacity": n })
            } else {
                return;
            }
        }
    };
    settings::update(app, patch);
}

/// Re-checks tray items from settings. Muda flips a check item on click, so
/// this also undoes that for radio-style groups.
pub fn sync<R: Runtime>(app: &AppHandle<R>, s: &Settings) {
    let Some(items) = app.try_state::<TrayItems<R>>() else { return };
    let _ = items.click_through.set_checked(s.click_through);
    let _ = items.on_top.set_checked(s.always_on_top);
    let _ = items.flip.set_checked(s.flip);
    for (n, item) in &items.sizes {
        let _ = item.set_checked(*n == s.size);
    }
    for (n, item) in &items.opacities {
        let _ = item.set_checked(*n == s.opacity);
    }
}

const FOCUS_START: &str = "Start focus timer";
const FOCUS_STOP: &str = "Stop focus timer";

pub fn set_focus_running<R: Runtime>(app: &AppHandle<R>, running: bool) {
    if let Some(items) = app.try_state::<TrayItems<R>>() {
        let _ = items.focus.set_text(if running { FOCUS_STOP } else { FOCUS_START });
    }
}

pub fn set_agent_tooltip<R: Runtime>(app: &AppHandle<R>, working: usize, input: usize) {
    let mut parts = Vec::new();
    if input > 0 {
        parts.push(format!("{input} waiting for you"));
    }
    if working > 0 {
        parts.push(format!("{working} working"));
    }
    let tip = if parts.is_empty() { "Bomboclat".to_string() } else { format!("Bomboclat · {}", parts.join(", ")) };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tip));
    }
}

pub fn set_visible<R: Runtime>(app: &AppHandle<R>, visible: bool) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = if visible { w.show() } else { w.hide() };
    }
    if let Some(items) = app.try_state::<TrayItems<R>>() {
        let _ = items.show.set_checked(visible);
    }
}

pub fn toggle_visible<R: Runtime>(app: &AppHandle<R>) {
    let visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    set_visible(app, !visible);
}

pub fn open_settings<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Bomboclat Settings")
        .inner_size(600.0, 720.0)
        .min_inner_size(520.0, 560.0)
        .center()
        .build();
    if let Err(e) = built {
        eprintln!("[tray] settings window failed: {e}");
    }
}

pub fn open_sessions<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("sessions") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, "sessions", WebviewUrl::App("sessions.html".into()))
        .title("Bomboclat · Claude sessions")
        .inner_size(980.0, 640.0)
        .min_inner_size(640.0, 420.0)
        .center()
        .build();
    if let Err(e) = built {
        eprintln!("[tray] sessions window failed: {e}");
    }
}

#[tauri::command]
pub fn open_sessions_window(app: AppHandle) {
    open_sessions(&app);
}

fn autostart_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

fn set_autostart<R: Runtime>(app: &AppHandle<R>, on: bool) {
    let mgr = app.autolaunch();
    let res = if on { mgr.enable() } else { mgr.disable() };
    if let Err(e) = res {
        eprintln!("[tray] autostart toggle failed: {e}");
    }
    if let Some(items) = app.try_state::<TrayItems<R>>() {
        let _ = items.autostart.set_checked(autostart_enabled(app));
    }
}

/// Right-click on the cat: the tray menu, at the cursor.
#[tauri::command]
pub fn show_menu(app: AppHandle, window: tauri::WebviewWindow) {
    if let Some(items) = app.try_state::<TrayItems<tauri::Wry>>() {
        if let Err(e) = window.popup_menu(&items.menu) {
            eprintln!("[tray] context menu failed: {e}");
        }
    }
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    autostart_enabled(&app)
}

#[tauri::command]
pub fn set_autostart_cmd(app: AppHandle, on: bool) -> bool {
    set_autostart(&app, on);
    autostart_enabled(&app)
}
