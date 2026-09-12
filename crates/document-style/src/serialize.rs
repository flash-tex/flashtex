//! JSON form of a [`Stylesheet`]: the inputs (class options, geometry,
//! delta) round-trip exactly; a derived `export` section carries the page
//! layout and the resolved styles of canonical block paths for adapters.
//!
//! This schema is a proposal for a future compiler/PDF adapter and is not part
//! of runtime-v1.

use crate::fonts::BaseSize;
use crate::geometry::{ClassOptions, Geometry, PageLayout, Paper, Rect};
use crate::json::{JsonError, Value};
use crate::length::{Pt, Skip};
use crate::style::{
    Alignment, Block, DeltaRule, InlineStyle, ListKind, ListStyle, ResolvedStyle, StyleDelta,
    Stylesheet,
};

pub const SCHEMA: &str = "flashtex-document-style/1";

/// Canonical paths whose resolved styles are exported for adapters.
pub fn canonical_paths() -> Vec<(String, Vec<Block>)> {
    let mut paths: Vec<Vec<Block>> = vec![
        vec![Block::Document],
        vec![Block::Document, Block::Paragraph],
        vec![Block::Document, Block::ParagraphAfterHeading],
    ];
    for level in 1..=5 {
        paths.push(vec![Block::Document, Block::Heading(level)]);
    }
    for kind in [
        ListKind::Itemize,
        ListKind::Enumerate,
        ListKind::Description,
    ] {
        let mut prefix = vec![Block::Document];
        for _ in 1..=4 {
            prefix.push(Block::List(kind));
            paths.push(prefix.clone());
            let mut item = prefix.clone();
            item.push(Block::Item);
            paths.push(item.clone());
            item.push(Block::Paragraph);
            paths.push(item);
            prefix.push(Block::Item);
        }
    }
    for a in [Alignment::Center, Alignment::Left, Alignment::Right] {
        paths.push(vec![Block::Document, Block::Align(a)]);
    }
    for s in [InlineStyle::Emph, InlineStyle::Bold, InlineStyle::Italic] {
        paths.push(vec![Block::Document, Block::Paragraph, Block::Inline(s)]);
    }
    paths
        .into_iter()
        .map(|p| (p.iter().map(|b| b.name()).collect::<Vec<_>>().join("/"), p))
        .collect()
}

fn skip_json(s: Skip) -> Value {
    Value::obj()
        .set("pt", s.pt)
        .set("plus", s.plus)
        .set("minus", s.minus)
}

fn skip_from(v: &Value) -> Result<Skip, JsonError> {
    Ok(Skip {
        pt: num(v, "pt")?,
        plus: num(v, "plus")?,
        minus: num(v, "minus")?,
    })
}

fn opt_pt(v: Option<Pt>) -> Value {
    v.map(|p| p.0).into()
}

fn rect_json(r: Rect) -> Value {
    Value::obj()
        .set("x", r.x.0)
        .set("y", r.y.0)
        .set("width", r.width.0)
        .set("height", r.height.0)
}

pub fn page_layout_json(p: &PageLayout) -> Value {
    let (bx, by, bw, bh) = p.text_area_bp();
    let (pw, ph) = p.paper_bp();
    Value::obj()
        .set("paper", p.paper.name())
        .set("unit", "pt")
        .set("paper_width", p.paper_width.0)
        .set("paper_height", p.paper_height.0)
        .set("text_area", rect_json(p.text_area))
        .set("columns", p.columns as f64)
        .set("top_skip", p.top_skip.0)
        .set("first_baseline_y", p.first_baseline_y().0)
        .set("head_height", p.head_height.0)
        .set("head_sep", p.head_sep.0)
        .set("foot_skip", p.foot_skip.0)
        .set(
            "bp",
            Value::obj()
                .set("paper_width", pw)
                .set("paper_height", ph)
                .set(
                    "text_area",
                    Value::obj()
                        .set("x", bx)
                        .set("y", by)
                        .set("width", bw)
                        .set("height", bh),
                ),
        )
        .set(
            "latex",
            Value::obj()
                .set("textwidth", p.latex.textwidth.0)
                .set("textheight", p.latex.textheight.0)
                .set("oddsidemargin", p.latex.oddsidemargin.0)
                .set("evensidemargin", p.latex.evensidemargin.0)
                .set("topmargin", p.latex.topmargin.0)
                .set("headheight", p.latex.headheight.0)
                .set("headsep", p.latex.headsep.0)
                .set("footskip", p.latex.footskip.0)
                .set("topskip", p.latex.topskip.0)
                .set("marginparwidth", p.latex.marginparwidth.0)
                .set("marginparsep", p.latex.marginparsep.0),
        )
}

fn list_json(l: &ListStyle) -> Value {
    Value::obj()
        .set("kind", l.kind.name())
        .set("depth", l.depth as f64)
        .set("leftmargin", l.leftmargin.0)
        .set("labelwidth", l.labelwidth.0)
        .set("labelsep", l.labelsep.0)
        .set("topsep", skip_json(l.topsep))
        .set("partopsep", skip_json(l.partopsep))
        .set("parsep", skip_json(l.parsep))
        .set("itemsep", skip_json(l.itemsep))
}

pub fn resolved_json(s: &ResolvedStyle) -> Value {
    Value::obj()
        .set("font_size", s.font_size.0)
        .set("baselineskip", s.baselineskip.0)
        .set("bold", s.bold)
        .set("italic", s.italic)
        .set("parindent", s.parindent.0)
        .set("first_line_indent", s.first_line_indent)
        .set("space_before", skip_json(s.space_before))
        .set("space_after", skip_json(s.space_after))
        .set("alignment", s.alignment.name())
        .set("left_margin", s.left_margin.0)
        .set("right_margin", s.right_margin.0)
        .set("run_in_after", opt_pt(s.run_in_after))
        .set("list", s.list.as_ref().map(list_json))
}

fn geometry_json(g: &Geometry) -> Value {
    Value::obj()
        .set("margin", opt_pt(g.margin))
        .set("top", opt_pt(g.top))
        .set("bottom", opt_pt(g.bottom))
        .set("left", opt_pt(g.left))
        .set("right", opt_pt(g.right))
        .set("textwidth", opt_pt(g.textwidth))
        .set("textheight", opt_pt(g.textheight))
        .set("includehead", g.includehead)
        .set("includefoot", g.includefoot)
}

fn rule_json(r: &DeltaRule) -> Value {
    Value::obj()
        .set("block", r.block.map(|b| b.name()))
        .set("font_size", opt_pt(r.font_size))
        .set("baselineskip", opt_pt(r.baselineskip))
        .set("bold", r.bold)
        .set("italic", r.italic)
        .set("parindent", opt_pt(r.parindent))
        .set("first_line_indent", r.first_line_indent)
        .set("space_before", r.space_before.map(skip_json))
        .set("space_after", r.space_after.map(skip_json))
        .set("alignment", r.alignment.map(|a| a.name()))
}

fn delta_json(d: &StyleDelta) -> Value {
    Value::obj()
        .set("parskip", d.parskip.map(skip_json))
        .set("rules", d.rules.iter().map(rule_json).collect::<Vec<_>>())
}

impl Stylesheet {
    /// Full JSON: inputs plus the derived `export` (page layout, canonical
    /// resolved styles, heading baseline gaps).
    pub fn to_json_value(&self) -> Value {
        let mut styles = Value::obj();
        for (name, path) in canonical_paths() {
            let resolved = self
                .resolve(&path)
                .expect("canonical_paths() never exceeds MAX_LIST_NESTING_DEPTH");
            styles = styles.set(&name, resolved_json(&resolved));
        }
        let mut gaps = Value::obj();
        for level in 1..=3u8 {
            gaps = gaps.set(
                &format!("heading{level}"),
                Value::obj()
                    .set("before", skip_json(self.heading_gap_before(level)))
                    .set("after", skip_json(self.heading_gap_after(level))),
            );
        }
        Value::obj()
            .set("schema", SCHEMA)
            .set("class", "article")
            .set(
                "options",
                Value::obj()
                    .set("paper", self.options.paper.name())
                    .set("size", self.options.size.name()),
            )
            .set("geometry", self.geometry.as_ref().map(geometry_json))
            .set("delta", delta_json(&self.delta))
            .set(
                "export",
                Value::obj()
                    .set("unit", "pt")
                    .set("parskip", skip_json(self.parskip()))
                    .set("page", page_layout_json(&self.page_layout()))
                    .set("heading_baseline_gaps", gaps)
                    .set("styles", styles),
            )
    }

    pub fn to_json(&self) -> String {
        self.to_json_value().to_string_pretty()
    }

    /// Rebuild a stylesheet from its JSON inputs; `export` is ignored and
    /// regenerated on demand.
    pub fn from_json(text: &str) -> Result<Stylesheet, JsonError> {
        let v = Value::parse(text)?;
        let schema = v.get("schema").and_then(Value::as_str).unwrap_or("");
        if schema != SCHEMA {
            return Err(err(&format!("unsupported schema `{schema}`")));
        }
        let options = v.get("options").ok_or_else(|| err("missing options"))?;
        let paper = options
            .get("paper")
            .and_then(Value::as_str)
            .and_then(Paper::parse)
            .ok_or_else(|| err("bad options.paper"))?;
        let size = options
            .get("size")
            .and_then(Value::as_str)
            .and_then(BaseSize::parse)
            .ok_or_else(|| err("bad options.size"))?;
        let geometry = match v.get("geometry") {
            None | Some(Value::Null) => None,
            Some(g) => Some(Geometry {
                margin: opt_num(g, "margin")?.map(Pt),
                top: opt_num(g, "top")?.map(Pt),
                bottom: opt_num(g, "bottom")?.map(Pt),
                left: opt_num(g, "left")?.map(Pt),
                right: opt_num(g, "right")?.map(Pt),
                textwidth: opt_num(g, "textwidth")?.map(Pt),
                textheight: opt_num(g, "textheight")?.map(Pt),
                includehead: opt_bool(g, "includehead")?.unwrap_or(false),
                includefoot: opt_bool(g, "includefoot")?.unwrap_or(false),
            }),
        };
        let mut delta = StyleDelta::default();
        if let Some(d) = v.get("delta") {
            if let Some(p) = d.get("parskip").filter(|p| !p.is_null()) {
                delta.parskip = Some(skip_from(p)?);
            }
            for r in d.get("rules").and_then(Value::as_arr).unwrap_or(&[]) {
                let block = match r.get("block") {
                    None | Some(Value::Null) => None,
                    Some(b) => Some(
                        b.as_str()
                            .and_then(Block::parse)
                            .ok_or_else(|| err("bad delta rule block"))?,
                    ),
                };
                delta.rules.push(DeltaRule {
                    block,
                    font_size: opt_num(r, "font_size")?.map(Pt),
                    baselineskip: opt_num(r, "baselineskip")?.map(Pt),
                    bold: opt_bool(r, "bold")?,
                    italic: opt_bool(r, "italic")?,
                    parindent: opt_num(r, "parindent")?.map(Pt),
                    first_line_indent: opt_bool(r, "first_line_indent")?,
                    space_before: match r.get("space_before") {
                        None | Some(Value::Null) => None,
                        Some(s) => Some(skip_from(s)?),
                    },
                    space_after: match r.get("space_after") {
                        None | Some(Value::Null) => None,
                        Some(s) => Some(skip_from(s)?),
                    },
                    alignment: match r.get("alignment") {
                        None | Some(Value::Null) => None,
                        Some(a) => Some(
                            a.as_str()
                                .and_then(Alignment::parse)
                                .ok_or_else(|| err("bad delta rule alignment"))?,
                        ),
                    },
                });
            }
        }
        Ok(Stylesheet {
            options: ClassOptions { paper, size },
            geometry,
            delta,
        })
    }
}

fn err(message: &str) -> JsonError {
    JsonError {
        offset: 0,
        message: message.to_string(),
    }
}

fn num(v: &Value, key: &str) -> Result<f64, JsonError> {
    v.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| err(&format!("missing number `{key}`")))
}

fn opt_num(v: &Value, key: &str) -> Result<Option<f64>, JsonError> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Num(n)) => Ok(Some(*n)),
        Some(_) => Err(err(&format!("`{key}` must be a number or null"))),
    }
}

fn opt_bool(v: &Value, key: &str) -> Result<Option<bool>, JsonError> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(err(&format!("`{key}` must be a boolean or null"))),
    }
}
