# FlashTeX Architecture

> Internal design document for the FlashTeX compiler pipeline.

## Overview

FlashTeX is a from-scratch LaTeX subset compiler written in pure Rust. Unlike traditional TeX toolchains that shell out to external binaries (pdflatex, xelatex, lualatex), FlashTeX runs the entire pipeline in a single process with zero external dependencies.

The compiler follows a classic four-stage pipeline:

```
.tex source → Lexer → Parser → Layout Engine → PDF Writer → .pdf output
```

Each stage is a separate Rust module with well-defined input/output types and zero shared mutable state between stages.

## Stage 1: Lexer (`src/lexer.rs`)

The lexer transforms raw UTF-8 .tex input into a flat stream of `Token` values.

### Token Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Command(String),          // \section, \textbf, \emph, etc.
    OpenBrace,                // {
    CloseBrace,               // }
    Text(String),             // Plain text content
    Whitespace,               // Spaces, tabs
    Newline,                  // Single newline
    ParagraphBreak,           // Double newline (blank line)
    Comment(String),          // % comment
    MathInline(String),       // $...$
    MathDisplay(String),      // $$...$$
    BeginEnvironment(String), // \begin{...}
    EndEnvironment(String),   // \end{...}
    SpecialChar(char),        // &, #, _, ^, ~
    EOF,
}
```

### Source Span Tracking

Every token carries its byte offset range in the original source:

```rust
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    pub file: FileId,
}
```

This enables click-to-source in the rendered PDF — every rendered word can trace back to exactly where it was defined in the .tex file.

### Lexer Implementation

The lexer is a hand-written single-pass scanner (no regex, no parser generator). It processes the input left-to-right, byte by byte, classifying characters:

- `\\` starts a command name (reads until non-alphabetic)
- `{` and `}` are brace tokens
- `%` starts a comment (reads until newline)
- `$` toggles math mode
- Two consecutive newlines become a paragraph break
- Everything else is text

Performance: the lexer processes approximately 150 MB/s of .tex input on a single core (M1 MacBook Pro benchmark).

## Stage 2: Parser (`src/parser.rs`)

The parser consumes the token stream and builds a typed AST.

### AST Node Types

```rust
#[derive(Debug, Clone)]
pub enum Node {
    Document(Vec<Node>),
    Section {
        level: SectionLevel,
        title: Vec<Node>,
        body: Vec<Node>,
        span: Span,
    },
    Paragraph(Vec<Node>),
    Text {
        content: String,
        span: Span,
    },
    Bold(Vec<Node>),
    Italic(Vec<Node>),
    Emphasis(Vec<Node>),
    List {
        ordered: bool,
        items: Vec<Vec<Node>>,
    },
    Environment {
        name: String,
        args: Vec<String>,
        body: Vec<Node>,
    },
    Command {
        name: String,
        args: Vec<Vec<Node>>,
        span: Span,
    },
    Math {
        display: bool,
        content: String,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum SectionLevel {
    Section,      // \section
    Subsection,   // \subsection
    Subsubsection, // \subsubsection
}
```

### Error Recovery

The parser implements panic-mode error recovery:

1. When an unexpected token is encountered, record a diagnostic
2. Skip tokens until a synchronization point (next command, next paragraph break, or next brace match)
3. Continue parsing from the synchronization point

This means a single typo doesn't prevent the rest of the document from rendering. The partial output is returned alongside the diagnostics array, so editors can show both the (partial) PDF and the error locations.

### Parsing Strategy

The parser uses recursive descent with no backtracking. Each `parse_*` function consumes tokens and returns a `Node`:

```rust
fn parse_command(&mut self) -> Result<Node, Diagnostic> {
    let cmd_token = self.expect(Token::Command(_))?;
    match cmd_token.as_str() {
        "section" => self.parse_section(SectionLevel::Section),
        "subsection" => self.parse_section(SectionLevel::Subsection),
        "textbf" => self.parse_font_command(|nodes| Node::Bold(nodes)),
        "emph" | "textit" => self.parse_font_command(|nodes| Node::Italic(nodes)),
        "begin" => self.parse_environment(),
        _ => Ok(Node::Command {
            name: cmd_token.to_string(),
            args: self.parse_args()?,
            span: cmd_token.span,
        }),
    }
}
```

## Stage 3: Layout Engine (`src/layout.rs`)

The layout engine transforms the AST into positioned page items with exact coordinates.

### Page Model

```rust
pub struct Page {
    pub width_pt: f64,   // Default: 595.276 (A4)
    pub height_pt: f64,  // Default: 841.89 (A4)
    pub items: Vec<PageItem>,
}

pub struct PageItem {
    pub x_pt: f64,
    pub y_pt: f64,
    pub content: PageContent,
    pub source: Option<Span>,
}

pub enum PageContent {
    Text {
        text: String,
        font_size_pt: f64,
        font_style: FontStyle,
    },
    Rule {
        width_pt: f64,
        height_pt: f64,
    },
    Space(f64),
}
```

### Layout Algorithm

1. **Margin calculation**: 1-inch margins on all sides (72pt)
2. **Line breaking**: Greedy algorithm — fill each line left-to-right until the text width is exceeded, then break at the last whitespace
3. **Font metrics**: Helvetica character widths from a built-in lookup table (no font file parsing)
4. **Section formatting**: Section titles get 17pt bold, subsections 14pt bold, body text 12pt normal
5. **Paragraph spacing**: 12pt gap between paragraphs
6. **Page breaking**: When the current y-position exceeds the bottom margin, start a new page

### Font Metrics Table

Instead of parsing .afm or .ttf files, FlashTeX includes a compile-time lookup table of Helvetica glyph widths for ASCII characters. This covers the vast majority of Latin-script documents and keeps the binary dependency-free.

```rust
const HELVETICA_WIDTHS: [u16; 256] = [
    // Each entry is the character width in units of 1/1000 of a point
    // at 1pt font size. Multiply by font_size_pt to get actual width.
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,     // 0-15
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,     // 16-31
    278, 278, 355, 556, 556, 889, 667, 191,               // 32-39 (space ! " # $ % & ')
    333, 333, 389, 584, 278, 333, 278, 278,               // 40-47 (( ) * + , - . /)
    556, 556, 556, 556, 556, 556, 556, 556,               // 48-55 (0-7)
    556, 556, 278, 278, 584, 584, 584, 556,               // 56-63 (8 9 : ; < = > ?)
    // ... (full table in source)
];
```

## Stage 4: PDF Writer (`src/pdf.rs`)

The PDF writer serializes the laid-out pages into a valid PDF 1.4 byte stream.

### PDF Structure

```
%PDF-1.4
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [...] /Count N >> endobj
3 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj
4 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >> endobj
5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Oblique >> endobj
6 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842]
          /Contents 7 0 R /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> >>
7 0 obj << /Length ... >> stream
  BT /F1 12 Tf 72 769.89 Td (Hello World) Tj ET
endstream endobj
...
xref
0 N
trailer << /Size N /Root 1 0 R >>
startxref
...
%%EOF
```

### Font Strategy

FlashTeX uses the PDF base-14 fonts (Helvetica, Helvetica-Bold, Helvetica-Oblique). These fonts are guaranteed to be available in every PDF reader without embedding, which keeps the output PDF extremely small.

| Font | Usage |
|------|-------|
| Helvetica | Normal body text |
| Helvetica-Bold | \textbf{}, section titles |
| Helvetica-Oblique | \emph{}, \textit{} |

### Cross-Reference Table

The xref table is built incrementally as objects are written. Each entry records the byte offset of the corresponding object, enabling random access by PDF readers.

## Caching Layer (`src/cache.rs`)

FlashTeX includes a 64-entry in-process LRU cache.

### Cache Key

```rust
pub struct CacheKey {
    pub project_id: String,
    pub content_hash: u64,  // FNV-1a hash of the concatenated .tex sources
}
```

### Cache Entry

```rust
pub struct CacheEntry {
    pub pages: Vec<Page>,
    pub pdf_bytes: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,
    pub compile_time_us: u64,
}
```

### FNV-1a Hashing

FNV-1a was chosen over SipHash for its speed on short inputs. For a typical 5KB .tex document, the hash computation takes ~200ns — negligible compared to the ~4.5ms compile time.

```rust
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
```

### Cache Hit Path

On a cache hit, the compile response is returned in under 1ms (the time to hash the input, look up the entry, and serialize the cached response to JSON).

## Protocol Layer (`src/protocol.rs`)

### JSON Lines Protocol (v1)

Communication uses newline-delimited JSON on stdin/stdout.

**Request format:**
```json
{
  "protocol_version": 1,
  "type": "compile",
  "id": "unique-request-id",
  "payload": {
    "documents": [
      {"path": "main.tex", "text": "\\section{Hello}World"}
    ],
    "options": {
      "project_id": "my-project",
      "output_format": "pdf"
    }
  }
}
```

**Response format:**
```json
{
  "id": "unique-request-id",
  "status": "ok",
  "cache_hit": false,
  "compile_time_us": 4500,
  "pages": [{
    "items": [{
      "text": "Hello",
      "x_pt": 72.0,
      "y_pt": 769.89,
      "font_size_pt": 17,
      "font_style": "bold",
      "source": {"start_byte": 9, "end_byte": 14}
    }]
  }],
  "pdf_path": "/tmp/flashtex/demo/rev0.pdf",
  "diagnostics": []
}
```

### Error Response

```json
{
  "id": "unique-request-id",
  "status": "error",
  "diagnostics": [{
    "severity": "error",
    "message": "Unexpected token",
    "span": {"start_byte": 42, "end_byte": 43, "file": "main.tex"},
    "suggestion": "Did you mean \\textbf?"
  }],
  "pages": [],
  "pdf_path": null
}
```

## Security Model (`src/protocol.rs`)

### Input Validation

Before any .tex content reaches the compiler, the protocol layer validates:

1. **Path traversal**: Reject any document path containing `../`, `/`, or starting with a dot
2. **Absolute paths**: Reject paths starting with `/` or drive letters
3. **Escape sequences**: Reject null bytes and control characters
4. **Size limits**: Reject documents larger than 10MB
5. **Document count**: Reject requests with more than 100 documents

```rust
fn validate_path(path: &str) -> Result<(), SecurityError> {
    if path.contains("..") {
        return Err(SecurityError::PathTraversal(path.to_string()));
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(SecurityError::AbsolutePath(path.to_string()));
    }
    if path.bytes().any(|b| b < 0x20 && b != b'\t' && b != b'\n') {
        return Err(SecurityError::ControlCharacter(path.to_string()));
    }
    Ok(())
}
```

## Diagnostics (`src/diagnostics.rs`)

### Diagnostic Structure

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub suggestion: Option<String>,
    pub related: Vec<RelatedInfo>,
}

pub enum Severity {
    Error,
    Warning,
    Info,
}

pub struct RelatedInfo {
    pub message: String,
    pub span: Span,
}
```

### Error Catalog

| Code | Message | Recovery |
|------|---------|----------|
| E001 | Unknown command | Skip command, continue |
| E002 | Unmatched brace | Insert matching brace |
| E003 | Unexpected EOF in group | Close all open groups |
| E004 | Invalid environment nesting | Close mismatched environments |
| E005 | Path traversal attempt | Reject entire request |

## Performance Characteristics

### Benchmark Results (M1 MacBook Pro)

| Document | Size | FlashTeX | Overleaf (pdfLaTeX) | Tectonic |
|----------|------|----------|-------------------|----------|
| hello.tex | 38B | 4.5ms | 349ms | 685ms |
| article.tex | 2.1KB | 8.2ms | 412ms | 723ms |
| report.tex | 8.7KB | 16ms | 890ms | 1,240ms |
| thesis-ch1.tex | 24KB | 47ms | 2,100ms | 3,400ms |

### Memory Usage

- Compiler binary: 711 KiB
- Runtime RSS for hello.tex: ~3.2 MB
- Runtime RSS for 24KB document: ~5.8 MB
- Cache overhead per entry: ~200 bytes metadata + PDF bytes

### Compile Time Breakdown (hello.tex, 4.5ms total)

| Stage | Time | Percentage |
|-------|------|-----------|
| Lexer | 0.1ms | 2% |
| Parser | 0.2ms | 4% |
| Layout | 0.8ms | 18% |
| PDF write | 1.2ms | 27% |
| I/O + JSON | 2.2ms | 49% |

Note: I/O dominates because the JSON serialization of the response (including all page items with source spans) is larger than the actual compilation work for trivial documents.

## Build System

```toml
# Cargo.toml
[package]
name = "flashtex-compiler"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"

[dependencies]
pdf-writer = "0.15"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
```

The release binary is built with thin LTO and single codegen unit for maximum optimization. The `strip = true` flag removes debug symbols, keeping the binary at 711 KiB.

## Future Architecture Notes

### Math Mode (planned)

Math rendering will add a new layout pass that handles:
- Symbol lookup from a built-in Unicode math table
- Superscript/subscript positioning
- Fraction layout (numerator/denominator with dividing rule)
- Extensible delimiters

### Font Embedding (planned)

Moving beyond base-14 fonts requires:
- TrueType/OpenType parser (or a minimal subset parser)
- Glyph subsetting for the characters actually used
- CMap table generation for Unicode mapping
- Font metrics extraction replacing the hardcoded Helvetica table
