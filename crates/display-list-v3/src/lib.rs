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
//! * [`diag`]: `diag-v1` structured diagnostics (`DIAG`), capability-gated.
//! * [`endpoint`]: the `FLASHTEX_DISPLAY_LIST` grammar (`fd:N`,
//!   `socket:PATH`, `pipe:NAME` or a file) and opening it for writing.
//! * [`json`]: the small JSON reader/writer the control messages use.
//! * [`canonical`]: the canonical text of decoded frames (decoder parity).
//!
//! This crate never links the engine (GPL-2.0-or-later); the engine links
//! this crate. `scripts/check-license-boundary.sh` enforces the direction.

pub mod canonical;
pub mod client;
pub mod diag;
pub mod endpoint;
pub mod frame;
pub mod json;
pub mod page;
pub mod resource;
pub mod sha256;
pub mod transport;

/// The protocol name every `HELLO` carries.
pub const PROTOCOL: &str = "display-list-v3";
/// Major version: a peer refuses any other.
pub const VERSION_MAJOR: u32 = 3;
/// The minor version `flashtex-host` (LaTeX) speaks and answers in its
/// `HELLO`: 3.2. Minor versions only add (new sections, JSON keys, message
/// kinds a peer may ignore); 3.3's additions (spec §11) are the Typst
/// host's and nothing requires the LaTeX host to send any of them, so it
/// stays at 3.2 and its replies are unchanged.
pub const VERSION_MINOR: u32 = 2;
/// The specification's latest minor version, 3.3 (spec §11): what the
/// reference [`client`] says in its `HELLO` and what `flashtex-typst-host`
/// speaks. 3.3's item opcodes go only to a client that also accepts them
/// ([`accept`]).
pub const LATEST_MINOR: u32 = 3;

/// Scaled points per PDF point (big point): 65536 × 72.27 / 72, exactly
/// 6578176/100.
pub const SP_PER_BP_NUM: i64 = 6_578_176;
pub const SP_PER_BP_DEN: i64 = 100;

/// Scaled points to PDF points.
pub fn sp_to_bp(sp: i32) -> f64 {
    sp as f64 * SP_PER_BP_DEN as f64 / SP_PER_BP_NUM as f64
}

pub use transport::widen_socket_buffers;

/// The `HELLO` `accept` entry and host capability for `PROGRESS` (spec §6.8).
pub const PROGRESS_CAPABILITY: &str = "progress-v1";

/// 3.3 `HELLO` `accept` entries (spec §11.7): what a client draws. A host
/// never sends an item opcode or message the client did not accept.
pub mod accept {
    /// COLORSPACES, FILL/STROKE_COLOR_CS, FILL/STROKE_ALPHA (§11.3).
    pub const COLOR_SPACES: &str = "color-spaces";
    /// LINE_STATE for stroked glyphs (§11.4).
    pub const LINE_STATE: &str = "line-state";
    /// IMAGE `data` + IMAGE_DATA, and PDF islands (§11.5).
    pub const IMAGE_DATA: &str = "image-data";
    /// FONT `program_from`: each font program once per connection (§11.1).
    pub const FONT_PROGRAM_REFS: &str = "font-program-refs";
    /// FONT with an empty program and a `file` the client reads (§11.1).
    pub const FONT_FILES: &str = "font-files";
}

/// 3.3 host capability: the host answers RESOLVE and LOCATE (spec §11.6).
pub const RESOLVE_CAPABILITY: &str = "resolve-v1";

/// Message kinds (the byte after a frame's length).
pub mod kind {
    // client -> host
    pub const C_HELLO: u8 = 0x01;
    pub const COMPILE: u8 = 0x02;
    pub const CANCEL: u8 = 0x03;
    pub const BYE: u8 = 0x04;
    /// 3.3: click -> source (spec §11.6).
    pub const RESOLVE: u8 = 0x05;
    /// 3.3: source -> page (spec §11.6).
    pub const LOCATE: u8 = 0x06;
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
    /// 3.2: an external tool (bibtex, biber, makeindex) the host runs for a
    /// compile: started, finished, or all settled (spec §6.4).
    pub const TOOL: u8 = 0x4C;
    /// 3.3: an IMAGE's bytes (spec §11.5), right after its IMAGE.
    pub const IMAGE_DATA: u8 = 0x4D;
    /// 3.3: the reply to RESOLVE (spec §11.6).
    pub const RESOLVED: u8 = 0x4E;
    /// 3.3: the reply to LOCATE (spec §11.6).
    pub const LOCATED: u8 = 0x4F;
    /// 3.3, the Typst host: what happened to a package (spec §11.8), for a
    /// client that accepts `packages-v1`.
    pub const PACKAGE: u8 = 0x50;
    /// `diag-v1` (capability-gated, spec §6.7): one structured diagnostic.
    /// The diagnostics family has its own range (0x60..=0x6F) so that it
    /// never meets the page-protocol kinds of a later minor version.
    pub const DIAG: u8 = 0x60;
    /// `progress-v1` (capability-gated, spec §6.8): a heartbeat while a
    /// compile typesets, at each pass start and every few hundred
    /// milliseconds of page or segment checkpoints, also when the pages
    /// are not sent (a later `.aux` pass that changes nothing). Its own
    /// range (0x70..=0x7F), like the diagnostics family.
    pub const PROGRESS: u8 = 0x70;

    /// Name for logs and dumps.
    pub fn name(k: u8) -> &'static str {
        match k {
            C_HELLO => "client-hello",
            COMPILE => "compile",
            CANCEL => "cancel",
            BYE => "bye",
            RESOLVE => "resolve",
            LOCATE => "locate",
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
            TOOL => "tool",
            IMAGE_DATA => "image-data",
            RESOLVED => "resolved",
            LOCATED => "located",
            PACKAGE => "package",
            DIAG => "diag",
            PROGRESS => "progress",
            _ => "unknown",
        }
    }
}
