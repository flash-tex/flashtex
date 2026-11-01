//! The file trace, for debugging: moved unchanged from flashtex-engine's
//! `system.rs` (which re-exports it) with the lookup proofs that write it
//! (docs/design/xetex/PLAN.md §3.3).

use std::io::Write;

/// Debugging (`FLASHTEX_FILE_TRACE=FILE`): append one line per change the
/// checkpoint layer makes to an output file.
pub fn file_trace(msg: impl FnOnce() -> String) {
    static T: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    if let Some(p) = T.get_or_init(|| std::env::var("FLASHTEX_FILE_TRACE").ok()) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(p)
        {
            let _ = writeln!(f, "{}", msg());
        }
    }
}
