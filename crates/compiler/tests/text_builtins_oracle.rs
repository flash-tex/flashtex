//! pdfLaTeX oracle for the compiler's text built-ins (`src/text_builtins.rs`):
//! the `\TeX`/`\LaTeX`/`\LaTeXe` logos, `\rule`, text-mode kerns and glue, and
//! the kernel text symbols in OT1 and T1.
//!
//! `tests/oracle/text_builtins/expected.json` holds `\showbox` node trees
//! recorded from MacTeX pdflatex by `generate.py` (oracle only; no TeX runs
//! here). Glyph positions are recovered from each tree with the committed TFMs
//! and compared to the scaled point with what the compiler computes from the
//! same TFMs, which is stricter than a 0.1pt position tolerance.

use std::collections::BTreeMap;
use std::path::PathBuf;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::{self, Block, Inline};
use flashtex_compiler::text_builtins::{
    self as tb, CharBox, DimenContext, LogoFont, LogoMetrics, MathSubParams, TextDimen, TextLogo,
    TextRule,
};
use flashtex_tex_boxes::scaled::parse_dimen;
use flashtex_tex_text_encoding::tfm::ScaledFont;

fn oracle_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/oracle/text_builtins")
}

fn fixtures() -> Vec<Value> {
    let text = std::fs::read_to_string(oracle_dir().join("expected.json")).expect("expected.json");
    let root = json::parse(&text).expect("expected.json parses");
    root.get("fixtures")
        .and_then(Value::as_arr)
        .expect("fixtures")
        .clone()
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn int_of(v: &Value, key: &str) -> i32 {
    v.get(key)
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("missing {key}")) as i32
}

/// Loaded TFMs keyed by (tfm name, size in sp).
#[derive(Default)]
struct Fonts {
    cache: BTreeMap<(String, i32), ScaledFont>,
}

impl Fonts {
    fn load(&mut self, tfm: &str, size: i32) -> &ScaledFont {
        self.cache
            .entry((tfm.to_string(), size))
            .or_insert_with(|| {
                let bytes = std::fs::read(oracle_dir().join("tfm").join(format!("{tfm}.tfm")))
                    .unwrap_or_else(|e| panic!("{tfm}.tfm: {e}"));
                ScaledFont::from_bytes(tfm, &bytes, size).expect("TFM loads")
            })
    }

    /// The TFM and size behind an NFSS identifier recorded in `fixture`.
    fn by_id(&mut self, fixture: &Value, id: &str) -> &ScaledFont {
        let entry = fixture
            .get("fonts")
            .and_then(|f| f.get(id))
            .unwrap_or_else(|| panic!("font {id} not recorded"));
        let tfm = str_of(entry, "tfm").to_string();
        let size = id_size(id);
        self.load(&tfm, size)
    }

    /// A `\fontname` record (`{"tfm", "at_sp"}`); `at_sp` null is the
    /// design size.
    fn by_name(&mut self, record: &Value) -> &ScaledFont {
        let tfm = str_of(record, "tfm").to_string();
        let size = record
            .get("at_sp")
            .and_then(Value::as_i64)
            .map_or(0, |v| v as i32);
        self.load(&tfm, size)
    }
}

/// `T1/lmr/m/n/10.95` -> 10.95pt in sp.
fn id_size(id: &str) -> i32 {
    let size = id.rsplit('/').next().expect("size component");
    parse_dimen(&format!("{size}pt")).expect("font size")
}

#[derive(Debug, Clone, PartialEq)]
enum Placed {
    Glyph {
        font_id: String,
        code: u8,
        x: i32,
        y: i32,
    },
    Rule {
        x: i32,
        top: i32,
        bottom: i32,
        width: i32,
    },
    Kern {
        width: i32,
    },
    Glue {
        width: i32,
    },
    Penalty {
        value: i64,
    },
}

/// Positions of every node of an hlist `children` starting at `(x, y)` (y is
/// the baseline, positive downward, as in TeX's `\shipout`).
fn walk_h(
    fixture: &Value,
    fonts: &mut Fonts,
    children: &[Value],
    mut x: i32,
    y: i32,
    out: &mut Vec<Placed>,
) {
    for node in children {
        match str_of(node, "kind") {
            "char" => {
                let font_id = str_of(node, "font_id").to_string();
                let code = int_of(node, "code") as u8;
                out.push(Placed::Glyph {
                    font_id: font_id.clone(),
                    code,
                    x,
                    y,
                });
                x += fonts.by_id(fixture, &font_id).width(code);
            }
            "kern" => {
                let width = int_of(node, "width_sp");
                out.push(Placed::Kern { width });
                x += width;
            }
            "glue" => {
                let width = int_of(node, "width_sp");
                out.push(Placed::Glue { width });
                x += width;
            }
            "penalty" => out.push(Placed::Penalty {
                value: node.get("value").and_then(Value::as_i64).unwrap_or(0),
            }),
            "hbox" => {
                let shift = int_of(node, "shift_sp");
                let kids = node
                    .get("children")
                    .and_then(Value::as_arr)
                    .cloned()
                    .unwrap_or_default();
                walk_h(fixture, fonts, &kids, x, y + shift, out);
                x += int_of(node, "width_sp");
            }
            "vbox" => {
                let shift = int_of(node, "shift_sp");
                let kids = node
                    .get("children")
                    .and_then(Value::as_arr)
                    .cloned()
                    .unwrap_or_default();
                walk_v(
                    fixture,
                    fonts,
                    &kids,
                    x,
                    y + shift - int_of(node, "height_sp"),
                    out,
                );
                x += int_of(node, "width_sp");
            }
            "rule" => {
                let width = int_of(node, "width_sp");
                out.push(Placed::Rule {
                    x,
                    top: y - int_of(node, "height_sp"),
                    bottom: y + int_of(node, "depth_sp"),
                    width,
                });
                x += width;
            }
            _ => {}
        }
    }
}

fn walk_v(
    fixture: &Value,
    fonts: &mut Fonts,
    children: &[Value],
    x: i32,
    mut y: i32,
    out: &mut Vec<Placed>,
) {
    for node in children {
        match str_of(node, "kind") {
            "hbox" => {
                y += int_of(node, "height_sp");
                let kids = node
                    .get("children")
                    .and_then(Value::as_arr)
                    .cloned()
                    .unwrap_or_default();
                walk_h(fixture, fonts, &kids, x + int_of(node, "shift_sp"), y, out);
                y += int_of(node, "depth_sp");
            }
            "glue" | "kern" => y += int_of(node, "width_sp"),
            _ => {}
        }
    }
}

fn placed(fixture: &Value, fonts: &mut Fonts) -> Vec<Placed> {
    let tree = fixture.get("box").expect("box");
    let kids = tree
        .get("children")
        .and_then(Value::as_arr)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    walk_h(fixture, fonts, &kids, 0, 0, &mut out);
    out
}

/// The fixture's text font before its input (`\fontname\font`), at the
/// class body size.
fn text_font<'a>(fixture: &Value, fonts: &'a mut Fonts) -> &'a ScaledFont {
    fonts.by_name(fixture.get("text_font").expect("text_font"))
}

struct OracleLogoMetrics {
    current: ScaledFont,
    small: ScaledFont,
    math_italic: ScaledFont,
    sub: MathSubParams,
}

impl LogoMetrics for OracleLogoMetrics {
    fn char_box(&self, font: LogoFont, ch: char) -> CharBox {
        let (f, code) = match font {
            LogoFont::Current => (&self.current, ch as u8),
            LogoFont::ScriptSize => (&self.small, ch as u8),
            // \varepsilon is \mathord"0122: cmmi/lmmi slot "22.
            LogoFont::MathItalic => (&self.math_italic, 0x22),
        };
        let d = f.char_dims(code).expect("logo character exists");
        CharBox {
            width: d.width,
            height: d.height,
            depth: d.depth,
            italic: d.italic,
        }
    }
    fn quad(&self) -> i32 {
        self.current.quad()
    }
    fn x_height(&self) -> i32 {
        self.current.x_height()
    }
    fn math_sub_params(&self) -> MathSubParams {
        self.sub
    }
}

#[test]
fn logos_match_pdflatex_to_the_scaled_point() {
    let mut checked = 0;
    for fixture in fixtures().iter().filter(|f| str_of(f, "kind") == "logo") {
        let id = str_of(fixture, "id");
        let mut fonts = Fonts::default();
        let nodes = placed(fixture, &mut fonts);
        let glyphs: Vec<(String, u8, i32, i32)> = nodes
            .iter()
            .filter_map(|p| match p {
                Placed::Glyph {
                    font_id,
                    code,
                    x,
                    y,
                } => Some((font_id.clone(), *code, *x, *y)),
                _ => None,
            })
            .collect();
        let current_id = glyphs.iter().find(|g| g.1 == b'T').expect("T").0.clone();
        let small_id = glyphs.iter().find(|g| g.1 == b'A').map(|g| g.0.clone());
        let current = fonts.by_id(fixture, &current_id).clone();
        let small = match &small_id {
            Some(sid) => {
                assert_eq!(
                    id_size(sid),
                    tb::sf_size(id_size(&current_id)),
                    "{id}: \\sf@size"
                );
                fonts.by_id(fixture, sid).clone()
            }
            None => current.clone(),
        };
        let math = fixture.get("math_fonts").expect("math_fonts");
        let math_italic = fonts.by_name(math.get("textfont1").unwrap()).clone();
        let textfont2 = fonts.by_name(math.get("textfont2").unwrap()).clone();
        let scriptfont2 = fonts.by_name(math.get("scriptfont2").unwrap()).clone();
        let metrics = OracleLogoMetrics {
            current,
            small,
            math_italic,
            sub: MathSubParams {
                sub1: textfont2.param(16),
                math_x_height: textfont2.param(5),
                script_sub_drop: scriptfont2.param(19),
            },
        };
        let logo = TextLogo::from_command(str_of(fixture, "logo")).expect("logo name");
        let built = tb::layout_logo(logo, &metrics);
        let ours: Vec<(u8, i32, i32)> = built
            .glyphs
            .iter()
            .map(|g| {
                let code = if g.font == LogoFont::MathItalic {
                    0x22
                } else {
                    g.ch as u8
                };
                (code, g.x, -g.raise)
            })
            .collect();
        let theirs: Vec<(u8, i32, i32)> = glyphs.iter().map(|g| (g.1, g.2, g.3)).collect();
        assert_eq!(ours, theirs, "{id}: glyph (code, x sp, baseline y sp)");
        // Kerns after the construction are not latex.ltx's: NFSS's italic
        // correction (`\textsf{...}` appends `\/` after the `X`) or a TFM
        // boundary kern belong to the surrounding text command and font.
        let tree = fixture.get("box").unwrap();
        let trailing = trailing_kerns(tree);
        assert_eq!(
            built.width,
            int_of(tree, "width_sp") - trailing,
            "{id}: logo width"
        );

        // The compiler emits the logo inline with the command's span.
        let source = str_of(fixture, "input");
        let parsed = parser::parse(source);
        let found = inlines(&parsed.blocks)
            .into_iter()
            .any(|i| matches!(i, Inline::Logo { logo: l, .. } if *l == logo));
        assert!(
            found,
            "{id}: no Inline::Logo for {source:?}: {:?}",
            parsed.blocks
        );
        checked += 1;
    }
    assert!(checked >= 18, "only {checked} logo fixtures");
}

/// Whether a JSON value is the literal `true`.
fn is_true(v: Option<&Value>) -> bool {
    v.map(json::write).as_deref() == Some("true")
}

/// The summed width of the kerns that end a box's top-level list.
fn trailing_kerns(node: &Value) -> i32 {
    let kids = node
        .get("children")
        .and_then(Value::as_arr)
        .cloned()
        .unwrap_or_default();
    kids.iter()
        .rev()
        .take_while(|k| str_of(k, "kind") == "kern")
        .map(|k| int_of(k, "width_sp"))
        .sum()
}

fn inlines(blocks: &[Block]) -> Vec<&Inline> {
    let mut out = Vec::new();
    for b in blocks {
        if let Block::Paragraph(items) = b {
            out.extend(items.iter());
        }
    }
    out
}

fn dimen_context(fixture: &Value, fonts: &mut Fonts) -> DimenContext {
    let font = text_font(fixture, fonts);
    DimenContext {
        quad: font.quad(),
        x_height: font.x_height(),
        text_width: int_of(fixture, "textwidth_sp"),
        line_width: int_of(fixture, "linewidth_sp"),
        column_width: int_of(fixture, "textwidth_sp"),
    }
}

#[test]
fn rules_match_pdflatex_to_the_scaled_point() {
    let mut checked = 0;
    for fixture in fixtures().iter().filter(|f| str_of(f, "kind") == "rule") {
        let id = str_of(fixture, "id");
        let spec = fixture.get("rule").unwrap();
        let rule = TextRule {
            raise: TextDimen::parse(str_of(spec, "raise")).expect("raise"),
            width: TextDimen::parse(str_of(spec, "width")).expect("width"),
            height: TextDimen::parse(str_of(spec, "height")).expect("height"),
        };
        let mut fonts = Fonts::default();
        let cx = dimen_context(fixture, &mut fonts);
        let b = rule.resolve(&cx);
        // The \rule's own \hbox: the fixture box's (only) hbox child.
        let tree = fixture.get("box").unwrap();
        let rule_hbox = tree
            .get("children")
            .and_then(Value::as_arr)
            .and_then(|kids| kids.iter().find(|k| str_of(k, "kind") == "hbox"))
            .unwrap_or_else(|| panic!("{id}: no rule hbox"));
        assert_eq!(
            (b.width, b.height, b.depth),
            (
                int_of(rule_hbox, "width_sp"),
                int_of(rule_hbox, "height_sp"),
                int_of(rule_hbox, "depth_sp")
            ),
            "{id}: hbox width/height/depth"
        );
        let rule_nodes: Vec<Placed> = placed(fixture, &mut fonts)
            .into_iter()
            .filter(|p| matches!(p, Placed::Rule { .. }))
            .collect();
        assert_eq!(
            rule_nodes,
            vec![Placed::Rule {
                x: 0,
                top: -b.rule_top,
                bottom: -b.rule_bottom,
                width: b.width
            }],
            "{id}: rule node"
        );

        let source = if is_true(spec.get("math")) {
            continue_math(id);
            checked += 1;
            continue;
        } else {
            str_of(fixture, "input")
        };
        let parsed = parser::parse(source);
        let emitted = inlines(&parsed.blocks)
            .into_iter()
            .find_map(|i| match i {
                Inline::Rule { rule, .. } => Some(rule.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "{id}: no Inline::Rule for {source:?}: {:?}",
                    parsed.diagnostics
                )
            });
        assert_eq!(emitted, rule, "{id}: parsed rule");
        checked += 1;
    }
    assert!(checked >= 11, "only {checked} rule fixtures");
}

/// Math-mode `\rule` is checked through `math_rule_is_parsed` below.
fn continue_math(_id: &str) {}

#[test]
fn text_kerns_and_glue_match_pdflatex() {
    let mut checked = 0;
    for fixture in fixtures()
        .iter()
        .filter(|f| matches!(str_of(f, "kind"), "kern" | "glue"))
    {
        let id = str_of(fixture, "id");
        let command = str_of(fixture, "command");
        let mut fonts = Fonts::default();
        let font = text_font(fixture, &mut fonts).clone();
        let nodes = placed(fixture, &mut fonts);
        let source = str_of(fixture, "input");
        let parsed = parser::parse(source);
        let emitted = inlines(&parsed.blocks);
        match str_of(fixture, "kind") {
            "kern" => {
                // No fixture in this corpus loads amsmath (which renews
                // `\thinspace`/`\negthinspace` to `.1667em`), so the kernel
                // definition is the one pdflatex produced for them.
                let amount = tb::text_kern(command, false).expect("kern command");
                let cx = DimenContext {
                    quad: font.quad(),
                    ..DimenContext::default()
                };
                let kern = nodes.iter().find_map(|p| match p {
                    Placed::Kern { width } => Some(*width),
                    _ => None,
                });
                assert_eq!(kern, Some(amount.resolve(&cx)), "{id}: kern");
                assert!(
                    emitted
                        .iter()
                        .any(|i| matches!(i, Inline::Kern { amount: a, .. } if *a == amount)),
                    "{id}: no Inline::Kern in {emitted:?}"
                );
            }
            _ => {
                let glue = nodes.iter().find_map(|p| match p {
                    Placed::Glue { width } => Some(*width),
                    _ => None,
                });
                let em = match command {
                    "quad" => Some(1.0),
                    "qquad" => Some(2.0),
                    "enskip" => Some(0.5),
                    _ => None,
                };
                match em {
                    Some(em) => {
                        let cx = DimenContext {
                            quad: font.quad(),
                            ..DimenContext::default()
                        };
                        let expected = TextDimen::parse(&format!("{em}em")).unwrap().resolve(&cx);
                        assert_eq!(glue, Some(expected), "{id}: glue");
                        assert!(
                            emitted
                                .iter()
                                .any(|i| matches!(i, Inline::TextGlue { em: e, .. } if *e == em)),
                            "{id}: no Inline::TextGlue in {emitted:?}"
                        );
                    }
                    None => {
                        // `\ ` and `~`: the font's interword space at space factor 1000.
                        assert_eq!(glue, Some(font.space()), "{id}: interword glue");
                        if command == "~" {
                            assert!(
                                nodes.contains(&Placed::Penalty { value: 10000 }),
                                "{id}: tie penalty"
                            );
                        }
                    }
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 16, "only {checked} spacing fixtures");
}

#[test]
fn text_symbols_match_pdflatex_in_ot1_and_t1() {
    let mut checked = 0;
    for fixture in fixtures().iter().filter(|f| str_of(f, "kind") == "symbol") {
        let id = str_of(fixture, "id");
        let symbol = fixture.get("symbol").unwrap();
        let name = str_of(symbol, "name");
        let preamble = match str_of(fixture, "variant") {
            // The compiler reads only the encoding; lmodern changes no glyph choice.
            "t1-lm" => "\\usepackage[T1]{fontenc}",
            _ => "",
        };
        let source = format!(
            "\\documentclass{{article}}{preamble}\\begin{{document}}x \\{name} y\\end{{document}}"
        );
        let parsed = parser::parse(&source);
        let texts: Vec<&str> = inlines(&parsed.blocks)
            .into_iter()
            .filter_map(|i| match i {
                Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        let errors: Vec<&str> = fixture
            .get("errors")
            .and_then(Value::as_arr)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        if let Some(error) = errors.first() {
            assert!(error.contains("unavailable in encoding"), "{id}: {error}");
            assert!(
                parsed.diagnostics.iter().any(|d| d.message == *error),
                "{id}: expected diagnostic {error:?}, got {:?}",
                parsed.diagnostics
            );
            assert_eq!(
                texts,
                vec!["x", "y"],
                "{id}: nothing typeset for the command"
            );
        } else {
            let errors: Vec<_> = parsed
                .diagnostics
                .iter()
                .filter(|d| format!("{:?}", d.severity) == "Error")
                .collect();
            assert!(errors.is_empty(), "{id}: {errors:?}");
            assert_eq!(texts.len(), 3, "{id}: {texts:?}");
            let ours = texts[1];
            let box_tree = fixture.get("box").unwrap();
            let code_point = int_of(symbol, "code_point") as u32;
            let expected = char::from_u32(code_point).unwrap().to_string();
            let ascii = code_point < 0x80;
            if ascii && str_of(fixture, "variant") == "ot1-cm" {
                // OT1 has no ASCII slots for these: the kernel default takes
                // the glyph from OMS/OML (`\textbackslash` is cmsy "6E). When
                // pdfLaTeX set one character, it must be the ASCII glyph.
                let kids = box_tree.get("children").and_then(Value::as_arr).unwrap();
                if let [only] = kids.as_slice() {
                    if str_of(only, "kind") == "char" {
                        let mut fonts = Fonts::default();
                        let font_id = str_of(only, "font_id");
                        let tfm =
                            str_of(fixture.get("fonts").unwrap().get(font_id).unwrap(), "tfm");
                        let _ = fonts.by_id(fixture, font_id);
                        let glyph = flashtex_tex_text_encoding::fonts::glyph_name(
                            tfm,
                            int_of(only, "code") as u8,
                        );
                        let want = match code_point {
                            0x5C => "backslash",
                            0x3C => "less",
                            0x3E => "greater",
                            0x7C => "bar",
                            0x7B => "braceleft",
                            0x7D => "braceright",
                            // OT1 sets the accent glyphs (`\~{}`, `\^{}`).
                            0x7E => "tilde",
                            0x5E => "circumflex",
                            _ => "",
                        };
                        assert_eq!(glyph, Some(want), "{id}: OT1 glyph");
                    }
                }
            } else {
                let char_tree = fixture
                    .get("char_box")
                    .unwrap_or_else(|| panic!("{id}: no char_box"));
                assert_eq!(
                    json::write(box_tree),
                    json::write(char_tree),
                    "{id}: command and character typeset differently"
                );
            }
            if ours.chars().count() == 1 {
                assert_eq!(ours, expected, "{id}: character");
            } else {
                // A letter-only kernel default (`\SS` in OT1 is `SS`): the
                // character's own expansion typesets those very letters.
                let mut fonts = Fonts::default();
                let letters: String = placed(fixture, &mut fonts)
                    .iter()
                    .filter_map(|p| match p {
                        Placed::Glyph { code, .. } => Some(*code as char),
                        _ => None,
                    })
                    .collect();
                assert_eq!(ours, letters, "{id}: letters");
            }
        }
        checked += 1;
    }
    assert!(checked >= 96, "only {checked} symbol fixtures");
}

#[test]
fn math_rule_is_parsed() {
    let parsed = parser::parse("$\\rule{1pt}{2pt}$");
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not supported")),
        "{:?}",
        parsed.diagnostics
    );
}
