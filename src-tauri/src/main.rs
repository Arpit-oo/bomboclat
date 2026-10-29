// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if bomboclat_lib::run_hook_if_requested() {
        return;
    }
    bomboclat_lib::run()
}
