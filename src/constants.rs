use std::path::PathBuf;

use directories::ProjectDirs;

pub const SHELL32_PATH: &str = r"C:\Windows\System32\shell32.dll";
pub const EXPLORER_PATH: &str = r"C:\Windows\explorer.exe";
pub const RUN_VALUE_NAME: &str = "WindowsWatermarkRemover";

/// x86-64 near return. One byte is enough: the function never runs.
#[cfg(target_arch = "x86_64")]
pub const RET: [u8; 1] = [0xC3];

#[cfg(target_arch = "aarch64")]
pub const RET: [u8; 4] = [0xc0, 0x03, 0x1f, 0xd6];

pub fn data_dir() -> PathBuf {
    ProjectDirs::from("com", "artechio", "wwr")
        .expect("app data dir")
        .data_dir()
        .to_owned()
}
