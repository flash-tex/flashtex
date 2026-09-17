//! FlashTeX bibliography data layer: an original BibTeX `.bib` parser, name
//! parser, citation resolver, and `plain`/`unsrt`/`alpha` layout data.
//!
//! No external crates, no existing BibTeX engine. Spans are UTF-8 byte
//! offsets and diagnostics follow `docs/contracts/runtime-v1.md`.
//!
//! ```
//! use flashtex_bibliography::{Citation, Span, Style, format_bibliography, load, resolve};
//!
//! let db = load(r#"@article{knuth84, author = "Donald E. Knuth", title = {The {\TeX}book},
//!                  journal = {Journal}, year = 1984, pages = {1-2}}"#);
//! assert!(db.diagnostics.is_empty());
//! let cites = [Citation::new("knuth84", Span::new(6, 13))];
//! let res = resolve(&cites, &db, Style::Plain);
//! assert_eq!(res.items[0].label, "1");
//! let items = format_bibliography(&db, &res);
//! assert_eq!(items[0].text(), "Donald\u{a0}E. Knuth. The TeXbook. Journal, pages 1–2, 1984.");
//! ```

pub mod diagnostics;
pub mod format;
pub mod latex;
pub mod model;
pub mod names;
pub mod parser;
pub mod resolve;

pub use diagnostics::{Diagnostic, Severity, Span};
pub use format::{Block, FormattedEntry, Run, RunStyle, format_bibliography, format_entry};
pub use model::{Database, Entry, EntryTypeSpec, Field, Fields, Macro, Preamble, validate};
pub use names::{Name, format_name, parse_name, parse_names, split_names};
pub use parser::parse;
pub use resolve::{BibItem, Citation, Resolution, Style, resolve};

/// Parse and validate a `.bib` source: syntax recovery, macro resolution,
/// duplicate keys, unknown entry types, and missing required fields.
pub fn load(src: &str) -> Database {
    let mut db = parse(src);
    validate(&mut db);
    db
}
