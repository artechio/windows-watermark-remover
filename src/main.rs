//! Silent Insider evaluation watermark remover (UWD2-style).
//!
//! Finds CDesktopWatermark::s_DesktopBuildPaint in shell32 and writes a `ret`
//! into the running explorer.exe process. No window, no prompts.

#![windows_subsystem = "windows"]

mod cache;
mod constants;
mod explorer;
mod fetch_pdb;
mod inject;
mod parse_pdb;
mod pe_guid;
mod scan_dll;
mod startup;
mod structural_scan;

fn main() {
    let code = match run() {
        Ok(()) => 0,
        Err(_) => 1,
    };
    std::process::exit(code);
}

fn run() -> Result<(), String> {
    explorer::wait_for_explorer();
    let guid = pe_guid::shell32_pdb_guid()?;
    let rva = cache::get_rva(&guid)?;
    unsafe {
        inject::inject(rva)?;
        inject::refresh();
    }
    let _ = startup::ensure_logon_run();
    Ok(())
}
