mod agents;
mod hooks;
mod input;
mod models;
mod reminders;
mod settings;
mod terminal;
mod tray;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_window_state::StateFlags;

use settings::SettingsState;

/// Ctrl+Shift+Alt+C toggles the cat. Plain Ctrl+Shift+C would steal
/// DevTools-inspect and terminal copy from every other app.
fn toggle_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT), Code::KeyC)
}

/// `bomboclat --settings` / `--sessions` open those windows, also when the
/// app is already running.
fn open_requested<R: tauri::Runtime>(app: &tauri::AppHandle<R>, args: &[String]) {
    if args.iter().any(|a| a == "--settings") {
        tray::open_settings(app);
    }
    if args.iter().any(|a| a == "--sessions") {
        tray::open_sessions(app);
    }
}

#[tauri::command]
fn frontend_log(msg: String) {
    eprintln!("[webview] {msg}");
}

/// `bomboclat.exe --hook-event`: record a Claude Code notification and exit
/// without starting the app. See hooks.rs.
pub fn run_hook_if_requested() -> bool {
    if hooks::is_hook_invocation() {
        hooks::hook_event_main();
        return true;
    }
    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            tray::set_visible(app, true);
            open_requested(app, &args);
        }))
        .plugin(tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::POSITION)
                .with_denylist(&["settings"])
                .build())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("bcmodel", |ctx, req| models::serve(ctx.app_handle(), &req))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed && *shortcut == toggle_shortcut() {
                        tray::toggle_visible(app);
                    }
                })
                .build())
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::update_settings,
            settings::reset_settings,
            tray::get_autostart,
            tray::show_menu,
            tray::open_sessions_window,
            agents::list_sessions,
            hooks::claude_hook_status,
            hooks::set_claude_hook,
            terminal::open_terminal,
            terminal::resume_session,
            tray::set_autostart_cmd,
            frontend_log,
            models::list_models,
            models::import_model,
            models::remove_model,
            reminders::start_focus_cmd,
            reminders::stop_focus_cmd,
            reminders::test_bubble,
        ])
        .setup(|app| {
            let handle = app.handle();
            let mut s = settings::load(handle);
            if s.model != models::PIXEL && models::find(handle, &s.model).is_none() {
                // Model was deleted or BongoCat uninstalled.
                s.model = models::PIXEL.into();
            }
            app.manage(SettingsState(std::sync::Mutex::new(s.clone())));
            app.manage(reminders::PomodoroState::default());
            app.manage(agents::SessionsState::default());
            settings::apply(handle, &s);
            tray::build(handle)?;
            if let Err(e) = app.global_shortcut().register(toggle_shortcut()) {
                eprintln!("[shortcut] register failed: {e}");
            }
            input::spawn(handle.clone());
            agents::spawn(handle.clone());
            reminders::spawn(handle.clone());
            open_requested(handle, &std::env::args().collect::<Vec<_>>());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running bomboclat");
}
