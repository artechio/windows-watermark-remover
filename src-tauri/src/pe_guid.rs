//! Read the CodeView RSDS PDB identity from on-disk shell32.dll.

use crate::constants::SHELL32_PATH;

/// Prefer the Microsoft symbol-server id. If the DLL has no RSDS debug
/// directory, fall back to a stable PE fingerprint so caching still works.
pub fn shell32_cache_key(emit: &dyn Fn(String)) -> Result<String, String> {
    let dll = std::fs::read(SHELL32_PATH).map_err(|e| e.to_string())?;
    if let Some(guid) = rsds_guid_age(&dll) {
        emit(format!("Found RSDS debug id: {guid}"));
        return Ok(guid);
    }
    emit("No RSDS debug directory in shell32.dll — using PE fingerprint and skipping PDB download.".into());
    pe_fingerprint(&dll).ok_or_else(|| "could not read shell32.dll PE header".into())
}

fn pe_fingerprint(dll: &[u8]) -> Option<String> {
    let e_lfanew = u32::from_le_bytes(dll.get(0x3C..0x40)?.try_into().ok()?) as usize;
    // COFF TimeDateStamp at e_lfanew + 8.
    let stamp = u32::from_le_bytes(dll.get(e_lfanew + 8..e_lfanew + 12)?.try_into().ok()?);
    let opt = e_lfanew + 24;
    let magic = u16::from_le_bytes(dll.get(opt..opt + 2)?.try_into().ok()?);
    let size_of_image = if magic == 0x20B {
        u32::from_le_bytes(dll.get(opt + 56..opt + 60)?.try_into().ok()?)
    } else if magic == 0x10B {
        u32::from_le_bytes(dll.get(opt + 56..opt + 60)?.try_into().ok()?)
    } else {
        return None;
    };
    Some(format!("PE-{stamp:08X}-{size_of_image:08X}"))
}

fn rsds_guid_age(dll: &[u8]) -> Option<String> {
    let e_lfanew = u32::from_le_bytes(dll.get(0x3C..0x40)?.try_into().ok()?) as usize;
    let opt = e_lfanew + 24;
    let magic = u16::from_le_bytes(dll.get(opt..opt + 2)?.try_into().ok()?);
    // IMAGE_DIRECTORY_ENTRY_DEBUG = 6.
    // PE32 data directories start at optional-header + 96 → debug at +144.
    // PE32+ data directories start at optional-header + 112 → debug at +160.
    let (dbg_rva, dbg_size) = if magic == 0x20B {
        (
            u32::from_le_bytes(dll.get(opt + 160..opt + 164)?.try_into().ok()?),
            u32::from_le_bytes(dll.get(opt + 164..opt + 168)?.try_into().ok()?),
        )
    } else if magic == 0x10B {
        (
            u32::from_le_bytes(dll.get(opt + 144..opt + 148)?.try_into().ok()?),
            u32::from_le_bytes(dll.get(opt + 148..opt + 152)?.try_into().ok()?),
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
        let addr_rva = u32::from_le_bytes(dll.get(b + 20..b + 24)?.try_into().ok()?);
        let ptr = u32::from_le_bytes(dll.get(b + 24..b + 28)?.try_into().ok()?) as usize;
        let data_off = if ptr != 0 {
            ptr
        } else {
            rva_to_offset(dll, addr_rva)? as usize
        };
        if data_off + 24 > dll.len() || size < 24 {
            continue;
        }
        if &dll[data_off..data_off + 4] != b"RSDS" {
            continue;
        }
        let guid = &dll[data_off + 4..data_off + 20];
        let age = u32::from_le_bytes(dll.get(data_off + 20..data_off + 24)?.try_into().ok()?);
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

#[cfg(test)]
mod tests {
    #[test]
    fn pe32_plus_debug_dir_offset_is_160() {
        // Optional header start + 112 (fixed) + 6*8 (debug index) = 160.
        assert_eq!(112 + 6 * 8, 160);
        assert_eq!(96 + 6 * 8, 144);
    }
}
