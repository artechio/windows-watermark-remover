use std::fs;
use std::path::Path;

use crate::constants::data_dir;
use crate::explorer::{close, explorer_pids, open_explorer, shell32_base, verify_rva};
use crate::fetch_pdb;
use crate::parse_pdb::parse_pdb;
use crate::scan_dll;
use crate::structural_scan;

/// Resolve the desktop watermark painter in the current shell32.
pub fn get_rva(cache_key: &str, emit: &dyn Fn(String)) -> Result<u32, String> {
    let dir = data_dir();
    let rva_path = dir.join(format!("{cache_key}.rva"));

    if rva_path.exists() {
        emit("Using a saved fix for this Windows build.".into());
        let file = fs::read(&rva_path).map_err(|e| e.to_string())?;
        let bytes: [u8; 4] = file
            .try_into()
            .map_err(|_| "Saved fix file is unreadable.".to_string())?;
        return Ok(u32::from_be_bytes(bytes));
    }

    // Only Microsoft symbol ids look like 32+ hex chars; PE-* fingerprints skip PDB.
    if !cache_key.starts_with("PE-") {
        emit("Downloading build symbols from Microsoft…".into());
        let url = fetch_pdb::build_url(cache_key);
        if let Some(pdbfile) = fetch_pdb::try_fetch(&url) {
            emit("Symbols downloaded. Locating the watermark painter…".into());
            let rva = parse_pdb(pdbfile)?;
            emit("Found via Microsoft symbols.".into());
            save_rva_and_patterns(&dir, cache_key, rva)?;
            return Ok(rva);
        }
        emit("Symbols are not available yet. Trying a local scan…".into());
    } else {
        emit("Using a local scan for this build…".into());
    }

    let dll_bytes = scan_dll::read_dll()?;
    if let Some(rva) = try_multi_pattern_scan(&dir, cache_key, &dll_bytes, emit)? {
        return Ok(rva);
    }

    emit("Scanning the desktop shell for the watermark painter…".into());
    if let Some(rva) = structural_scan::find_by_gdi_calls(&dll_bytes) {
        let anchor = scan_dll::read_at_rva(&dll_bytes, rva, 8).unwrap_or_default();
        emit("Candidate found. Checking the live desktop…".into());
        if verify_live(rva, &anchor)? {
            emit("Live check passed.".into());
            save_rva_and_patterns(&dir, cache_key, rva)?;
            return Ok(rva);
        }
        emit("Live check failed for that candidate.".into());
    } else {
        emit("Local scan did not find a unique match.".into());
    }

    Err(
        "Could not find the desktop watermark painter on this Windows build."
            .into(),
    )
}

fn try_multi_pattern_scan(
    dir: &Path,
    guid: &str,
    dll_bytes: &[u8],
    emit: &dyn Fn(String),
) -> Result<Option<u32>, String> {
    let patterns_path = dir.join("patterns.bin");
    if !patterns_path.exists() {
        emit("No saved local pattern yet.".into());
        return Ok(None);
    }
    let data = fs::read(&patterns_path).map_err(|e| e.to_string())?;
    let Some(patterns) = scan_dll::load_patterns(&data) else {
        emit("Saved pattern file is unreadable.".into());
        return Ok(None);
    };
    emit("Trying the saved local pattern…".into());
    let hits = scan_dll::scan_for_multi_pattern(dll_bytes, &patterns);
    if hits.len() != 1 {
        emit("Saved pattern did not match this build.".into());
        return Ok(None);
    }
    let Some(rva) = scan_dll::file_offset_to_rva(dll_bytes, hits[0]) else {
        return Ok(None);
    };
    let anchor = &patterns[0].1;
    if !verify_live(rva, anchor)? {
        emit("Saved pattern failed the live check.".into());
        return Ok(None);
    }
    cleanup_old_rva_files(dir);
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{guid}.rva")), rva.to_be_bytes()).map_err(|e| e.to_string())?;
    emit("Matched with the saved local pattern.".into());
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
        .ok_or_else(|| "The desktop shell is not running.".to_string())?;
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
