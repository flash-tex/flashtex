//! Links, named destinations, outlines and document information for the
//! exact export route, in the object shapes pdfTeX 1.40.29 writes for
//! hyperref 7.01p (checked against pdflatex output in `tests/navigation.rs`):
//!
//! - `/Link` annotations with `/Border`, `/H /I` and `/C`, acting either as
//!   `/GoTo` to a named destination or as a `/URI` action, listed in the
//!   page's `/Annots`;
//! - `/D [page /XYZ left top null]` destinations in the catalog's
//!   `/Names /Dests` name tree, leaves of six names (pdfTeX's
//!   `name_tree_kids_max`) with `/Limits`;
//! - an `/Outlines` tree whose parent/child structure comes from hyperref
//!   levels (a bookmark's parent is the nearest earlier one with a smaller
//!   level), closed items counting minus their children, open items all
//!   visible descendants;
//! - `/PageMode`, `/OpenAction [first page /Fit]` and the Info strings
//!   (UTF-16BE with a byte-order mark for non-ASCII text, as hyperref writes
//!   them).
//!
//! Nothing dangles: a `/GoTo` or outline entry naming a destination the
//! document does not define, a destination on a page that does not exist, a
//! malformed rectangle or colour, or an unbounded string is an error before
//! any byte is written. Positions are the caller's exact decimals.

use crate::exact::{Decimal, ExactError};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub const MAX_NAME_BYTES: usize = 1024;
pub const MAX_URI_BYTES: usize = 64 * 1024;
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_LINKS: usize = 1_000_000;
pub const MAX_DESTINATIONS: usize = 1_000_000;
pub const MAX_OUTLINE_ITEMS: usize = 100_000;
/// pdfTeX's `name_tree_kids_max`.
pub const NAME_TREE_KIDS_MAX: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Navigation {
    /// Link annotations by page index, in `/Annots` order. Pages past the
    /// end of this list have none.
    pub links: Vec<Vec<LinkAnnotation>>,
    pub destinations: BTreeMap<String, Destination>,
    /// Outline entries in document order.
    pub outlines: Vec<OutlineItem>,
    /// hyperref `bookmarksopen`: items with children start expanded.
    pub outlines_open: bool,
    /// hyperref `bookmarksopenlevel`: with `outlines_open`, only items whose
    /// level is below this one start expanded (`\@bookmarkopenstatus`:
    /// `\ifnum#1<\Hy@openlevel`). `None` is hyperref's default
    /// `\maxdimen`: every level.
    pub outlines_open_level: Option<i32>,
    pub info: DocumentInfo,
    pub page_mode: Option<PageMode>,
    /// `/OpenAction [<first page> /Fit]` (hyperref's `pdfstartview=Fit`).
    pub open_fit_first_page: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkAnnotation {
    /// `/Rect [llx lly urx ury]`.
    pub rect: [Decimal; 4],
    /// `/Border` (three numbers: corner radii and width).
    pub border: Vec<Decimal>,
    /// `/C`: empty, gray, RGB or CMYK components in `[0, 1]`.
    pub color: Vec<Decimal>,
    pub action: LinkAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAction {
    /// `/A << /S /GoTo /D (name) >>`; the name must be a destination.
    GoTo(String),
    /// `/A << /Type /Action /S /URI /URI (uri) >>`.
    Uri(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    /// Page index (0-based).
    pub page: usize,
    pub view: View,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// `/XYZ left top null` (hyperref's anchors).
    Xyz { left: Decimal, top: Decimal },
    /// `/Fit`.
    Fit,
    /// `/FitH top`.
    FitH { top: Decimal },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    pub title: String,
    pub destination: String,
    /// hyperref level (part -1, section 1, ...); only relative order matters.
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentInfo {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    /// Replaces the writer's `/Creator (FlashTeX)`.
    pub creator: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMode {
    UseNone,
    UseOutlines,
}

/// Object numbers and bodies for one document, planned before any byte is
/// written (the catalog and pages refer to later objects).
pub(crate) struct Plan {
    /// ` /Annots [ .. ]` for each page (empty string: none).
    pub page_entries: Vec<String>,
    /// Catalog entries after `/Pages`.
    pub catalog_entries: String,
    /// Info entries after `/Producer`, including `/Creator`.
    pub info_entries: String,
    /// `(object number, body)` in increasing number order.
    pub objects: Vec<(usize, String)>,
}

fn invalid(message: String) -> ExactError {
    ExactError::Invalid(message)
}

fn check_name(what: &str, name: &str) -> Result<(), ExactError> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(invalid(format!(
            "{what} name {name:?} must be 1..={MAX_NAME_BYTES} bytes"
        )));
    }
    Ok(())
}

fn check_unit(what: &str, values: &[Decimal]) -> Result<(), ExactError> {
    if !matches!(values.len(), 0 | 1 | 3 | 4) {
        return Err(invalid(format!(
            "{what} has {} components; /C takes 0, 1, 3 or 4",
            values.len()
        )));
    }
    for v in values {
        let x = v.approx();
        if !(0.0..=1.0).contains(&x) {
            return Err(invalid(format!("{what} component {v} is outside [0, 1]")));
        }
    }
    Ok(())
}

/// A PDF string literal: printable ASCII verbatim (parentheses and
/// backslashes escaped), every other byte as a three-digit octal escape.
pub fn string_literal(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 2);
    out.push('(');
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(b as char);
            }
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out.push(')');
    out
}

/// A PDF text string: ASCII as is, anything else UTF-16BE with a BOM.
pub fn text_string(text: &str) -> String {
    if text.is_ascii() {
        return string_literal(text.as_bytes());
    }
    let mut bytes = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    string_literal(&bytes)
}

fn numbers(values: &[Decimal]) -> String {
    values
        .iter()
        .map(Decimal::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parent index of each outline item: the nearest earlier item with a
/// smaller hyperref level.
fn outline_parents(items: &[OutlineItem]) -> Vec<Option<usize>> {
    let mut stack: Vec<usize> = Vec::new();
    let mut parents = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        while stack
            .last()
            .is_some_and(|&top| items[top].level >= item.level)
        {
            stack.pop();
        }
        parents.push(stack.last().copied());
        stack.push(index);
    }
    parents
}

pub(crate) fn plan(
    navigation: &Navigation,
    page_count: usize,
    page_object: impl Fn(usize) -> usize,
    next: &mut usize,
) -> Result<Plan, ExactError> {
    let mut plan = Plan {
        page_entries: vec![String::new(); page_count],
        catalog_entries: String::new(),
        info_entries: String::new(),
        objects: Vec::new(),
    };

    // Validation first: nothing is numbered for a document that fails.
    if navigation.links.len() > page_count {
        return Err(invalid(format!(
            "links are given for {} pages but the document has {page_count}",
            navigation.links.len()
        )));
    }
    if navigation.destinations.len() > MAX_DESTINATIONS {
        return Err(ExactError::Limit("destinations"));
    }
    if navigation.outlines.len() > MAX_OUTLINE_ITEMS {
        return Err(ExactError::Limit("outline items"));
    }
    let link_total: usize = navigation.links.iter().map(Vec::len).sum();
    if link_total > MAX_LINKS {
        return Err(ExactError::Limit("link annotations"));
    }
    for (name, destination) in &navigation.destinations {
        check_name("destination", name)?;
        if destination.page >= page_count {
            return Err(invalid(format!(
                "destination {name:?} is on page {} of {page_count}",
                destination.page + 1
            )));
        }
    }
    for (page, links) in navigation.links.iter().enumerate() {
        for link in links {
            let [llx, lly, urx, ury] = &link.rect;
            if urx.approx() < llx.approx() || ury.approx() < lly.approx() {
                return Err(invalid(format!(
                    "page {}: link rectangle [{llx} {lly} {urx} {ury}] is inverted",
                    page + 1
                )));
            }
            if link.border.len() != 3 {
                return Err(invalid(format!(
                    "page {}: /Border needs three numbers, got {}",
                    page + 1,
                    link.border.len()
                )));
            }
            check_unit(&format!("page {} link /C", page + 1), &link.color)?;
            match &link.action {
                LinkAction::GoTo(name) => {
                    check_name("link destination", name)?;
                    if !navigation.destinations.contains_key(name) {
                        return Err(invalid(format!(
                            "page {}: link to undefined destination {name:?}",
                            page + 1
                        )));
                    }
                }
                LinkAction::Uri(uri) => {
                    if uri.is_empty()
                        || uri.len() > MAX_URI_BYTES
                        || uri.bytes().any(|b| b < 0x20 || b == 0x7f)
                    {
                        return Err(invalid(format!(
                            "page {}: URI must be 1..={MAX_URI_BYTES} bytes without control characters",
                            page + 1
                        )));
                    }
                }
            }
        }
    }
    for item in &navigation.outlines {
        if item.title.len() > MAX_TEXT_BYTES {
            return Err(ExactError::Limit("outline title bytes"));
        }
        if !navigation.destinations.contains_key(&item.destination) {
            return Err(invalid(format!(
                "outline entry {:?} names undefined destination {:?}",
                item.title, item.destination
            )));
        }
    }
    let info = &navigation.info;
    for text in [
        &info.title,
        &info.author,
        &info.subject,
        &info.keywords,
        &info.creator,
    ]
    .into_iter()
    .flatten()
    {
        if text.len() > MAX_TEXT_BYTES {
            return Err(ExactError::Limit("document information bytes"));
        }
    }

    // Link annotations, page by page.
    for (page, links) in navigation.links.iter().enumerate() {
        if links.is_empty() {
            continue;
        }
        let mut refs = String::new();
        for link in links {
            let number = *next;
            *next += 1;
            let _ = write!(refs, "{number} 0 R ");
            let head = format!(
                "/Border[{}]/H/I{}",
                numbers(&link.border),
                if link.color.is_empty() {
                    String::new()
                } else {
                    format!("/C[{}]", numbers(&link.color))
                }
            );
            let rect = numbers(&link.rect);
            let body = match &link.action {
                LinkAction::GoTo(name) => format!(
                    "<<\n/Type /Annot\n/Subtype /Link\n{head}\n/Rect [{rect}]\n/A << /S /GoTo /D {} >>\n>>",
                    string_literal(name.as_bytes())
                ),
                LinkAction::Uri(uri) => format!(
                    "<<\n/Type /Annot\n{head}\n/Rect [{rect}]\n/Subtype/Link/A<</Type/Action/S/URI/URI{}>>\n>>",
                    string_literal(uri.as_bytes())
                ),
            };
            plan.objects.push((number, body));
        }
        plan.page_entries[page] = format!(" /Annots [ {refs}]");
    }

    // Destinations and their name tree.
    let mut leaves: Vec<(usize, String, String)> = Vec::new();
    if !navigation.destinations.is_empty() {
        let mut named: Vec<(String, usize)> = Vec::new();
        for (name, destination) in &navigation.destinations {
            let number = *next;
            *next += 1;
            let view = match &destination.view {
                View::Xyz { left, top } => format!("/XYZ {left} {top} null"),
                View::Fit => "/Fit".to_string(),
                View::FitH { top } => format!("/FitH {top}"),
            };
            plan.objects.push((
                number,
                format!("<<\n/D [{} 0 R {view}]\n>>", page_object(destination.page)),
            ));
            named.push((name.clone(), number));
        }
        // Leaves, then parents, six at a time, until one root remains.
        let mut level: Vec<(usize, String, String)> = Vec::new();
        let single = named.len() <= NAME_TREE_KIDS_MAX;
        for chunk in named.chunks(NAME_TREE_KIDS_MAX) {
            let number = *next;
            *next += 1;
            let mut names = String::new();
            for (name, object) in chunk {
                if !names.is_empty() {
                    names.push(' ');
                }
                let _ = write!(names, "{} {object} 0 R", string_literal(name.as_bytes()));
            }
            let first = chunk[0].0.clone();
            let last = chunk[chunk.len() - 1].0.clone();
            let body = if single {
                format!("<<\n/Names [{names}]\n>>")
            } else {
                format!(
                    "<<\n/Names [{names}]\n/Limits [{} {}]\n>>",
                    string_literal(first.as_bytes()),
                    string_literal(last.as_bytes())
                )
            };
            plan.objects.push((number, body));
            level.push((number, first, last));
        }
        while level.len() > 1 {
            let mut parents = Vec::new();
            for chunk in level.chunks(NAME_TREE_KIDS_MAX) {
                let number = *next;
                *next += 1;
                let kids = chunk
                    .iter()
                    .map(|(n, ..)| format!("{n} 0 R"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let first = chunk[0].1.clone();
                let last = chunk[chunk.len() - 1].2.clone();
                plan.objects.push((
                    number,
                    format!(
                        "<<\n/Kids [{kids}]\n/Limits [{} {}]\n>>",
                        string_literal(first.as_bytes()),
                        string_literal(last.as_bytes())
                    ),
                ));
                parents.push((number, first, last));
            }
            level = parents;
        }
        leaves = level;
    }
    let names_dict = leaves.first().map(|(root, ..)| {
        let number = *next;
        *next += 1;
        plan.objects
            .push((number, format!("<<\n/Dests {root} 0 R\n>>")));
        number
    });

    // Outlines.
    let items = &navigation.outlines;
    let outline_root = if items.is_empty() {
        None
    } else {
        let parents = outline_parents(items);
        let numbers_of: Vec<usize> = items
            .iter()
            .map(|_| {
                let n = *next;
                *next += 1;
                n
            })
            .collect();
        let root = *next;
        *next += 1;
        // Children of each item, and of the root (`top`), in document order.
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); items.len()];
        let mut top: Vec<usize> = Vec::new();
        for (index, parent) in parents.iter().enumerate() {
            match parent {
                Some(p) => kids[*p].push(index),
                None => top.push(index),
            }
        }
        let open = |index: usize| {
            navigation.outlines_open
                && navigation
                    .outlines_open_level
                    .is_none_or(|level| items[index].level < level)
        };
        // Visible descendants of each open item: every child, plus the
        // visible descendants of the open ones. A child always comes after
        // its parent, so one reverse pass sees every child first.
        let mut visible = vec![0usize; items.len()];
        for index in (0..items.len()).rev() {
            visible[index] = kids[index]
                .iter()
                .map(|&k| 1 + if open(k) { visible[k] } else { 0 })
                .sum();
        }
        for (index, item) in items.iter().enumerate() {
            let siblings = parents[index].map_or(&top, |p| &kids[p]);
            let position = siblings.iter().position(|&s| s == index).unwrap_or(0);
            let mut body = format!(
                "<<\n/Title {}\n/A << /S /GoTo /D {} >>\n/Parent {} 0 R",
                text_string(&item.title),
                string_literal(item.destination.as_bytes()),
                parents[index].map_or(root, |p| numbers_of[p])
            );
            if position > 0 {
                let _ = write!(body, "\n/Prev {} 0 R", numbers_of[siblings[position - 1]]);
            }
            if position + 1 < siblings.len() {
                let _ = write!(body, "\n/Next {} 0 R", numbers_of[siblings[position + 1]]);
            }
            let own = &kids[index];
            if let (Some(first), Some(last)) = (own.first(), own.last()) {
                let count = if open(index) {
                    visible[index] as i64
                } else {
                    -(own.len() as i64)
                };
                let _ = write!(
                    body,
                    "\n/First {} 0 R\n/Last {} 0 R\n/Count {count}",
                    numbers_of[*first], numbers_of[*last]
                );
            }
            body.push_str("\n>>");
            plan.objects.push((numbers_of[index], body));
        }
        let visible: usize = top
            .iter()
            .map(|&t| 1 + if open(t) { visible[t] } else { 0 })
            .sum();
        plan.objects.push((
            root,
            format!(
                "<<\n/Type /Outlines\n/First {} 0 R\n/Last {} 0 R\n/Count {visible}\n>>",
                numbers_of[top[0]],
                numbers_of[*top.last().expect("an outline has a top-level item")]
            ),
        ));
        Some(root)
    };

    if let Some(root) = outline_root {
        let _ = write!(plan.catalog_entries, " /Outlines {root} 0 R");
    }
    if let Some(names) = names_dict {
        let _ = write!(plan.catalog_entries, " /Names {names} 0 R");
    }
    match navigation.page_mode {
        Some(PageMode::UseOutlines) => plan.catalog_entries.push_str(" /PageMode /UseOutlines"),
        Some(PageMode::UseNone) => plan.catalog_entries.push_str(" /PageMode /UseNone"),
        None => {}
    }
    if navigation.open_fit_first_page && page_count > 0 {
        let _ = write!(
            plan.catalog_entries,
            " /OpenAction [{} 0 R /Fit]",
            page_object(0)
        );
    }

    let _ = write!(
        plan.info_entries,
        " /Creator {}",
        info.creator
            .as_deref()
            .map_or_else(|| "(FlashTeX)".to_string(), text_string)
    );
    for (key, value) in [
        ("Title", &info.title),
        ("Author", &info.author),
        ("Subject", &info.subject),
        ("Keywords", &info.keywords),
    ] {
        if let Some(value) = value {
            let _ = write!(plan.info_entries, " /{key} {}", text_string(value));
        }
    }
    plan.objects.sort_by_key(|(number, _)| *number);
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_escape_delimiters_and_encode_non_ascii_as_utf16() {
        assert_eq!(string_literal(b"a(b)\\c"), "(a\\(b\\)\\\\c)");
        assert_eq!(string_literal(b"\x00\n"), "(\\000\\012)");
        assert_eq!(text_string("Ok"), "(Ok)");
        assert_eq!(text_string("é"), "(\\376\\377\\000\\351)");
    }

    #[test]
    fn outline_parents_follow_levels() {
        let item = |level| OutlineItem {
            title: String::new(),
            destination: String::new(),
            level,
        };
        let items = [item(1), item(2), item(3), item(2), item(1), item(0)];
        assert_eq!(
            outline_parents(&items),
            vec![None, Some(0), Some(1), Some(0), None, None]
        );
    }
}
