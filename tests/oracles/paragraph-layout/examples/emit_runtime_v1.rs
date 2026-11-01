//! Emits the runtime-v1 `compile_result` for the page-geometry oracle
//! document (`docs/pages-default.tex` body, article 12pt Letter default
//! margins) in PDF points, one item per word. Used by
//! `tools/preview_pdf_agreement.sh` to feed `flashtex-pdf` and the CoreText
//! renderer with identical positions.
//!
//! Usage: cargo run --example emit_runtime_v1 [geometry1in] > out.json

use flashtex_document_style::{BaseSize, ClassOptions, Geometry, Paper, Pt};
use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::document::{DocumentSpec, layout_document};
use flashtex_paragraph_layout::runtime_v1::compile_result_json;
use flashtex_paragraph_layout::style::ArticleLayout;
use flashtex_paragraph_layout::*;

const BODY: &str = "Reproducibility and verification are the characteristic responsibilities of any typographical documentation effort, and the international community expects deliberately measured compilation results rather than approximations of representative behaviour. Hyphenation boundaries determine whether an unremarkable paragraph fits comfortably or overflows; consequently the implementation compares its automatically generated discretionary breaks against the reference engine's independently computed positions.\n\nDeliberately unbalanced paragraphs demonstrate emergency stretchability: extraordinarily incomprehensible terminology, uncharacteristically interdisciplinary collaboration, and counterproductive institutionalization complicate justification considerably, particularly when consecutive polysyllabic constructions eliminate conventional opportunities for satisfactory interword adjustment throughout the measurement.\n\nThis paragraph is deliberately long so that a real TeX engine and the FlashTeX compiler both have to break it into several lines, which lets the comparison tool report where each line starts and how far every word drifts from the oracle position. It keeps going with ordinary prose, a few longer words such as verification and reproducibility, and finally ends here.\n\nShort last paragraph with fine coffee and an em dash \u{2014} done.\n\nReproducibility and verification are the characteristic responsibilities of any typographical documentation effort, and the international community expects deliberately measured compilation results rather than approximations of representative behaviour. Hyphenation boundaries determine whether an unremarkable paragraph fits comfortably or overflows; consequently the implementation compares its automatically generated discretionary breaks against the reference engine's independently computed positions.\n\nDeliberately unbalanced paragraphs demonstrate emergency stretchability: extraordinarily incomprehensible terminology, uncharacteristically interdisciplinary collaboration, and counterproductive institutionalization complicate justification considerably, particularly when consecutive polysyllabic constructions eliminate conventional opportunities for satisfactory interword adjustment throughout the measurement.\n\nThis paragraph is deliberately long so that a real TeX engine and the FlashTeX compiler both have to break it into several lines, which lets the comparison tool report where each line starts and how far every word drifts from the oracle position. It keeps going with ordinary prose, a few longer words such as verification and reproducibility, and finally ends here.\n\nShort last paragraph with fine coffee and an em dash \u{2014} done.\n\nReproducibility and verification are the characteristic responsibilities of any typographical documentation effort, and the international community expects deliberately measured compilation results rather than approximations of representative behaviour. Hyphenation boundaries determine whether an unremarkable paragraph fits comfortably or overflows; consequently the implementation compares its automatically generated discretionary breaks against the reference engine's independently computed positions.\n\nDeliberately unbalanced paragraphs demonstrate emergency stretchability: extraordinarily incomprehensible terminology, uncharacteristically interdisciplinary collaboration, and counterproductive institutionalization complicate justification considerably, particularly when consecutive polysyllabic constructions eliminate conventional opportunities for satisfactory interword adjustment throughout the measurement.\n\nThis paragraph is deliberately long so that a real TeX engine and the FlashTeX compiler both have to break it into several lines, which lets the comparison tool report where each line starts and how far every word drifts from the oracle position. It keeps going with ordinary prose, a few longer words such as verification and reproducibility, and finally ends here.\n\nShort last paragraph with fine coffee and an em dash \u{2014} done.\n\nReproducibility and verification are the characteristic responsibilities of any typographical documentation effort, and the international community expects deliberately measured compilation results rather than approximations of representative behaviour. Hyphenation boundaries determine whether an unremarkable paragraph fits comfortably or overflows; consequently the implementation compares its automatically generated discretionary breaks against the reference engine's independently computed positions.\n\nDeliberately unbalanced paragraphs demonstrate emergency stretchability: extraordinarily incomprehensible terminology, uncharacteristically interdisciplinary collaboration, and counterproductive institutionalization complicate justification considerably, particularly when consecutive polysyllabic constructions eliminate conventional opportunities for satisfactory interword adjustment throughout the measurement.\n\nThis paragraph is deliberately long so that a real TeX engine and the FlashTeX compiler both have to break it into several lines, which lets the comparison tool report where each line starts and how far every word drifts from the oracle position. It keeps going with ordinary prose, a few longer words such as verification and reproducibility, and finally ends here.\n\nShort last paragraph with fine coffee and an em dash \u{2014} done.";

fn main() {
    let geometry = std::env::args().nth(1).as_deref() == Some("geometry1in");
    let article = ArticleLayout::new(
        ClassOptions {
            paper: Paper::Letter,
            size: BaseSize::Pt12,
        },
        geometry.then(|| Geometry::margin(Pt::inches(1.0))),
    );
    let hyph = LiangHyphenator::en_us_subset();
    let spec = DocumentSpec {
        text: BODY,
        font: &Core14Times::ROMAN,
        size: article.body.font_size.0,
        hyphenator: &hyph,
        line: article.line.clone(),
        page: article.page.clone(),
    };
    let doc = layout_document(&spec);
    let path = if geometry {
        "pages-geometry1in.tex"
    } else {
        "pages-default.tex"
    };
    print!(
        "{}",
        compile_result_json(
            &doc.pages,
            BODY,
            path,
            "paragraph-layout-oracle",
            1,
            BP_PER_TEX_PT
        )
    );
}
