use std::path::PathBuf;

use directories::ProjectDirs;

pub const SHELL32_PATH: &str = r"C:\Windows\System32\shell32.dll";
pub const EXPLORER_PATH: &str = r"C:\Windows\explorer.exe";

/// Display name in Task Manager → Startup and the installed file name.
pub const RUN_VALUE_NAME: &str = "Windows Watermark Remover";
pub const APP_EXE_NAME: &str = "Windows Watermark Remover.exe";
pub const APP_FOLDER_NAME: &str = "Windows Watermark Remover";

/// x86-64 near return. One byte is enough: the function never runs.
#[cfg(target_arch = "x86_64")]
pub const RET: [u8; 1] = [0xC3];

#[cfg(target_arch = "aarch64")]
pub const RET: [u8; 4] = [0xc0, 0x03, 0x1f, 0xd6];

/// Fixed per-user folder under LocalAppData (no admin). Holds the silent
/// startup copy, cache, and logs.
pub fn app_home() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local).join(APP_FOLDER_NAME);
    }
    ProjectDirs::from("com", "artechio", APP_FOLDER_NAME)
        .expect("app data dir")
        .data_local_dir()
        .to_owned()
}

pub fn data_dir() -> PathBuf {
    app_home()
}

pub fn installed_exe_path() -> PathBuf {
    app_home().join(APP_EXE_NAME)
}
