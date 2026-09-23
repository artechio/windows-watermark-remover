use std::fs;
use std::path::Path;

use crate::constants::data_dir;
use crate::explorer::{close, explorer_pids, open_explorer, shell32_base, verify_rva};
use crate::fetch_pdb;
use crate::parse_pdb::parse_pdb;
use crate::scan_dll;
use crate::structural_scan;

/// Resolve CDesktopWatermark::s_DesktopBuildPaint for the current shell32.
pub fn get_rva(guid: &str, emit: &dyn Fn(String)) -> Result<u32, String> {
    let dir = data_dir();
    let rva_path = dir.join(format!("{guid}.rva"));

    if rva_path.exists() {
        emit("Using cached RVA file.".into());
        let file = fs::read(&rva_path).map_err(|e| e.to_string())?;
        let bytes: [u8; 4] = file
            .try_into()
            .map_err(|_| "corrupt RVA cache".to_string())?;
        return Ok(u32::from_be_bytes(bytes));
    }

    emit("Fetching shell32.pdb from Microsoft symbol server…".into());
    let url = fetch_pdb::build_url(guid);
    emit(format!("GET {url}"));
    if let Some(pdbfile) = fetch_pdb::try_fetch(&url) {
        emit(format!("Downloaded PDB ({} bytes). Parsing symbols…", pdbfile.len()));
        let rva = parse_pdb(pdbfile)?;
        emit(format!("PDB symbol hit at RVA {rva:#x}"));
        save_rva_and_patterns(&dir, guid, rva)?;
        return Ok(rva);
    }
    emit("Symbol server returned 404 or failed. Trying pattern / structural scan…".into());

    let dll_bytes = scan_dll::read_dll()?;
    if let Some(rva) = try_multi_pattern_scan(&dir, guid, &dll_bytes, emit)? {
        return Ok(rva);
    }

    emit("Running structural SetTextColor scan on shell32.dll…".into());
    if let Some(rva) = structural_scan::find_by_gdi_calls(&dll_bytes) {
        let anchor = scan_dll::read_at_rva(&dll_bytes, rva, 8).unwrap_or_default();
        emit(format!("Structural candidate RVA {rva:#x}. Verifying in live Explorer…"));
        if verify_live(rva, &anchor)? {
            emit("Live verification OK.".into());
            save_rva_and_patterns(&dir, guid, rva)?;
            return Ok(rva);
        }
        emit("Live verification failed for structural candidate.".into());
    } else {
        emit("Structural scan found no unique candidate.".into());
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
    emit: &dyn Fn(String),
) -> Result<Option<u32>, String> {
    let patterns_path = dir.join("patterns.bin");
    if !patterns_path.exists() {
        emit("No patterns.bin cache yet.".into());
        return Ok(None);
    }
    let data = fs::read(&patterns_path).map_err(|e| e.to_string())?;
    let Some(patterns) = scan_dll::load_patterns(&data) else {
        emit("patterns.bin unreadable.".into());
        return Ok(None);
    };
    emit(format!("Scanning with {} cached sub-patterns…", patterns.len()));
    let hits = scan_dll::scan_for_multi_pattern(dll_bytes, &patterns);
    if hits.len() != 1 {
        emit(format!("Pattern scan hits: {} (need exactly 1).", hits.len()));
        return Ok(None);
    }
    let Some(rva) = scan_dll::file_offset_to_rva(dll_bytes, hits[0]) else {
        return Ok(None);
    };
    let anchor = &patterns[0].1;
    if !verify_live(rva, anchor)? {
        emit("Pattern candidate failed live verification.".into());
        return Ok(None);
    }
    cleanup_old_rva_files(dir);
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{guid}.rva")), rva.to_be_bytes()).map_err(|e| e.to_string())?;
    emit(format!("Pattern scan matched RVA {rva:#x}"));
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
