//! `display-list-v3`: FlashTeX's preview wire format and the engine host's
//! socket protocol. The specification is `docs/protocol/display-list-v3.md`;
//! this crate is its reference implementation, MIT-licensed like the app.
//!
//! * [`frame`]: how every message is framed, on the socket and in a file.
//! * [`page`]: the binary `PAGE`/`FORM` body (positioned glyphs, rules, paths,
//!   images, forms, links, destinations, source spans), with an encoder the
//!   engine uses and a decoder clients use, and the content hash.
//! * [`resource`]: `FONT`, `IMAGE` and `SOURCES` bodies.
//! * [`client`]: a blocking client for the engine host's Unix socket.
//! * [`endpoint`]: the `FLASHTEX_DISPLAY_LIST` grammar (`fd:N`,
//!   `socket:PATH`, `pipe:NAME` or a file) and opening it for writing.
//! * [`json`]: the small JSON reader/writer the control messages use.
//! * [`canonical`]: the canonical text of decoded frames (decoder parity).
//!
//! This crate never links the engine (GPL-2.0-or-later); the engine links
//! this crate. `scripts/check-license-boundary.sh` enforces the direction.

pub mod canonical;
pub mod client;
pub mod endpoint;
pub mod frame;
pub mod json;
pub mod page;
pub mod resource;
pub mod sha256;

/// The protocol name every `HELLO` carries.
pub const PROTOCOL: &str = "display-list-v3";
/// Major version: a peer refuses any other.
pub const VERSION_MAJOR: u32 = 3;
/// Minor version: additions a peer may ignore (new sections, new JSON keys,
/// new message kinds it does not understand).
pub const VERSION_MINOR: u32 = 1;

/// Scaled points per PDF point (big point): 65536 × 72.27 / 72, exactly
/// 6578176/100.
pub const SP_PER_BP_NUM: i64 = 6_578_176;
pub const SP_PER_BP_DEN: i64 = 100;

/// Scaled points to PDF points.
pub fn sp_to_bp(sp: i32) -> f64 {
    sp as f64 * SP_PER_BP_DEN as f64 / SP_PER_BP_NUM as f64
}

/// Widen a Unix socket's send and receive buffers to 4 MiB. macOS gives a
/// Unix stream socket 8 KiB each way by default, which splits a font program
/// or a large page into hundreds of reads and writes; both ends of every
/// display-list connection call this. Errors are ignored (the default
/// buffers still work).
pub fn widen_socket_buffers(s: &std::os::unix::net::UnixStream) {
    use std::os::fd::AsRawFd;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    const SOL_SOCKET: i32 = 0xffff;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    const SO_SNDBUF: i32 = 0x1001;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    const SO_RCVBUF: i32 = 0x1002;
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    const SOL_SOCKET: i32 = 1;
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    const SO_SNDBUF: i32 = 7;
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    const SO_RCVBUF: i32 = 8;
    extern "C" {
        fn setsockopt(
            fd: i32,
            level: i32,
            name: i32,
            value: *const std::ffi::c_void,
            len: u32,
        ) -> i32;
    }
    let size: i32 = 4 << 20;
    for opt in [SO_SNDBUF, SO_RCVBUF] {
        // SAFETY: a valid descriptor and a 4-byte int option value.
        unsafe {
            setsockopt(
                s.as_raw_fd(),
                SOL_SOCKET,
                opt,
                &size as *const i32 as *const _,
                4,
            );
        }
    }
}

/// Message kinds (the byte after a frame's length).
pub mod kind {
    // client -> host
    pub const C_HELLO: u8 = 0x01;
    pub const COMPILE: u8 = 0x02;
    pub const CANCEL: u8 = 0x03;
    pub const BYE: u8 = 0x04;
    // host -> client
    pub const HELLO: u8 = 0x41;
    pub const STARTED: u8 = 0x42;
    pub const FONT: u8 = 0x43;
    pub const IMAGE: u8 = 0x44;
    pub const PAGE: u8 = 0x45;
    pub const FORM: u8 = 0x46;
    pub const SOURCES: u8 = 0x47;
    pub const DIAGNOSTIC: u8 = 0x48;
    pub const DONE: u8 = 0x49;
    pub const ERROR: u8 = 0x4A;
    /// 3.1: which of an incremental client's pages are current or stale.
    pub const PAGES: u8 = 0x4B;

    /// Name for logs and dumps.
    pub fn name(k: u8) -> &'static str {
        match k {
            C_HELLO => "client-hello",
            COMPILE => "compile",
            CANCEL => "cancel",
            BYE => "bye",
            HELLO => "hello",
            STARTED => "started",
            FONT => "font",
            IMAGE => "image",
            PAGE => "page",
            FORM => "form",
            SOURCES => "sources",
            DIAGNOSTIC => "diagnostic",
            DONE => "done",
            ERROR => "error",
            PAGES => "pages",
            _ => "unknown",
        }
    }
}
