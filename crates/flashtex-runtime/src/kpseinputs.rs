//! What kpathsea's configuration came from, outside the files a run reads
//! (#2058): the environment variables it read, and the files its
//! configuration is read from (texmf.cnf, the fontmaps, the `ls-R` and
//! alias databases).
//!
//! A stored S₀ holds what a run reached under one configuration; a process
//! that opens it under another (`TEXINPUTS`, `max_print_line`, a package
//! installed into an `ls-R` tree) must not take it for its own run
//! (`host::Key::check_config` in the engine). kpathsea's searches and
//! settings come from those variables and files, and a lookup made again
//! sees a database change only where it searches the disk, so they are
//! keyed themselves.
//!
//! - **The environment.** kpathsea is compiled with `getenv` and `putenv`
//!   renamed to [`flashtex_env_getenv`] and [`flashtex_env_putenv`]
//!   (build.rs), which note each name and call the C library's. A name it
//!   sets itself (`SELFAUTOLOC`, `engine`, `MAKETEX_MODE` when PDF output
//!   starts) is its own state, not the process's input: [`env_reads`]
//!   leaves those out. (MSVC's kpathsea sets through `_putenv`, which
//!   win32lib.h names directly; a name it sets and reads there stays in,
//!   which can only refuse more.)
//! - **The files.** flashtex_kpse.c sets kpathsea's `record_input` (what
//!   web2c's `-recorder` uses), called for each texmf.cnf and fontmap it
//!   reads and, by a FlashTeX change to db.c, for each database; each is
//!   noted with its stat signature then ([`inputs`]), since kpathsea reads
//!   them once per instance.
//!
//! Without the `kpathsea` feature both lists are empty.

use crate::statsig::StatSig;
use std::collections::BTreeSet;
use std::sync::Mutex;

struct Noted {
    read: BTreeSet<String>,
    set: BTreeSet<String>,
    inputs: Vec<(String, StatSig)>,
}

static NOTED: Mutex<Noted> = Mutex::new(Noted {
    read: BTreeSet::new(),
    set: BTreeSet::new(),
    inputs: Vec::new(),
});

fn noted() -> std::sync::MutexGuard<'static, Noted> {
    // (called from C: never a panic for a poisoned lock)
    NOTED.lock().unwrap_or_else(|e| e.into_inner())
}

/// Every environment variable kpathsea has read in this process and never
/// set, by name, sorted.
pub fn env_reads() -> Vec<String> {
    let n = noted();
    n.read.difference(&n.set).cloned().collect()
}

/// Every file kpathsea has read its configuration from in this process,
/// with its stat signature when it was read, in order, each once.
pub fn inputs() -> Vec<(String, StatSig)> {
    noted().inputs.clone()
}

#[cfg_attr(not(feature = "kpathsea"), allow(dead_code))]
fn note_read(name: &str) {
    let mut n = noted();
    if !n.read.contains(name) {
        n.read.insert(name.to_string());
    }
}

#[cfg_attr(not(feature = "kpathsea"), allow(dead_code))]
fn note_set(name: &str) {
    let mut n = noted();
    if !n.set.contains(name) {
        n.set.insert(name.to_string());
    }
}

#[cfg_attr(not(feature = "kpathsea"), allow(dead_code))]
fn note_input(path: &str) {
    if noted().inputs.iter().any(|(p, _)| p == path) {
        return;
    }
    // (signed outside the lock; a file kpathsea could not stat is keyed
    // by the default signature, which no file has)
    let sig = StatSig::of(path).unwrap_or_default();
    let mut n = noted();
    if !n.inputs.iter().any(|(p, _)| p == path) {
        n.inputs.push((path.to_string(), sig));
    }
}

#[cfg(feature = "kpathsea")]
mod hooks {
    use std::ffi::{c_char, c_int, CStr};

    extern "C" {
        fn getenv(name: *const c_char) -> *mut c_char;
        #[cfg_attr(windows, link_name = "_putenv")]
        fn putenv(entry: *mut c_char) -> c_int;
    }

    /// kpathsea's `getenv` (build.rs): the name noted, then the C
    /// library's answer.
    ///
    /// # Safety
    /// As `getenv`: `name` is a NUL-terminated string.
    #[no_mangle]
    pub unsafe extern "C" fn flashtex_env_getenv(name: *const c_char) -> *mut c_char {
        if !name.is_null() {
            // SAFETY: the caller's string, NUL-terminated.
            let n = unsafe { CStr::from_ptr(name) };
            super::note_read(&n.to_string_lossy());
        }
        // SAFETY: as the caller's call.
        unsafe { getenv(name) }
    }

    /// kpathsea's `putenv` (build.rs): the name before the `=` noted as
    /// kpathsea's own, then the C library's `putenv`.
    ///
    /// # Safety
    /// As `putenv`: `entry` is a NUL-terminated `NAME=VALUE` that stays
    /// allocated while it is in the environment.
    #[no_mangle]
    pub unsafe extern "C" fn flashtex_env_putenv(entry: *mut c_char) -> c_int {
        if !entry.is_null() {
            // SAFETY: the caller's string, NUL-terminated.
            let e = unsafe { CStr::from_ptr(entry) }.to_bytes();
            let name = e.split(|&c| c == b'=').next().unwrap_or(e);
            super::note_set(&String::from_utf8_lossy(name));
        }
        // SAFETY: as the caller's call.
        unsafe { putenv(entry) }
    }

    /// kpathsea's `record_input` (flashtex_kpse.c): a file its
    /// configuration was read from.
    ///
    /// # Safety
    /// `path` is a NUL-terminated string.
    #[no_mangle]
    pub unsafe extern "C" fn flashtex_kpse_note_input(path: *const c_char) {
        if !path.is_null() {
            // SAFETY: the caller's string, NUL-terminated.
            let p = unsafe { CStr::from_ptr(path) };
            super::note_input(&p.to_string_lossy());
        }
    }
}

#[cfg(all(test, feature = "kpathsea"))]
mod tests {
    use std::ffi::CString;

    /// A name read is listed until kpathsea sets it; a file is listed once.
    #[test]
    fn reads_are_listed_and_kpathsea_s_own_names_are_not() {
        let a = CString::new("FLASHTEX_KPSEINPUTS_TEST_A").unwrap();
        let b = CString::new("FLASHTEX_KPSEINPUTS_TEST_B").unwrap();
        // SAFETY: NUL-terminated names.
        unsafe {
            super::hooks::flashtex_env_getenv(a.as_ptr());
            super::hooks::flashtex_env_getenv(b.as_ptr());
        }
        let reads = super::env_reads();
        assert!(reads.contains(&"FLASHTEX_KPSEINPUTS_TEST_A".to_string()));
        assert!(reads.contains(&"FLASHTEX_KPSEINPUTS_TEST_B".to_string()));
        // (leaked: putenv keeps the string)
        let e = CString::new("FLASHTEX_KPSEINPUTS_TEST_B=1")
            .unwrap()
            .into_raw();
        // SAFETY: a NUL-terminated entry that stays allocated.
        assert_eq!(unsafe { super::hooks::flashtex_env_putenv(e) }, 0);
        let reads = super::env_reads();
        assert!(reads.contains(&"FLASHTEX_KPSEINPUTS_TEST_A".to_string()));
        assert!(!reads.contains(&"FLASHTEX_KPSEINPUTS_TEST_B".to_string()));
        assert_eq!(
            std::env::var("FLASHTEX_KPSEINPUTS_TEST_B").as_deref(),
            Ok("1")
        );
        let f = std::env::temp_dir().join(format!("kpseinputs-{}", std::process::id()));
        std::fs::write(&f, "x").unwrap();
        let p = CString::new(f.to_string_lossy().as_bytes()).unwrap();
        // SAFETY: a NUL-terminated path.
        unsafe {
            super::hooks::flashtex_kpse_note_input(p.as_ptr());
            super::hooks::flashtex_kpse_note_input(p.as_ptr());
        }
        let n = super::inputs()
            .iter()
            .filter(|(q, _)| *q == f.to_string_lossy())
            .count();
        assert_eq!(n, 1);
        let _ = std::fs::remove_file(&f);
    }
}
