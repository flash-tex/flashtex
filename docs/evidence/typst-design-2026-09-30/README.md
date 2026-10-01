# Typst design evidence, 2026-09-30

Input to DESIGN.md §15 ("Typst support", analysed design). Three tracks ran in parallel on
mac-m5pro-kabir (M5 Pro, 15 cores, 24 GB) against Typst **0.15.1**. The machine was busy
with other agents' benchmarks (1-minute load 8–60), so absolute times may be 5–30% high.
Ratios were taken back to back and hold.

| Track | File | Question | Headline |
|---|---|---|---|
| A, engine integration (measured) | [`track-a-engine.md`](track-a-engine.md), [`raw-a/`](raw-a/), [`prototype/`](prototype/) | How does Typst plug in behind `display-list-v3`, how fast, how exact? | Whole-document compile per edit. p95 at 10/100/300/1,000 pages: standard 4.8–5.2 / 53–60 / 205–227 / 1,000–1,400 ms; seeded 1-pass loop 2.0–2.6 / 16.4–17.4 / 60–64 / 353–387 ms, identical on 192/192 edits. PDF-derived f64 origins give 0 differing pixels at 2×. v3 gaps E1–E8, with "PDF islands" for gradients, tilings, SVG and colour glyphs. |
| B, editing UX parity | [`track-b-ux.md`](track-b-ux.md) | Can Typst editing feel as good as LaTeX, and vice versa? | Two tiers: in-app syntax (Swift for LaTeX, a `typst-syntax` Rust library for Typst) and host-served `lang-v1` semantics (typst-ide; tinymist-query optional). LaTeX gains ranked, with structured diagnostics first because v3's `DIAGNOSTIC` has no column. |
| C, red-team and licensing | [`track-c-redteam.md`](track-c-redteam.md) | What in the first §15 sketch is wrong or risky? | v3 is not engine-neutral; Apache-2.0 *is* GPLv3-compatible, so the real risk is the shared MIT crate; the GPL-3 NewCM10 font must not be compiled in; pin Typst per project (12/94 older templates break on 0.15.1); 7.3% of Universe packages are GPL-family; trademark needs Typst GmbH's authorization for commercial use. |

Where the tracks differ: Track C measured `typst watch` on a lighter document without a
bibliography (28.7 ms median, 79.4 ms p95 at 301 pages, one-page PNG). Track A measured
in-process on citation-heavy documents (Track A §2.2). Track A's ablation shows the cost
grows with introspection density: at 300 pages, p50 is 33.5 ms bare and 190 ms full.
DESIGN §15.3 uses Track A's numbers.

## Prototype

[`prototype/`](prototype/) holds the Track A embedder (`tbench`, Rust, over the unmodified
`=0.15.1` crates) and the Core Graphics pixel-diff harness. It is **MIT-licensed evidence**:
it is not linked into, or built by, anything else in the repository. It has its own
`[workspace]` and `Cargo.lock`, and it is outside the root workspace's members. Only source
and build instructions are committed; generated documents, PDFs, PNGs and `target/` are
not. See [`prototype/README.md`](prototype/README.md) to reproduce the numbers.
