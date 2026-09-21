//! Braced groups in math (TeX §1186): an ordinary atom unless they hold
//! only ordinary atoms, while the engine's `\begingroup` around a
//! `\begin{...}` forms no atom; `\bmod`'s `mod` states its Bin class.

use flashtex_compiler::math::{AtomClass, MathAtom, Nucleus};
use flashtex_compiler::parser::{parse, Block, Inline};

fn atoms(formula: &str) -> Vec<MathAtom> {
    let parsed = parse(&format!("\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\begin{{document}}\n{formula}\n\\end{{document}}\n"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    for block in &parsed.blocks {
        let Block::Paragraph(content) = block else { continue };
        for inline in content {
            if let Inline::Math { list, .. } = inline {
                return list.atoms.clone();
            }
        }
    }
    panic!("no formula in {formula}");
}

fn shape(formula: &str) -> Vec<&'static str> {
    atoms(formula)
        .iter()
        .map(|a| match &a.nucleus {
            Nucleus::Group(body) if body.atoms.is_empty() => "{}",
            Nucleus::Group(_) => "group",
            Nucleus::Matrix { .. } => "matrix",
            Nucleus::Symbol(_) => "symbol",
            Nucleus::Text(_) => "text",
            _ => "other",
        })
        .collect()
}

#[test]
fn a_group_holding_a_non_ordinary_atom_is_one_atom() {
    assert_eq!(shape("$a{=}b$"), ["symbol", "group", "symbol"]);
    assert_eq!(shape("$a{+}b$"), ["symbol", "group", "symbol"]);
    assert_eq!(shape("${}+1$"), ["{}", "symbol", "symbol"]);
    assert_eq!(shape("${a+b}^2$"), ["group"]);
}

#[test]
fn a_group_of_ordinary_atoms_still_flattens() {
    assert_eq!(shape("${ab}c$"), ["symbol", "symbol", "symbol"]);
}

#[test]
fn an_environment_forms_no_group_atom() {
    assert_eq!(shape("$\\begin{pmatrix} a \\end{pmatrix},b$"), ["matrix", "symbol", "symbol"]);
}

#[test]
fn bmod_is_binary() {
    let list = atoms("$a\\bmod b$");
    let modulo = list.iter().find(|a| matches!(&a.nucleus, Nucleus::Text(t) if t == "mod")).expect("mod");
    assert_eq!(modulo.class_override, Some(AtomClass::Bin));
}
