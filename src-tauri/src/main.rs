// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Sign-in re-apply must not depend on WebView2 / the GUI window.
    // At logon, Explorer often restarts; a headless watcher is more reliable.
    if let Some(code) = wwr_lib::try_run_startup_apply() {
        std::process::exit(code);
    }
    wwr_lib::run()
}
