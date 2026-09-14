---
name: Bug Report
about: Report a compiler bug or incorrect output
title: "[BUG] "
labels: bug
assignees: ''
---

## Describe the bug

A clear description of what the bug is.

## Input (.tex source)

```latex
% Paste the minimal .tex input that reproduces the issue
\section{Example}
Your problematic LaTeX here.
```

## Expected output

Describe what the compiler should produce (or paste expected JSON response).

## Actual output

Paste the actual compiler response:

```json
{
  "id": "...",
  "status": "...",
  "diagnostics": [...]
}
```

## Environment

- **FlashTeX version**: (run `./flashtex-compiler --version`)
- **Rust version**: (run `rustc --version`)
- **OS**: (e.g., macOS 14, Ubuntu 24.04, Windows 11)
- **Architecture**: (e.g., x86_64, aarch64)

## Additional context

Any other context — screenshots of incorrect PDF output, comparison with pdfLaTeX output, etc.
