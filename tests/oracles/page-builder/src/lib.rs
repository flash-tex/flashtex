//! FlashTeX page builder: an original implementation of TeX's vertical-mode
//! machinery following *TeX: The Program* (tex.web) and *The TeXbook*
//! chapters 12 and 15.
//!
//! * [`vlist`]: appending boxes with interline glue (§679), rules (§1056),
//!   paragraph lines and their club/widow/broken penalties (§890), displays
//!   (§1199–§1205), LaTeX `\addvspace`/`\addpenalty`.
//! * [`split`]: `prune_page_top`, `vert_break`, `\vsplit` (§967–§979).
//! * [`page`]: the page builder with insertions and `\tracingpages`
//!   output (§980–§1028), `\end` handling (§1054).
//! * [`latex`]: `\pagebreak`, `\nopagebreak`, `\newpage`, `\clearpage`,
//!   `\enlargethispage` and a LaTeX single-column output routine model
//!   (`\@makecol`, `\@doclearpage`, `\raggedbottom`, `\flushbottom`).
//! * [`pack`]: `vpackage` and `vlist_out` vertical positions.
//! * [`format`]: the line format of the pdflatex oracle fixtures.
//!
//! All dimensions are integer scaled points, as in TeX.

pub mod format;
pub mod latex;
pub mod node;
pub mod pack;
pub mod page;
pub mod scaled;
pub mod split;
pub mod vlist;

pub use node::{BoxNode, GlueKind, GlueSpec, InsNode, Node, Order};
pub use pack::{vlist_positions, vpack_natural, vpackage, PackSpec, VBox};
pub use page::{FiredPage, InsertClass, OutputResult, OutputRoutine, PageBuilder, PageParams};
pub use scaled::Scaled;
