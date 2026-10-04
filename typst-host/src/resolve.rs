//! On-demand source mapping (display-list-v3 spec §11.6, E8): `RESOLVE`
//! (a click on a page → the source position it came from) and `LOCATE`
//! (a source position → where it is drawn), answered with typst-ide's own
//! `jump_from_click` and `jump_from_cursor` over the document whose pages
//! the client shows.
//!
//! Page space is the display list's (spec §4.1): sp from the top-left corner
//! of the page box, bleed included, y down; Typst's frame starts inside the
//! bleed, so the bleed is subtracted on the way in and added on the way out.

use std::num::NonZeroUsize;

use flashtex_display_list::json::Json;
use typst::layout::{Abs, Point};
use typst::{World, WorldExt};
use typst_ide::Jump;
use typst_layout::PagedDocument;

use crate::world::{project_file, HostWorld};

/// sp per bp (spec §1).
const SP_PER_BP: f64 = 65_781.76;

fn sp(bp: f64) -> i64 {
    (bp * SP_PER_BP).round() as i64
}

/// Where a click at (`x`, `y`) (sp, page space) on page `page` (0-based)
/// comes from: `file`, `line` (1-based), `column` (0-based byte column) and
/// `byte` (offset in the file), or `"none": true` (nothing there, or a link,
/// which is a jump rather than a source position).
pub fn resolve(
    world: &HostWorld,
    doc: &PagedDocument,
    page: usize,
    x: i64,
    y: i64,
) -> Vec<(String, Json)> {
    let none = vec![("none".to_string(), Json::Bool(true))];
    let Some(p) = doc.pages().get(page) else {
        return none;
    };
    let point = Point::new(
        Abs::pt(x as f64 / SP_PER_BP) - p.bleed.left,
        Abs::pt(y as f64 / SP_PER_BP) - p.bleed.top,
    );
    let position = typst::introspection::PagedPosition {
        page: NonZeroUsize::new(page + 1).unwrap(),
        point,
    };
    let Some(Jump::File(id, byte)) = typst_ide::jump_from_click(world, doc, &position) else {
        return none;
    };
    let (Some(path), Ok(src)) = (world.path_of(id), world.source(id)) else {
        return none;
    };
    let Some(line) = src.lines().byte_to_line(byte) else {
        return none;
    };
    let Some(start) = src.lines().line_to_byte(line) else {
        return none;
    };
    vec![
        (
            "file".into(),
            Json::Str(path.to_string_lossy().into_owned()),
        ),
        ("line".into(), Json::Int(line as i64 + 1)),
        ("column".into(), Json::Int((byte - start) as i64)),
        ("byte".into(), Json::Int(byte as i64)),
    ]
}

/// Where byte `byte` of project file `path` (absolute, as SOURCES names it)
/// is drawn: `[page (0-based), x, y]` in page space (sp); empty when nothing
/// is drawn from there or the file is not the project's.
pub fn locate(world: &HostWorld, doc: &PagedDocument, path: &str, byte: usize) -> Vec<[i64; 3]> {
    let Ok(rel) = std::path::Path::new(path).strip_prefix(world.root()) else {
        return vec![];
    };
    let rel = rel.to_string_lossy().replace('\\', "/");
    let Ok(id) = project_file(&rel) else {
        return vec![];
    };
    let Ok(src) = world.source(id) else {
        return vec![];
    };
    if byte > src.text().len() {
        return vec![];
    }
    typst_ide::jump_from_cursor(doc, &src, byte)
        .into_iter()
        .filter_map(|p| {
            let page = doc.pages().get(p.page.get() - 1)?;
            Some([
                p.page.get() as i64 - 1,
                sp((p.point.x + page.bleed.left).to_pt()),
                sp((p.point.y + page.bleed.top).to_pt()),
            ])
        })
        .collect()
}

/// `world.range` for a span, re-exported for the tests' cross-checks.
#[doc(hidden)]
pub fn span_start(world: &HostWorld, span: typst::syntax::Span) -> Option<usize> {
    world.range(span).map(|r| r.start)
}
