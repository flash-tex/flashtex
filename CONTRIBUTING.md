# Contributing to FlashTeX

Thanks for your interest in contributing to FlashTeX! This document outlines the process for contributing to the project.

## Getting Started

1. **Fork the repo** — Click the "Fork" button on GitHub
2. **Clone your fork** — `git clone https://github.com/YOUR_USERNAME/flashtex.git`
3. **Create a branch** — `git checkout -b feature/your-feature-name`
4. **Build the project** — `cargo build`
5. **Run tests** — `cargo test`

## Development Setup

FlashTeX requires:
- Rust 2021 edition (stable toolchain)
- No external TeX distribution needed

```bash
# Clone and build
git clone https://github.com/flash-tex/flashtex.git
cd flashtex
cargo build --release

# Run the test suite
cargo test

# Run a single compile
echo '{"protocol_version":1,"type":"compile","id":"r1","payload":{"documents":[{"path":"main.tex","text":"\\section{Hello}World"}]}}' | ./target/release/flashtex-compiler
```

## Project Structure

| Module | Purpose |
|--------|---------|
| `lexer` | Tokenizes .tex input into a stream of LaTeX tokens |
| `parser` | Builds an AST from the token stream |
| `layout` | Computes page geometry, line breaks, and glyph positions |
| `pdf` | Writes the final PDF bytes (Helvetica base-14 fonts) |
| `protocol` | JSON Lines stdin/stdout protocol handler |
| `cache` | 64-entry LRU cache keyed by FNV-1a hash |
| `diagnostics` | Source-span error reporting with recovery |
| `json` | Custom zero-dependency JSON parser |
| `lib` | Public API surface |

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Every public function needs a doc comment
- Add tests for new features in the corresponding module's `tests` submodule

## Pull Request Process

1. Update tests to cover your changes
2. Make sure `cargo test` passes with zero failures
3. Make sure `cargo clippy` produces zero warnings
4. Open a PR against `main` with a clear description of what changed and why
5. One approving review required before merge

## Reporting Issues

Open an issue on GitHub with:
- A minimal .tex document that reproduces the problem
- Expected vs actual compiler output
- Your Rust toolchain version (`rustc --version`)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
