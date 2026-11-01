use flashtex_font_resources::{inspect_static_truetype, sha256, MAX_FONT_BYTES};
use std::io::Read;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: inspect <explicit-font-path>")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_FONT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let metadata = inspect_static_truetype(&bytes)?;
    println!(
        "sha256={} bytes={} metadata={metadata:?}",
        sha256(&bytes),
        bytes.len()
    );
    Ok(())
}
