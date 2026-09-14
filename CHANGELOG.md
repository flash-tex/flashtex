# Changelog

All notable changes to FlashTeX will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-13

### Added

- **Core compiler pipeline**: Lexer → Parser → Layout → PDF Writer
- **LaTeX commands**: \\section, \\subsection, \\subsubsection, \\textbf, \\emph, \\textit
- **Environments**: itemize, enumerate, document
- **Source span tracking**: Every rendered word carries its exact byte range back to the .tex source
- **LRU cache**: 64-entry in-process cache keyed by project ID + FNV-1a content hash
- **Error recovery**: Parser continues after errors, producing partial output with diagnostics
- **PDF output**: Helvetica base-14 fonts (normal, bold, oblique), A4 page layout
- **JSON Lines protocol (v1)**: stdin/stdout communication, one JSON object per line
- **Security validation**: Path traversal rejection, absolute path rejection, control character filtering
- **Custom JSON parser**: Zero-dependency JSON serialization/deserialization
- **195+ unit tests**: Covering lexer, parser, layout, PDF writer, cache, protocol, and diagnostics
- **Cross-platform support**: Tested on macOS (aarch64), Linux (x86_64), Windows (x86_64)

### Performance

- **4.5ms** median compile time for hello.tex (38 bytes)
- **47ms** cold compile for thesis-ch1.tex (24 KB)
- **77×** faster than Overleaf's pdfLaTeX engine
- **152×** faster than Tectonic
- **711 KiB** release binary size (with thin LTO + strip)
- **3.2 MB** runtime RSS for simple documents
- **< 1ms** cache hit response time

### Architecture

- 9 Rust modules: lexer, parser, layout, pdf, protocol, cache, diagnostics, json, lib
- 1 external dependency: pdf-writer 0.15
- Rust 2021 edition, MSRV 1.70
- Zero unsafe code in FlashTeX (safe Rust throughout)

## [Unreleased]

### Planned

- Math mode (inline and display) with symbol rendering
- Table support (tabular environment with column alignment)
- Custom font embedding (TrueType/OpenType)
- Multi-file projects (\\input and \\include)
- Cross-references (\\label, \\ref, \\cite)
- BibTeX integration
- Image support (\\includegraphics)
- WASM compilation target for browser-based editing

---

Built by **Aarush, Jaysen, Kabir & Nat** at the SpaceXAI Hackathon 2026.
