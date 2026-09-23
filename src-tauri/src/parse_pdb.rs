use std::io::Cursor;

use pdb::FallibleIterator;

/// Find the RVA of CDesktopWatermark::s_DesktopBuildPaint in a shell32 PDB.
pub fn parse_pdb(pdbfile: Vec<u8>) -> Result<u32, String> {
    let mut shell32 = pdb::PDB::open(Cursor::new(pdbfile)).map_err(|e| e.to_string())?;
    let symbol_table = shell32.global_symbols().map_err(|e| e.to_string())?;
    let address_map = shell32.address_map().map_err(|e| e.to_string())?;
    let mut iter = symbol_table.iter();
    while let Ok(Some(symbol)) = iter.next() {
        let Ok(data) = symbol.parse() else {
            continue;
        };
        if let pdb::SymbolData::Public(d) = data {
            if d.function && d.name.to_string().contains("s_DesktopBuildPaint") {
                let rva = d
                    .offset
                    .to_rva(&address_map)
                    .ok_or_else(|| "symbol has no RVA".to_string())?;
                return Ok(rva.0);
            }
        }
    }
    Err("s_DesktopBuildPaint not found in PDB".into())
}
