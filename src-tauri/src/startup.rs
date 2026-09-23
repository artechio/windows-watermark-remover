//! Optional logon re-apply under the current user Run key.

use std::env;
use std::fs;
use std::io::Write;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_READ, KEY_WRITE, REG_CREATE_KEY_DISPOSITION, REG_OPTION_NON_VOLATILE, REG_SZ,
    REG_VALUE_TYPE, RRF_RT_REG_SZ,
};

use crate::constants::{data_dir, RUN_VALUE_NAME};

const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn run_command_line() -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    // Quote the path; keep --apply outside the quotes so Windows passes it as argv.
    Ok(format!("\"{}\" --apply", exe.to_string_lossy()))
}

pub fn append_startup_log(line: &str) {
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("startup.log");
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", line);
    }
}

pub fn is_logon_run_enabled() -> bool {
    read_run_value().is_some()
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

fn enable_logon_run() -> Result<(), String> {
    let command = run_command_line()?;
    append_startup_log(&format!("Enabling startup entry: {command}"));
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
            Some(std::slice::from_raw_parts(value.as_ptr() as *const u8, bytes as usize)),
        );
        let _ = RegCloseKey(hkey);
        if set != ERROR_SUCCESS {
            return Err(format!("Could not save startup setting ({set:?})"));
        }
    }
    Ok(())
}

fn disable_logon_run() -> Result<(), String> {
    append_startup_log("Disabling startup entry.");
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
