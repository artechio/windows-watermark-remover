use std::ffi::c_void;
use std::mem::size_of;
use std::path::Path;

use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use windows::Win32::Foundation::{CloseHandle, FALSE, HANDLE, HMODULE};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::ProcessStatus::{
    EnumProcessModulesEx, GetModuleFileNameExW, LIST_MODULES_ALL,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

use crate::constants::{EXPLORER_PATH, SHELL32_PATH};

pub fn wait_for_explorer() {
    for _ in 0..60 {
        if !explorer_pids().is_empty() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

pub fn explorer_pids() -> Vec<u32> {
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let target = Path::new(EXPLORER_PATH);
    sys.processes()
        .values()
        .filter_map(|proc| {
            let exe = proc.exe()?;
            if exe.to_string_lossy().eq_ignore_ascii_case(&target.to_string_lossy()) {
                Some(proc.pid().as_u32())
            } else {
                None
            }
        })
        .collect()
}

pub unsafe fn open_explorer(pid: u32) -> Result<HANDLE, String> {
    OpenProcess(
        PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
        FALSE,
        pid,
    )
    .map_err(|e| e.to_string())
}

pub unsafe fn shell32_base(handle: HANDLE) -> Result<u64, String> {
    let mut modules = [HMODULE::default(); 1024];
    let mut needed = 0u32;
    EnumProcessModulesEx(
        handle,
        modules.as_mut_ptr(),
        (modules.len() * size_of::<HMODULE>()) as u32,
        &mut needed,
        LIST_MODULES_ALL,
    )
    .map_err(|e| e.to_string())?;
    let count = (needed as usize) / size_of::<HMODULE>();
    let mut path_buf = [0u16; 260];
    for module in modules.iter().take(count) {
        let n = GetModuleFileNameExW(handle, *module, &mut path_buf);
        if n == 0 {
            continue;
        }
        let path = String::from_utf16_lossy(&path_buf[..n as usize]);
        if path.eq_ignore_ascii_case(SHELL32_PATH) {
            return Ok(module.0 as u64);
        }
    }
    Err("shell32.dll not mapped in explorer".into())
}

pub unsafe fn verify_rva(handle: HANDLE, base: u64, rva: u32, expected: &[u8]) -> bool {
    let mut buf = vec![0u8; expected.len()];
    let mut read = 0usize;
    let ok = ReadProcessMemory(
        handle,
        (base + rva as u64) as *const c_void,
        buf.as_mut_ptr() as *mut c_void,
        buf.len(),
        Some(&mut read),
    );
    ok.is_ok() && read == expected.len() && buf == expected
}

pub unsafe fn close(handle: HANDLE) {
    let _ = CloseHandle(handle);
}
