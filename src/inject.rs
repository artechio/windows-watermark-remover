use std::ffi::c_void;

use windows::core::s;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::Memory::{
    VirtualProtectEx, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
};
use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowA, GetWindow, GetWindowInfo, SendMessageA, GW_CHILD, WINDOWINFO, WM_COMMAND,
    WS_VISIBLE,
};

use crate::constants::RET;
use crate::explorer::{close, explorer_pids, open_explorer, shell32_base};

/// Patch every running explorer.exe: write `ret` at shell32+rva.
pub unsafe fn inject(rva: u32) -> Result<(), String> {
    let pids = explorer_pids();
    if pids.is_empty() {
        return Err("explorer.exe is not running".into());
    }
    let mut any = false;
    for pid in pids {
        let handle = open_explorer(pid)?;
        let result = (|| -> Result<(), String> {
            let base = shell32_base(handle)?;
            let addr = (base + rva as u64) as *const c_void;
            let mut old = PAGE_PROTECTION_FLAGS(0);
            VirtualProtectEx(
                handle,
                addr as *mut c_void,
                RET.len(),
                PAGE_EXECUTE_READWRITE,
                &mut old,
            )
            .map_err(|e| e.to_string())?;
            WriteProcessMemory(
                handle,
                addr,
                RET.as_ptr() as *const c_void,
                RET.len(),
                None,
            )
            .map_err(|e| e.to_string())?;
            let mut tmp = PAGE_PROTECTION_FLAGS(0);
            let _ = VirtualProtectEx(handle, addr as *mut c_void, RET.len(), old, &mut tmp);
            Ok(())
        })();
        close(handle);
        result?;
        any = true;
    }
    if any {
        Ok(())
    } else {
        Err("no explorer process patched".into())
    }
}

pub unsafe fn refresh() {
    let progman = FindWindowA(s!("Progman"), s!("Program Manager"));
    let hwnd = GetWindow(progman, GW_CHILD);
    let hwnd2 = GetWindow(hwnd, GW_CHILD);
    let mut wi = WINDOWINFO::default();
    wi.cbSize = std::mem::size_of::<WINDOWINFO>() as u32;
    if GetWindowInfo(hwnd2, &mut wi as *mut _).is_ok() {
        let visible = wi.dwStyle & WS_VISIBLE == WS_VISIBLE;
        if visible {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
        } else {
            SendMessageA(hwnd, WM_COMMAND, WPARAM(0x7402), LPARAM::default());
            SendMessageA(hwnd, WM_COMMAND, WPARAM(0x7402), LPARAM::default());
        }
    } else {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}
