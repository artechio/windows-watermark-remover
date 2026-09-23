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
    rva: Option<String>,
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
        emit("This app only patches Windows Explorer.".into());
        return PatchResult {
            ok: false,
            message: "Windows only".into(),
            rva: None,
        };
    }

    #[cfg(windows)]
    {
        match run_patch(&emit) {
            Ok(rva) => {
                emit(format!("Done. Patched s_DesktopBuildPaint at RVA {rva:#x}."));
                PatchResult {
                    ok: true,
                    message: "Watermark paint function disabled in Explorer memory.".into(),
                    rva: Some(format!("{rva:#x}")),
                }
            }
            Err(err) => {
                emit(format!("ERROR: {err}"));
                PatchResult {
                    ok: false,
                    message: err,
                    rva: None,
                }
            }
        }
    }
}

#[tauri::command]
fn get_logs(logs: State<'_, LogBus>) -> Vec<String> {
    logs.snapshot()
}

#[cfg(windows)]
fn run_patch(emit: &dyn Fn(String)) -> Result<u32, String> {
    emit("Waiting for explorer.exe…".into());
    explorer::wait_for_explorer();

    emit(format!("Reading PDB identity from {}", constants::SHELL32_PATH));
    let guid = pe_guid::shell32_pdb_guid()?;
    emit(format!("shell32 PDB id: {guid}"));

    emit("Resolving CDesktopWatermark::s_DesktopBuildPaint…".into());
    let rva = cache::get_rva(&guid, emit)?;
    emit(format!("Resolved RVA {rva:#x}"));

    emit("Writing ret into explorer.exe memory…".into());
    unsafe {
        inject::inject(rva, emit)?;
        emit("Refreshing desktop shell…".into());
        inject::refresh();
    }

    match startup::ensure_logon_run() {
        Ok(()) => emit("Registered silent re-run at user logon (HKCU Run).".into()),
        Err(e) => emit(format!("Logon Run registration skipped: {e}")),
    }

    Ok(rva)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(LogBus::default())
        .invoke_handler(tauri::generate_handler![remove_watermark, get_logs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
