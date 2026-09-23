//! Optional logon re-apply: install a silent copy under LocalAppData and
//! register that fixed path in the current-user Run key (no admin needed).

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_READ, KEY_WRITE, REG_CREATE_KEY_DISPOSITION, REG_OPTION_NON_VOLATILE, REG_SZ,
    REG_VALUE_TYPE, RRF_RT_REG_SZ,
};

use crate::constants::{app_home, data_dir, installed_exe_path, RUN_VALUE_NAME};

const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn append_startup_log(line: &str) {
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("startup.log");
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", line);
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(aa), Ok(bb)) => aa == bb,
        _ => a == b,
    }
}

/// Copy the running exe into the fixed LocalAppData location (unless we already
/// are that file). Returns the installed path.
pub fn install_startup_exe() -> Result<PathBuf, String> {
    let dest = installed_exe_path();
    let src = env::current_exe().map_err(|e| e.to_string())?;

    fs::create_dir_all(app_home()).map_err(|e| {
        format!("Could not create the app folder under Local AppData: {e}")
    })?;

    if dest.exists() && paths_equal(&src, &dest) {
        append_startup_log(&format!(
            "Already running from installed copy: {}",
            dest.display()
        ));
        return Ok(dest);
    }

    // Prefer a replace that works even if an older copy is locked: write temp then rename.
    let tmp = dest.with_extension("exe.new");
    if tmp.exists() {
        let _ = fs::remove_file(&tmp);
    }
    fs::copy(&src, &tmp).map_err(|e| {
        format!(
            "Could not copy to {}: {e}",
            tmp.display()
        )
    })?;

    if dest.exists() {
        // Best-effort replace; if the old file is locked, keep the .new and use it in Run.
        match fs::remove_file(&dest) {
            Ok(()) => {
                fs::rename(&tmp, &dest).map_err(|e| {
                    format!("Could not install {}: {e}", dest.display())
                })?;
            }
            Err(err) => {
                append_startup_log(&format!(
                    "Could not replace old install ({err}); using {}",
                    tmp.display()
                ));
                // Point Run at the .new file as a fallback name — better: rename with unique.
                // Fall back: leave as APP_EXE_NAME.new won't look good. Try copy overwrite.
                let _ = fs::remove_file(&tmp);
                fs::copy(&src, &dest).map_err(|e| {
                    format!(
                        "Could not update installed copy at {}: {e}. Close any running copy and try again.",
                        dest.display()
                    )
                })?;
            }
        }
    } else {
        fs::rename(&tmp, &dest).map_err(|e| {
            format!("Could not install {}: {e}", dest.display())
        })?;
    }

    append_startup_log(&format!(
        "Installed silent copy as {}",
        dest.display()
    ));
    Ok(dest)
}

fn run_command_for_path(exe: &Path) -> String {
    // Quote the path; keep --apply outside the quotes so Windows passes it as argv.
    format!("\"{}\" --apply", exe.to_string_lossy())
}

pub fn is_logon_run_enabled() -> bool {
    let Some(value) = read_run_value() else {
        return false;
    };
    // Only treat as enabled if the registered file still exists.
    if let Some(path) = exe_path_from_run_value(&value) {
        if path.exists() {
            return true;
        }
        append_startup_log(&format!(
            "Startup entry points to missing file ({}); clearing.",
            path.display()
        ));
        let _ = clear_run_value();
        return false;
    }
    // Unparseable but present — still show as on; enable will repair.
    true
}

fn exe_path_from_run_value(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(PathBuf::from(&rest[..end]));
    }
    let path = trimmed
        .split_whitespace()
        .next()
        .unwrap_or(trimmed);
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn read_run_value() -> Option<String> {
    let mut subkey = wide(RUN_SUBKEY);
    let mut name = wide(RUN_VALUE_NAME);
    let mut data = vec![0u16; 1024];
    let mut data_size = (data.len() * 2) as u32;
    let mut data_type = REG_VALUE_TYPE::default();
    unsafe {
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_mut_ptr()),
            PCWSTR(name.as_mut_ptr()),
            RRF_RT_REG_SZ,
            Some(&mut data_type),
            Some(data.as_mut_ptr() as *mut _),
            Some(&mut data_size),
        );
        if status != ERROR_SUCCESS {
            return None;
        }
    }
    let chars = (data_size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&data[..chars]))
}

pub fn set_logon_run(enabled: bool) -> Result<(), String> {
    if enabled {
        enable_logon_run()
    } else {
        disable_logon_run()
    }
}

fn write_run_value(command: &str) -> Result<(), String> {
    let value: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
    let mut subkey = wide(RUN_SUBKEY);
    let mut name = wide(RUN_VALUE_NAME);

    unsafe {
        let mut hkey = Default::default();
        let mut disposition = REG_CREATE_KEY_DISPOSITION::default();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_mut_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            Some(&mut disposition),
        );
        if status != ERROR_SUCCESS {
            return Err(format!("Could not open startup settings ({status:?})"));
        }
        let bytes = (value.len() * 2) as u32;
        let set = RegSetValueExW(
            hkey,
            PCWSTR(name.as_mut_ptr()),
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                value.as_ptr() as *const u8,
                bytes as usize,
            )),
        );
        let _ = RegCloseKey(hkey);
        if set != ERROR_SUCCESS {
            return Err(format!("Could not save startup setting ({set:?})"));
        }
    }
    Ok(())
}

fn clear_run_value() -> Result<(), String> {
    let mut subkey = wide(RUN_SUBKEY);
    let mut name = wide(RUN_VALUE_NAME);
    unsafe {
        let mut hkey = Default::default();
        let mut disposition = REG_CREATE_KEY_DISPOSITION::default();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_mut_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            None,
            &mut hkey,
            Some(&mut disposition),
        );
        if status != ERROR_SUCCESS {
            return Err(format!("Could not open startup settings ({status:?})"));
        }
        let delete = RegDeleteValueW(hkey, PCWSTR(name.as_mut_ptr()));
        let _ = RegCloseKey(hkey);
        if delete != ERROR_SUCCESS && delete != ERROR_FILE_NOT_FOUND {
            return Err(format!("Could not clear startup setting ({delete:?})"));
        }
    }
    Ok(())
}

fn enable_logon_run() -> Result<(), String> {
    let dest = install_startup_exe()?;
    let command = run_command_for_path(&dest);
    append_startup_log(&format!("Enabling startup entry: {command}"));
    write_run_value(&command)?;
    Ok(())
}

fn disable_logon_run() -> Result<(), String> {
    append_startup_log("Disabling startup entry.");
    clear_run_value()?;

    let dest = installed_exe_path();
    if dest.exists() {
        match fs::remove_file(&dest) {
            Ok(()) => append_startup_log(&format!("Removed installed copy {}", dest.display())),
            Err(err) => append_startup_log(&format!(
                "Could not remove installed copy {}: {err}",
                dest.display()
            )),
        }
    }
    Ok(())
}

/// If sign-in reapply is on, refresh the LocalAppData copy from this exe and
/// point the Run key at it (repairs desktop/Downloads shortcuts that vanished).
pub fn refresh_startup_install_if_enabled() -> Result<(), String> {
    if read_run_value().is_none() {
        return Ok(());
    }
    let dest = install_startup_exe()?;
    let command = run_command_for_path(&dest);
    write_run_value(&command)?;
    append_startup_log(&format!("Refreshed startup entry: {command}"));
    Ok(())
}
