//! Locate CDesktopWatermark::s_DesktopBuildPaint without a PDB.
//!
//! Find CALL [rip+disp] sites that hit GDI32!SetTextColor's IAT slot, keep those
//! that load 0x00FFFFFF as the color, then recover the enclosing function RVA
//! from .pdata (or a prologue walk).

use crate::scan_dll::{file_offset_to_rva, rva_to_file_offset};

struct ExecSection {
    virtual_address: u32,
    ptr_to_raw: u32,
    raw_size: u32,
}

fn executable_sections(dll: &[u8]) -> Vec<ExecSection> {
    let Ok(e_lfanew_bytes) = dll[0x3C..0x40].try_into() else {
        return vec![];
    };
    let e_lfanew = u32::from_le_bytes(e_lfanew_bytes) as usize;
    if e_lfanew + 24 > dll.len() {
        return vec![];
    }
    let num_sections =
        u16::from_le_bytes(dll[e_lfanew + 6..e_lfanew + 8].try_into().unwrap_or([0; 2])) as usize;
    let opt_hdr_size =
        u16::from_le_bytes(dll[e_lfanew + 20..e_lfanew + 22].try_into().unwrap_or([0; 2])) as usize;
    let sec_base = e_lfanew + 24 + opt_hdr_size;
    const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
    let mut out = Vec::new();
    for i in 0..num_sections {
        let b = sec_base + i * 40;
        if b + 40 > dll.len() {
            break;
        }
        let characteristics = u32::from_le_bytes(dll[b + 36..b + 40].try_into().unwrap());
        if characteristics & IMAGE_SCN_MEM_EXECUTE == 0 {
            continue;
        }
        out.push(ExecSection {
            virtual_address: u32::from_le_bytes(dll[b + 12..b + 16].try_into().unwrap()),
            raw_size: u32::from_le_bytes(dll[b + 16..b + 20].try_into().unwrap()),
            ptr_to_raw: u32::from_le_bytes(dll[b + 20..b + 24].try_into().unwrap()),
        });
    }
    out
}

fn find_iat_slot_rva(dll: &[u8], dll_name: &str, func_name: &str) -> Option<u32> {
    let e_lfanew = u32::from_le_bytes(dll[0x3C..0x40].try_into().ok()?) as usize;
    let opt_hdr = e_lfanew + 24;
    let magic = u16::from_le_bytes(dll.get(opt_hdr..opt_hdr + 2)?.try_into().ok()?);
    if magic != 0x020B {
        return None;
    }
    let import_dir_rva = u32::from_le_bytes(dll.get(opt_hdr + 120..opt_hdr + 124)?.try_into().ok()?);
    if import_dir_rva == 0 {
        return None;
    }
    let mut desc = rva_to_file_offset(dll, import_dir_rva)? as usize;
    loop {
        if desc + 20 > dll.len() {
            return None;
        }
        let orig_first_thunk = u32::from_le_bytes(dll[desc..desc + 4].try_into().ok()?);
        let first_thunk = u32::from_le_bytes(dll[desc + 16..desc + 20].try_into().ok()?);
        if orig_first_thunk == 0 && first_thunk == 0 {
            break;
        }
        let name_rva = u32::from_le_bytes(dll[desc + 12..desc + 16].try_into().ok()?);
        if let Some(name_file) = rva_to_file_offset(dll, name_rva).map(|v| v as usize) {
            if let Some(nul) = dll[name_file..].iter().position(|&b| b == 0) {
                if let Ok(this_dll) = std::str::from_utf8(&dll[name_file..name_file + nul]) {
                    if this_dll.eq_ignore_ascii_case(dll_name) {
                        if let Some(idx) = find_named_import_index(dll, orig_first_thunk, func_name)
                        {
                            return Some(first_thunk + idx as u32 * 8);
                        }
                    }
                }
            }
        }
        desc += 20;
    }
    None
}

fn find_named_import_index(dll: &[u8], orig_first_thunk_rva: u32, name: &str) -> Option<usize> {
    let thunk_file = rva_to_file_offset(dll, orig_first_thunk_rva)? as usize;
    let mut idx = 0usize;
    loop {
        let off = thunk_file + idx * 8;
        if off + 8 > dll.len() {
            return None;
        }
        let val = u64::from_le_bytes(dll[off..off + 8].try_into().ok()?);
        if val == 0 {
            return None;
        }
        if val & 0x8000_0000_0000_0000 == 0 {
            let ibn_file = rva_to_file_offset(dll, val as u32)? as usize + 2;
            if let Some(nul) = dll[ibn_file..].iter().position(|&b| b == 0) {
                if let Ok(n) = std::str::from_utf8(&dll[ibn_file..ibn_file + nul]) {
                    if n == name {
                        return Some(idx);
                    }
                }
            }
        }
        idx += 1;
    }
}

fn find_indirect_calls(dll: &[u8], target_rva: u32) -> Vec<usize> {
    let mut out = Vec::new();
    for s in executable_sections(dll) {
        let start = s.ptr_to_raw as usize;
        let end = (s.ptr_to_raw + s.raw_size).min(dll.len() as u32) as usize;
        if end < start + 6 {
            continue;
        }
        for i in 0..=(end - start - 6) {
            if dll[start + i] != 0xFF || dll[start + i + 1] != 0x15 {
                continue;
            }
            let disp32 =
                i32::from_le_bytes(dll[start + i + 2..start + i + 6].try_into().unwrap());
            let insn_rva = s.virtual_address + i as u32;
            let target = (insn_rva as i64 + 6 + disp32 as i64) as u32;
            if target == target_rva {
                out.push(start + i);
            }
        }
    }
    out
}

fn has_white_color_arg(dll: &[u8], call_site: usize) -> bool {
    let search_start = call_site.saturating_sub(40);
    let window = &dll[search_start..call_site];
    let mov_edx: &[u8] = &[0xBA, 0xFF, 0xFF, 0xFF, 0x00];
    let mov_r8d: &[u8] = &[0x41, 0xB8, 0xFF, 0xFF, 0xFF, 0x00];
    window.windows(5).any(|w| w == mov_edx) || window.windows(6).any(|w| w == mov_r8d)
}

fn find_func_via_pdata(dll: &[u8], call_site_rva: u32) -> Option<u32> {
    let e_lfanew = u32::from_le_bytes(dll[0x3C..0x40].try_into().ok()?) as usize;
    let opt_hdr = e_lfanew + 24;
    if opt_hdr + 144 > dll.len() {
        return None;
    }
    let exc_rva = u32::from_le_bytes(dll.get(opt_hdr + 136..opt_hdr + 140)?.try_into().ok()?);
    let exc_size = u32::from_le_bytes(dll.get(opt_hdr + 140..opt_hdr + 144)?.try_into().ok()?);
    if exc_rva == 0 || exc_size < 12 {
        return None;
    }
    let exc_file = rva_to_file_offset(dll, exc_rva)? as usize;
    let count = exc_size as usize / 12;
    for i in 0..count {
        let b = exc_file + i * 12;
        if b + 8 > dll.len() {
            break;
        }
        let begin = u32::from_le_bytes(dll[b..b + 4].try_into().ok()?);
        let end = u32::from_le_bytes(dll[b + 4..b + 8].try_into().ok()?);
        if begin <= call_site_rva && call_site_rva < end {
            return Some(begin);
        }
    }
    None
}

fn find_func_via_prologue_walk(dll: &[u8], call_site_file_offset: usize) -> Option<u32> {
    let limit = call_site_file_offset.saturating_sub(512);
    for pos in (limit..call_site_file_offset).rev() {
        if dll[pos] == 0xCC || dll[pos] == 0x90 {
            let func_start = pos + 1;
            if func_start < call_site_file_offset {
                return file_offset_to_rva(dll, func_start as u32);
            }
        }
    }
    None
}

/// Returns the function RVA when exactly one candidate is found.
pub fn find_by_gdi_calls(dll: &[u8]) -> Option<u32> {
    let iat_rva = find_iat_slot_rva(dll, "GDI32.dll", "SetTextColor")?;
    let call_sites = find_indirect_calls(dll, iat_rva);
    let mut candidates: Vec<u32> = Vec::new();
    for site in call_sites {
        if !has_white_color_arg(dll, site) {
            continue;
        }
        let site_rva = file_offset_to_rva(dll, site as u32)?;
        let func_rva =
            find_func_via_pdata(dll, site_rva).or_else(|| find_func_via_prologue_walk(dll, site));
        if let Some(rva) = func_rva {
            if !candidates.contains(&rva) {
                candidates.push(rva);
            }
        }
    }
    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}
