use std::fs;
use std::path::Path;

use crate::constants::data_dir;
use crate::explorer::{close, explorer_pids, open_explorer, shell32_base, verify_rva};
use crate::fetch_pdb;
use crate::parse_pdb::parse_pdb;
use crate::scan_dll;
use crate::structural_scan;

/// Resolve CDesktopWatermark::s_DesktopBuildPaint for the current shell32.
///
/// Order: cached RVA → Microsoft PDB → saved byte patterns → structural scan.
pub fn get_rva(guid: &str) -> Result<u32, String> {
    let dir = data_dir();
    let rva_path = dir.join(format!("{guid}.rva"));

    if rva_path.exists() {
        let file = fs::read(&rva_path).map_err(|e| e.to_string())?;
        let bytes: [u8; 4] = file
            .try_into()
            .map_err(|_| "corrupt RVA cache".to_string())?;
        return Ok(u32::from_be_bytes(bytes));
    }

    let url = fetch_pdb::build_url(guid);
    if let Some(pdbfile) = fetch_pdb::try_fetch(&url) {
        let rva = parse_pdb(pdbfile)?;
        save_rva_and_patterns(&dir, guid, rva)?;
        return Ok(rva);
    }

    let dll_bytes = scan_dll::read_dll()?;
    if let Some(rva) = try_multi_pattern_scan(&dir, guid, &dll_bytes)? {
        return Ok(rva);
    }

    if let Some(rva) = structural_scan::find_by_gdi_calls(&dll_bytes) {
        let anchor = scan_dll::read_at_rva(&dll_bytes, rva, 8).unwrap_or_default();
        if verify_live(rva, &anchor)? {
            save_rva_and_patterns(&dir, guid, rva)?;
            return Ok(rva);
        }
    }

    Err(
        "could not locate CDesktopWatermark::s_DesktopBuildPaint \
         (no PDB, no pattern match, structural scan failed)"
            .into(),
    )
}

fn try_multi_pattern_scan(
    dir: &Path,
    guid: &str,
    dll_bytes: &[u8],
) -> Result<Option<u32>, String> {
    let patterns_path = dir.join("patterns.bin");
    if !patterns_path.exists() {
        return Ok(None);
    }
    let data = fs::read(&patterns_path).map_err(|e| e.to_string())?;
    let Some(patterns) = scan_dll::load_patterns(&data) else {
        return Ok(None);
    };
    let hits = scan_dll::scan_for_multi_pattern(dll_bytes, &patterns);
    if hits.len() != 1 {
        return Ok(None);
    }
    let Some(rva) = scan_dll::file_offset_to_rva(dll_bytes, hits[0]) else {
        return Ok(None);
    };
    let anchor = &patterns[0].1;
    if !verify_live(rva, anchor)? {
        return Ok(None);
    }
    cleanup_old_rva_files(dir);
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{guid}.rva")), rva.to_be_bytes()).map_err(|e| e.to_string())?;
    Ok(Some(rva))
}

fn save_rva_and_patterns(dir: &Path, guid: &str, rva: u32) -> Result<(), String> {
    cleanup_old_rva_files(dir);
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{guid}.rva")), rva.to_be_bytes()).map_err(|e| e.to_string())?;
    if let Ok(dll_bytes) = scan_dll::read_dll() {
        if let Some(data) = scan_dll::save_patterns(&dll_bytes, rva) {
            let _ = fs::write(dir.join("patterns.bin"), data);
        }
    }
    Ok(())
}

fn cleanup_old_rva_files(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "rva") {
            let _ = fs::remove_file(path);
        }
    }
}

fn verify_live(rva: u32, expected: &[u8]) -> Result<bool, String> {
    if expected.is_empty() {
        return Ok(false);
    }
    let pid = explorer_pids()
        .into_iter()
        .next()
        .ok_or_else(|| "explorer.exe is not running".to_string())?;
    unsafe {
        let handle = open_explorer(pid)?;
        let base = match shell32_base(handle) {
            Ok(b) => b,
            Err(e) => {
                close(handle);
                return Err(e);
            }
        };
        let ok = verify_rva(handle, base, rva, expected);
        close(handle);
        Ok(ok)
    }
}
