use std::io::Read;

pub fn build_url(guid: &str) -> String {
    // HTTP matches UWD2 and avoids needing a TLS stack when cross-compiling.
    format!("http://msdl.microsoft.com/download/symbols/shell32.pdb/{guid}/shell32.pdb")
}

/// Download a PDB. Returns None on HTTP 404 (Insider symbols often lag).
pub fn try_fetch(url: &str) -> Option<Vec<u8>> {
    let resp = match ureq::get(url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => return None,
        Err(_) => return None,
    };
    let len: usize = resp
        .header("Content-Length")
        .and_then(|h| h.parse().ok())
        .unwrap_or(15_000_000);
    let mut buf = Vec::with_capacity(len);
    resp.into_reader().take(u64::MAX).read_to_end(&mut buf).ok()?;
    Some(buf)
}
