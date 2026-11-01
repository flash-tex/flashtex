//! `\includegraphics` in running text and the graphics box transforms as
//! structured inline nodes (`flashtex_compiler::graphics`).

use flashtex_compiler::graphics::{Graphic, TransformBox, TransformKind};
use flashtex_compiler::parser::{parse, Block, Inline};

fn inlines(src: &str) -> Vec<Inline> {
    let parsed = parse(src);
    let mut out = Vec::new();
    for b in parsed.blocks {
        if let Block::Paragraph(content) = b {
            out.extend(content);
        }
    }
    out
}

fn graphics(list: &[Inline]) -> Vec<&Graphic> {
    list.iter()
        .filter_map(|i| {
            if let Inline::Graphic(g) = i {
                Some(&**g)
            } else {
                None
            }
        })
        .collect()
}

fn transforms(list: &[Inline]) -> Vec<&TransformBox> {
    list.iter()
        .filter_map(|i| {
            if let Inline::Transform(t) = i {
                Some(&**t)
            } else {
                None
            }
        })
        .collect()
}

fn text_of(list: &[Inline]) -> String {
    list.iter()
        .filter_map(|i| {
            if let Inline::Text { text, .. } = i {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const PRE: &str = "\\documentclass{article}\n\\usepackage{graphicx}\n\\graphicspath{{images/}{figs/}}\n\\begin{document}\n";

#[test]
fn inline_includegraphics_keeps_keys_path_and_span() {
    let src = format!("{PRE}See \\includegraphics[width=0.5\\textwidth, trim={{1 2 3 4}}, clip]{{plots/a.png}} here.\n\\end{{document}}\n");
    let list = inlines(&src);
    let g = graphics(&list);
    assert_eq!(g.len(), 1, "{list:?}");
    assert_eq!(g[0].options, "width=0.5\\textwidth, trim={1 2 3 4}, clip");
    assert_eq!(g[0].path, "plots/a.png");
    assert!(!g[0].starred);
    assert!(g[0].space_before);
    assert_eq!(
        &src[g[0].span.start..g[0].span.end],
        "\\includegraphics[width=0.5\\textwidth, trim={1 2 3 4}, clip]{plots/a.png}"
    );
    assert_eq!(text_of(&list), "See here.");
    let parsed = parse(&src);
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|d| !d.message.contains("graphicspath") && !d.message.contains("includegraphics")),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn starred_and_bounding_box_forms() {
    let src = format!("{PRE}\\includegraphics*[10,20][110,70]{{b}}x\\includegraphics{{c.pdf}}\n\\end{{document}}\n");
    let list = inlines(&src);
    let g = graphics(&list);
    assert_eq!(g.len(), 2);
    assert!(g[0].starred);
    assert_eq!(g[0].options, "viewport=10 20 110 70");
    assert_eq!(g[0].path, "b");
    assert_eq!(g[1].options, "");
    assert!(!g[1].space_before);
}

#[test]
fn transforms_record_parameters_and_content() {
    let src = format!(
        "{PRE}A \\scalebox{{2}}[0.5]{{big \\textbf{{bold}}}} B \\resizebox*{{!}}{{2\\baselineskip}}{{r}} \\rotatebox[origin=c]{{-30}}{{rot $x$}} \\reflectbox{{m}} \\scalebox{{1.5}}{{\\rotatebox{{90}}{{n}}}}\n\\end{{document}}\n"
    );
    let list = inlines(&src);
    let t = transforms(&list);
    assert_eq!(t.len(), 5, "{list:?}");
    assert_eq!(
        t[0].kind,
        TransformKind::Scale {
            x: "2".into(),
            y: Some("0.5".into())
        }
    );
    assert_eq!(text_of(&t[0].content), "big bold");
    assert_eq!(
        t[1].kind,
        TransformKind::Resize {
            starred: true,
            width: "!".into(),
            height: "2\\baselineskip".into()
        }
    );
    assert_eq!(
        t[2].kind,
        TransformKind::Rotate {
            options: Some("origin=c".into()),
            angle: "-30".into()
        }
    );
    assert!(t[2]
        .content
        .iter()
        .any(|i| matches!(i, Inline::Math { .. })));
    assert_eq!(t[3].kind, TransformKind::Reflect);
    assert_eq!(
        t[4].kind,
        TransformKind::Scale {
            x: "1.5".into(),
            y: None
        }
    );
    let inner = transforms(&t[4].content);
    assert_eq!(inner.len(), 1);
    assert_eq!(
        inner[0].kind,
        TransformKind::Rotate {
            options: None,
            angle: "90".into()
        }
    );
    assert_eq!(
        &src[t[4].span.start..t[4].span.end],
        "\\scalebox{1.5}{\\rotatebox{90}{n}}"
    );
    assert_eq!(text_of(&list), "A B");
}

#[test]
fn edits_shift_graphic_and_transform_spans() {
    let old =
        format!("{PRE}One \\includegraphics{{a}} and \\scalebox{{2}}{{two}}.\n\\end{{document}}\n");
    let new = old.replace("One ", "Only one ");
    let (a, b) = (inlines(&old), inlines(&new));
    let (ga, gb) = (graphics(&a)[0].span, graphics(&b)[0].span);
    assert_eq!(gb.start, ga.start + 5);
    let (ta, tb) = (transforms(&a)[0], transforms(&b)[0]);
    assert_eq!(tb.span.start, ta.span.start + 5);
    assert_eq!(&new[tb.span.start..tb.span.end], "\\scalebox{2}{two}");
}
