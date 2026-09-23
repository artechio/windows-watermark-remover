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
use tauri::{AppHandle, Emitter, Manager, State};

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
        match run_patch(&emit) {
            Ok(()) => {
                emit("Done. The desktop watermark should be gone.".into());
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
        Ok(enabled)
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("Startup option is only available on Windows.".into())
    }
}

#[cfg(windows)]
fn run_patch(emit: &dyn Fn(String)) -> Result<(), String> {
    emit("Waiting for the desktop shell…".into());
    explorer::wait_for_explorer();

    emit("Checking this Windows build…".into());
    let cache_key = pe_guid::shell32_cache_key(emit)?;

    emit("Finding the watermark painter…".into());
    let rva = cache::get_rva(&cache_key, emit)?;
    emit("Watermark painter located.".into());

    emit("Applying the fix in memory…".into());
    unsafe {
        inject::inject(rva, emit)?;
        emit("Refreshing the desktop…".into());
        inject::refresh();
    }

    Ok(())
}

#[tauri::command]
fn should_auto_apply() -> bool {
    should_auto_apply_flag()
}

fn should_auto_apply_flag() -> bool {
    std::env::args().any(|arg| arg == "--apply")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let auto_apply = should_auto_apply_flag();
    tauri::Builder::default()
        .manage(LogBus::default())
        .invoke_handler(tauri::generate_handler![
            remove_watermark,
            get_logs,
            get_startup_enabled,
            set_startup_enabled,
            should_auto_apply
        ])
        .setup(move |app| {
            if auto_apply {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    let logs = handle.state::<LogBus>();
                    let result = remove_watermark(handle.clone(), logs);
                    let _ = handle.emit("wwr-auto-apply", result);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
