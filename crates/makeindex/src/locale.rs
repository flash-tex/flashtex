//! The C library calls whose result depends on the locale.
//!
//! makeindex runs in the C locale except in two places: `new_entry`
//! (genind.c) switches `LC_CTYPE` to the environment's (`setlocale(LC_CTYPE,
//! "")`) around its `first_letter`/`put_header` calls, and `sort_idx`
//! (sortid.c) does the same for `LC_COLLATE`, which only `strcoll` (`-L`,
//! `-T`) reads. Instead of changing the process's locale, the port asks the
//! C library for that locale object (`newlocale(mask, "", 0)`, which reads
//! `LC_ALL`, `LC_CTYPE`/`LC_COLLATE` and `LANG` exactly as `setlocale` does)
//! and calls the `_l` functions on it, so the answers are the platform's own
//! (on macOS a UTF-8 locale classifies bytes 0x80-0xFF as Latin-1 code
//! points). A locale the C library does not know leaves the C locale, as a
//! failing `setlocale` does. Without a C library to ask (Windows), the C
//! locale is used.

/// The C locale's `isupper`, `tolower`, `toupper`, `isdigit` (ASCII only).
pub fn c_isupper(c: i32) -> bool {
    (b'A' as i32..=b'Z' as i32).contains(&c)
}
pub fn c_tolower(c: i32) -> i32 {
    if c_isupper(c) {
        c + 32
    } else {
        c
    }
}
pub fn c_toupper(c: i32) -> i32 {
    if (b'a' as i32..=b'z' as i32).contains(&c) {
        c - 32
    } else {
        c
    }
}
pub fn c_isdigit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}

/// mkind.h's `TOLOWER` in the C locale.
pub fn tolower_c(c: u8) -> u8 {
    c_tolower(c as i32) as u8
}

#[cfg(unix)]
mod sys {
    use std::os::raw::{c_char, c_int, c_void};
    extern "C" {
        fn newlocale(mask: c_int, locale: *const c_char, base: *mut c_void) -> *mut c_void;
        fn freelocale(l: *mut c_void);
        fn isupper_l(c: c_int, l: *mut c_void) -> c_int;
        fn tolower_l(c: c_int, l: *mut c_void) -> c_int;
        fn toupper_l(c: c_int, l: *mut c_void) -> c_int;
        fn strcoll_l(a: *const c_char, b: *const c_char, l: *mut c_void) -> c_int;
    }

    pub struct Loc(*mut c_void);

    impl Loc {
        fn new(mask: c_int) -> Option<Loc> {
            // SAFETY: a NUL-terminated empty name; NULL base.
            let l = unsafe { newlocale(mask, c"".as_ptr(), std::ptr::null_mut()) };
            (!l.is_null()).then_some(Loc(l))
        }
        pub fn ctype() -> Option<Loc> {
            Loc::new(libc::LC_CTYPE_MASK)
        }
        pub fn collate() -> Option<Loc> {
            Loc::new(libc::LC_COLLATE_MASK)
        }
        pub fn isupper(&self, c: i32) -> bool {
            // SAFETY: a live locale object.
            unsafe { isupper_l(c, self.0) != 0 }
        }
        pub fn tolower(&self, c: i32) -> i32 {
            // SAFETY: as above.
            unsafe { tolower_l(c, self.0) }
        }
        pub fn toupper(&self, c: i32) -> i32 {
            // SAFETY: as above.
            unsafe { toupper_l(c, self.0) }
        }
        pub fn strcoll(&self, a: &[u8], b: &[u8]) -> i32 {
            let a = std::ffi::CString::new(a).unwrap_or_default();
            let b = std::ffi::CString::new(b).unwrap_or_default();
            // SAFETY: NUL-terminated strings and a live locale object.
            unsafe { strcoll_l(a.as_ptr(), b.as_ptr(), self.0) }
        }
    }

    impl Drop for Loc {
        fn drop(&mut self) {
            // SAFETY: made by newlocale, freed once.
            unsafe { freelocale(self.0) }
        }
    }
}

#[cfg(not(unix))]
mod sys {
    pub struct Loc;
    impl Loc {
        pub fn ctype() -> Option<Loc> {
            None
        }
        pub fn collate() -> Option<Loc> {
            None
        }
        pub fn isupper(&self, c: i32) -> bool {
            super::c_isupper(c)
        }
        pub fn tolower(&self, c: i32) -> i32 {
            super::c_tolower(c)
        }
        pub fn toupper(&self, c: i32) -> i32 {
            super::c_toupper(c)
        }
        pub fn strcoll(&self, a: &[u8], b: &[u8]) -> i32 {
            super::strcmp(a, b)
        }
    }
}

/// The environment's locale for one category, or the C locale.
pub struct Env {
    loc: Option<sys::Loc>,
}

impl Env {
    pub fn ctype() -> Env {
        Env {
            loc: sys::Loc::ctype(),
        }
    }
    pub fn collate() -> Env {
        Env {
            loc: sys::Loc::collate(),
        }
    }
    pub fn isupper(&self, c: i32) -> bool {
        match &self.loc {
            Some(l) => l.isupper(c),
            None => c_isupper(c),
        }
    }
    pub fn tolower(&self, c: i32) -> i32 {
        match &self.loc {
            Some(l) => l.tolower(c),
            None => c_tolower(c),
        }
    }
    pub fn toupper(&self, c: i32) -> i32 {
        match &self.loc {
            Some(l) => l.toupper(c),
            None => c_toupper(c),
        }
    }
    /// `strcoll` (the C locale's is `strcmp`).
    pub fn strcoll(&self, a: &[u8], b: &[u8]) -> i32 {
        match &self.loc {
            Some(l) => l.strcoll(a, b),
            None => strcmp(a, b),
        }
    }
}

/// `strcmp` on C strings (the slices hold no NUL).
pub fn strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let x = a.get(i).copied().unwrap_or(0) as i32;
        let y = b.get(i).copied().unwrap_or(0) as i32;
        if x != y || x == 0 {
            return x - y;
        }
        i += 1;
    }
}
