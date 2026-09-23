//! Persist a silent logon re-patch under the current user Run key.

use std::env;

use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_WRITE,
    REG_CREATE_KEY_DISPOSITION, REG_OPTION_NON_VOLATILE, REG_SZ,
};

use crate::constants::RUN_VALUE_NAME;

pub fn ensure_logon_run() -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy();
    let quoted = format!("\"{exe}\"");
    let value: Vec<u16> = quoted.encode_utf16().chain(std::iter::once(0)).collect();
    let mut subkey: Vec<u16> = r"Software\Microsoft\Windows\CurrentVersion\Run"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut name: Vec<u16> = RUN_VALUE_NAME.encode_utf16().chain(std::iter::once(0)).collect();

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
            return Err(format!("RegCreateKeyExW failed: {status:?}"));
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
            return Err(format!("RegSetValueExW failed: {set:?}"));
        }
    }
    Ok(())
}
