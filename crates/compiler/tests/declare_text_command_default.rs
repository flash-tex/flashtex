//! `\DeclareTextCommandDefault`, `\ProvideTextCommandDefault` and
//! `\DeclareTextSymbolDefault` (ltoutenc.dtx): a declared default command
//! expands to its body with no diagnostics, exactly like pdflatex.
//!
//! Oracles (TeX Live 2026, `pdflatex -interaction=nonstopmode`, all under
//! `\documentclass{article}`):
//! - `\DeclareTextCommandDefault{\textfoo}{FOO}` + `A \textfoo{} B`
//!   -> 0 errors, `A FOO B`.
//! - `\ProvideTextCommandDefault{\textbaz}{BAZ}` (fresh) + `A \textbaz{} B`
//!   -> 0 errors, `A BAZ B`.
//! - `\ProvideTextCommandDefault{\textbar}{BAR}` + `A \textbar{} B`
//!   -> 0 errors, `A|B` (the kernel `|` wins under the default encoding).
//! - `\DeclareTextCommandDefault` twice (`FOO` then `BAR`) -> 0 errors,
//!   `A BAR B`; `\ProvideTextCommandDefault` twice -> 0 errors, `A FOO B`.
//! - `\DeclareTextCommandDefault{\textfoo}[1]{Hi #1!}` + `X \textfoo{you} Y`
//!   -> 0 errors, `X Hi you! Y`.
//! - `\DeclareTextSymbol{\textdollar}{OT1}{36}` plus
//!   `\DeclareTextSymbolDefault{\textdollar}{OT1}` + `A \textdollar{} B`
//!   -> 0 errors, `A$B`.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn words(out: &CompileOutput) -> String {
    out.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.text.clone())
        .collect::<Vec<_>>()
        .join("")
}

fn compile_body(preamble: &str, body: &str) -> CompileOutput {
    compile(&format!(
        "\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    ))
}

#[test]
fn declared_default_command_expands_to_its_body() {
    let out = compile_body(
        "\\DeclareTextCommandDefault{\\textfoo}{FOO}",
        "A \\textfoo{} B",
    );
    assert!(
        out.diagnostics.is_empty(),
        "no diagnostics: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), "AFOOB", "pdflatex typesets `A FOO B`");
}

#[test]
fn provide_default_defines_a_fresh_command() {
    let out = compile_body(
        "\\ProvideTextCommandDefault{\\textbaz}{BAZ}",
        "A \\textbaz{} B",
    );
    assert!(
        out.diagnostics.is_empty(),
        "no diagnostics: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), "ABAZB", "pdflatex typesets `A BAZ B`");
}

#[test]
fn provide_default_keeps_the_existing_text_symbol() {
    let out = compile_body(
        "\\ProvideTextCommandDefault{\\textbar}{BAR}",
        "A \\textbar{} B",
    );
    assert!(
        out.diagnostics.is_empty(),
        "no diagnostics: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), "A|B", "pdflatex typesets `A|B`");
}

#[test]
fn redeclare_default_last_wins_silently() {
    let out = compile_body(
        "\\DeclareTextCommandDefault{\\textfoo}{FOO}\n\\DeclareTextCommandDefault{\\textfoo}{BAR}",
        "A \\textfoo{} B",
    );
    assert!(
        out.diagnostics.is_empty(),
        "no diagnostics: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), "ABARB", "pdflatex typesets `A BAR B`");
}

#[test]
fn reprovide_default_first_wins_silently() {
    let out = compile_body(
        "\\ProvideTextCommandDefault{\\textfoo}{FOO}\n\\ProvideTextCommandDefault{\\textfoo}{BAR}",
        "A \\textfoo{} B",
    );
    assert!(
        out.diagnostics.is_empty(),
        "no diagnostics: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), "AFOOB", "pdflatex typesets `A FOO B`");
}

#[test]
fn declared_default_command_takes_arguments() {
    let out = compile_body(
        "\\DeclareTextCommandDefault{\\textfoo}[1]{Hi #1!}",
        "X \\textfoo{you} Y",
    );
    assert!(
        out.diagnostics.is_empty(),
        "no diagnostics: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), "XHiyou!Y", "pdflatex typesets `X Hi you! Y`");
}

#[test]
fn symbol_default_with_paired_symbol_expands() {
    let out = compile_body(
        "\\DeclareTextSymbol{\\textdollar}{OT1}{36}\n\\DeclareTextSymbolDefault{\\textdollar}{OT1}",
        "A \\textdollar{} B",
    );
    assert!(
        out.diagnostics.is_empty(),
        "no diagnostics: {:?}",
        out.diagnostics
    );
    assert_eq!(words(&out), "A$B", "pdflatex typesets `A$B`");
}
