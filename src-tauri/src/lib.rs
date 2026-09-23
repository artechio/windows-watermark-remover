mod log_bus;

#[cfg(windows)]
mod cache;
#[cfg(windows)]
mod constants;
#[cfg(windows)]
mod explorer;
#[cfg(windows)]
mod fetch_pdb;
#[cfg(windows)]
mod inject;
#[cfg(windows)]
mod parse_pdb;
#[cfg(windows)]
mod pe_guid;
#[cfg(windows)]
mod scan_dll;
#[cfg(windows)]
mod startup;
#[cfg(windows)]
mod structural_scan;

use log_bus::LogBus;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchResult {
    ok: bool,
    message: String,
}

#[tauri::command]
fn remove_watermark(app: AppHandle, logs: State<'_, LogBus>) -> PatchResult {
    logs.clear();
    let emit = |line: String| {
        logs.push(line.clone());
        let _ = app.emit("wwr-log", line);
    };

    #[cfg(not(windows))]
    {
        emit("This app only runs on Windows.".into());
        return PatchResult {
            ok: false,
            message: "Windows only".into(),
        };
    }

    #[cfg(windows)]
    {
        match run_patch(&emit, false) {
            Ok(()) => {
                emit("Done. The desktop watermark should be gone.".into());
                if let Err(err) = startup::refresh_startup_install_if_enabled() {
                    emit(format!("Could not refresh the sign-in copy: {err}"));
                }
                PatchResult {
                    ok: true,
                    message: "The evaluation watermark was removed from this session.".into(),
                }
            }
            Err(err) => {
                emit(format!("ERROR: {err}"));
                PatchResult {
                    ok: false,
                    message: err,
                }
            }
        }
    }
}

#[tauri::command]
fn get_logs(logs: State<'_, LogBus>) -> Vec<String> {
    logs.snapshot()
}

#[tauri::command]
fn get_startup_enabled() -> bool {
    #[cfg(windows)]
    {
        startup::is_logon_run_enabled()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[tauri::command]
fn set_startup_enabled(enabled: bool) -> Result<bool, String> {
    #[cfg(windows)]
    {
        startup::set_logon_run(enabled)?;
        // Re-read so a missing/stale install is reported accurately.
        Ok(startup::is_logon_run_enabled())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("Startup option is only available on Windows.".into())
    }
}

#[cfg(windows)]
fn run_patch(emit: &dyn Fn(String), startup_mode: bool) -> Result<(), String> {
    emit("Waiting for the desktop shell…".into());
    if startup_mode {
        explorer::wait_for_desktop(emit);
    } else {
        explorer::wait_for_explorer();
    }

    emit("Checking this Windows build…".into());
    let cache_key = pe_guid::shell32_cache_key(emit)?;

    emit("Finding the watermark painter…".into());
    let rva = cache::get_rva(&cache_key, emit)?;
    emit("Watermark painter located.".into());

    if startup_mode {
        // Explorer often restarts during/after sign-in. Watch and re-apply.
        run_startup_watch(emit, rva)
    } else {
        emit("Applying the fix in memory…".into());
        unsafe {
            inject::inject(rva, emit)?;
            emit("Refreshing the desktop…".into());
            inject::refresh();
        }
        Ok(())
    }
}

/// Keep re-applying for a few minutes whenever Explorer's process set changes.
#[cfg(windows)]
fn run_startup_watch(emit: &dyn Fn(String), rva: u32) -> Result<(), String> {
    use std::time::{Duration, Instant};

    let watch_for = Duration::from_secs(150);
    let poll = Duration::from_secs(4);
    let deadline = Instant::now() + watch_for;
    let mut last_pids: Vec<u32> = Vec::new();
    let mut any_ok = false;
    let mut last_err: Option<String> = None;
    let mut attempt: u32 = 0;

    emit(format!(
        "Watching the desktop shell for {} seconds…",
        watch_for.as_secs()
    ));

    while Instant::now() < deadline {
        let mut pids = explorer::explorer_pids();
        pids.sort_unstable();

        let changed = pids != last_pids;
        if !pids.is_empty() && (changed || attempt == 0) {
            attempt += 1;
            emit(format!(
                "Applying the fix (attempt {attempt}, {} shell process(es))…",
                pids.len()
            ));
            match unsafe {
                inject::inject(rva, emit).map(|_| {
                    inject::refresh();
                })
            } {
                Ok(()) => {
                    any_ok = true;
                    last_err = None;
                    emit("Fix applied. Waiting in case Explorer restarts…".into());
                }
                Err(err) => {
                    emit(format!("Attempt {attempt} failed: {err}"));
                    last_err = Some(err);
                }
            }
            last_pids = pids;
        } else if pids.is_empty() {
            emit("Desktop shell not running yet — waiting…".into());
            last_pids.clear();
        }

        std::thread::sleep(poll);
    }

    if any_ok {
        emit("Startup re-apply finished.".into());
        Ok(())
    } else {
        Err(last_err.unwrap_or_else(|| {
            "Could not apply the fix after sign-in. The desktop shell may not have been ready."
                .into()
        }))
    }
}

#[tauri::command]
fn should_auto_apply() -> bool {
    should_auto_apply_flag()
}

fn should_auto_apply_flag() -> bool {
    std::env::args().any(|arg| {
        let lower = arg.to_ascii_lowercase();
        lower == "--apply" || lower == "/apply" || lower == "-apply"
    })
}

/// Headless sign-in path: no WebView/Tauri UI (more reliable at logon).
/// Returns `Some(exit_code)` when `--apply` was handled; `None` to open the UI.
#[cfg(windows)]
pub fn try_run_startup_apply() -> Option<i32> {
    if !should_auto_apply_flag() {
        return None;
    }

    startup::append_startup_log(&format!(
        "Headless apply launch args: {:?}",
        std::env::args().collect::<Vec<_>>()
    ));

    // Brief settle before the first look at Explorer.
    std::thread::sleep(std::time::Duration::from_secs(3));

    let emit = |line: String| {
        startup::append_startup_log(&line);
    };

    let result = run_patch(&emit, true);
    match &result {
        Ok(()) => {
            emit("Done. The desktop watermark should be gone.".into());
            startup::append_startup_log("Auto-apply finished ok=true");
            Some(0)
        }
        Err(err) => {
            emit(format!("ERROR: {err}"));
            startup::append_startup_log(&format!("Auto-apply finished ok=false message={err}"));
            Some(1)
        }
    }
}

#[cfg(not(windows))]
pub fn try_run_startup_apply() -> Option<i32> {
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Interactive UI only — startup re-apply is handled in main via try_run_startup_apply.
    tauri::Builder::default()
        .manage(LogBus::default())
        .invoke_handler(tauri::generate_handler![
            remove_watermark,
            get_logs,
            get_startup_enabled,
            set_startup_enabled,
            should_auto_apply
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
