//! Read the CodeView RSDS PDB identity from on-disk shell32.dll.

use crate::constants::SHELL32_PATH;

pub fn shell32_pdb_guid() -> Result<String, String> {
    let dll = std::fs::read(SHELL32_PATH).map_err(|e| e.to_string())?;
    rsds_guid_age(&dll).ok_or_else(|| "shell32.dll has no RSDS debug directory".into())
}

fn rsds_guid_age(dll: &[u8]) -> Option<String> {
    let e_lfanew = u32::from_le_bytes(dll.get(0x3C..0x40)?.try_into().ok()?) as usize;
    let opt = e_lfanew + 24;
    let magic = u16::from_le_bytes(dll.get(opt..opt + 2)?.try_into().ok()?);
    // PE32+ debug directory is data directory index 6 at optional-header + 144.
    let (dbg_rva, dbg_size) = if magic == 0x20B {
        (
            u32::from_le_bytes(dll.get(opt + 144..opt + 148)?.try_into().ok()?),
            u32::from_le_bytes(dll.get(opt + 148..opt + 152)?.try_into().ok()?),
        )
    } else if magic == 0x10B {
        (
            u32::from_le_bytes(dll.get(opt + 128..opt + 132)?.try_into().ok()?),
            u32::from_le_bytes(dll.get(opt + 132..opt + 136)?.try_into().ok()?),
        )
    } else {
        return None;
    };
    if dbg_rva == 0 || dbg_size < 28 {
        return None;
    }
    let dbg_off = rva_to_offset(dll, dbg_rva)? as usize;
    let entries = (dbg_size as usize) / 28;
    for i in 0..entries {
        let b = dbg_off + i * 28;
        let dtype = u32::from_le_bytes(dll.get(b + 12..b + 16)?.try_into().ok()?);
        if dtype != 2 {
            continue; // IMAGE_DEBUG_TYPE_CODEVIEW
        }
        let size = u32::from_le_bytes(dll.get(b + 16..b + 20)?.try_into().ok()?) as usize;
        let ptr = u32::from_le_bytes(dll.get(b + 24..b + 28)?.try_into().ok()?) as usize;
        if ptr + 24 > dll.len() || size < 24 {
            continue;
        }
        if &dll[ptr..ptr + 4] != b"RSDS" {
            continue;
        }
        let guid = &dll[ptr + 4..ptr + 20];
        let age = u32::from_le_bytes(dll.get(ptr + 20..ptr + 24)?.try_into().ok()?);
        // Microsoft symbol path uses GUID bytes in mixed endian form, then age hex.
        let d1 = u32::from_le_bytes(guid[0..4].try_into().ok()?);
        let d2 = u16::from_le_bytes(guid[4..6].try_into().ok()?);
        let d3 = u16::from_le_bytes(guid[6..8].try_into().ok()?);
        let d4 = &guid[8..16];
        return Some(format!(
            "{d1:08X}{d2:04X}{d3:04X}{}{age:X}",
            d4.iter().map(|b| format!("{b:02X}")).collect::<String>()
        ));
    }
    None
}

fn rva_to_offset(dll: &[u8], rva: u32) -> Option<u32> {
    let e_lfanew = u32::from_le_bytes(dll.get(0x3C..0x40)?.try_into().ok()?) as usize;
    let num = u16::from_le_bytes(dll.get(e_lfanew + 6..e_lfanew + 8)?.try_into().ok()?) as usize;
    let opt_size =
        u16::from_le_bytes(dll.get(e_lfanew + 20..e_lfanew + 22)?.try_into().ok()?) as usize;
    let sec = e_lfanew + 24 + opt_size;
    for i in 0..num {
        let b = sec + i * 40;
        let va = u32::from_le_bytes(dll.get(b + 12..b + 16)?.try_into().ok()?);
        let vsz = u32::from_le_bytes(dll.get(b + 8..b + 12)?.try_into().ok()?);
        let raw = u32::from_le_bytes(dll.get(b + 16..b + 20)?.try_into().ok()?);
        let ptr = u32::from_le_bytes(dll.get(b + 20..b + 24)?.try_into().ok()?);
        let span = vsz.max(raw);
        if rva >= va && rva < va.saturating_add(span) {
            return Some(rva - va + ptr);
        }
    }
    None
}
